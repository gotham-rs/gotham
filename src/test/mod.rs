//! Contains helpers for Gotham applications to use during testing.
//!
//! `TestServer::new(_)` is the most useful entry point.

use std::{cell, io, net, time};
use std::net::{TcpListener, TcpStream, SocketAddr, IpAddr};
use hyper::{self, client, server};
use hyper::server::NewService;
use futures::{future, Future, Stream};
use tokio_core::reactor;
use mio;

use handler::{NewHandler, NewHandlerService};
use router::Router;

/// The `TestServer` type, which is used as a harness when writing test cases for Hyper services
/// (which Gotham's `Router` is). An instance of `TestServer` is run single-threaded and
/// asynchronous, and only accessible by a client returned from the `TestServer`.
///
/// # Examples
///
/// ```rust
/// # extern crate hyper;
/// # extern crate futures;
/// # extern crate gotham;
/// #
/// # use gotham::state::State;
/// # use hyper::{Response, StatusCode};
/// #
/// # fn my_handler(state: State) -> (State, Response) {
/// #   (state, Response::new().with_status(StatusCode::Accepted))
/// # }
/// #
/// # fn main() {
/// use gotham::test::TestServer;
///
/// let mut test_server = TestServer::new(|| Ok(my_handler)).unwrap();
///
/// let uri = "http://localhost/".parse().unwrap();
///
/// let future = test_server.client().get(uri);
/// let response = test_server.run_request(future).unwrap();
///
/// assert_eq!(response.status(), StatusCode::Accepted);
/// # }
/// ```
pub struct TestServer<NH = Router>
where
    NH: NewHandler + 'static,
{
    core: reactor::Core,
    http: server::Http,
    timeout: u64,
    new_service: NewHandlerService<NH>,
}

/// The `TestRequestError` type represents all error states that can result from evaluating a
/// response future. See `TestServer::run_request` for usage.
#[derive(Debug)]
pub enum TestRequestError {
    /// The response was not received before the timeout duration elapsed
    TimedOut,
    /// A `std::io::Error` occurred before a response was received
    IoError(io::Error),
    /// A `hyper::Error` occurred before a response was received
    HyperError(hyper::Error),
}

impl<NH> TestServer<NH>
where
    NH: NewHandler + 'static,
{
    /// Creates a `TestServer` instance for the service spawned by `new_service`. This server has
    /// the same guarantee given by `hyper::server::Http::bind`, that a new service will be spawned
    /// for each connection.
    pub fn new(new_handler: NH) -> Result<TestServer<NH>, io::Error> {
        reactor::Core::new().map(|core| {
            TestServer {
                core: core,
                http: server::Http::new(),
                timeout: 10,
                new_service: NewHandlerService::new(new_handler),
            }
        })
    }

    /// Sets the request timeout to `t` seconds and returns a new `TestServer`. The default timeout
    /// value is 10 seconds.
    pub fn timeout(self, t: u64) -> TestServer<NH> {
        TestServer { timeout: t, ..self }
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see a default value as the source address for the connection.
    pub fn client(&self) -> client::Client<TestConnect> {
        self.client_with_address(SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 10000))
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see `client_addr` as the source address for the connection. The
    /// `client_addr` can be any value, and need not be contactable.
    pub fn client_with_address(&self, client_addr: net::SocketAddr) -> client::Client<TestConnect> {
        self.try_client_with_address(client_addr).expect(
            "TestServer: unable to spawn client",
        )
    }

    fn try_client_with_address(
        &self,
        client_addr: net::SocketAddr,
    ) -> io::Result<client::Client<TestConnect>> {
        let handle = self.core.handle();

        let (cs, ss) = {
            // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
            // it and then immediately discard the listener.
            let listener = TcpListener::bind("localhost:0")?;
            let listener_addr = listener.local_addr()?;
            let client = TcpStream::connect(listener_addr)?;
            let (server, _client_addr) = listener.accept()?;
            (client, server)
        };

        let cs = mio::net::TcpStream::from_stream(cs)?;
        let cs = reactor::PollEvented::new(cs, &handle)?;

        let ss = mio::net::TcpStream::from_stream(ss)?;
        let ss = reactor::PollEvented::new(ss, &handle)?;

        let service = self.new_service.new_service()?;
        self.http.bind_connection(&handle, ss, client_addr, service);
        Ok(
            client::Client::configure()
                .connector(TestConnect { stream: cell::RefCell::new(Some(cs)) })
                .build(&self.core.handle()),
        )
    }

    /// Runs the event loop until the response future is completed.
    ///
    /// If the future came from a different instance of `TestServer`, the event loop will run until
    /// the timeout is triggered.
    // TODO: Ensure this is impossible to trigger in the more ergonomic client interface to
    // `TestServer`, when such a thing is written.
    pub fn run_request<F>(&mut self, f: F) -> Result<F::Item, TestRequestError>
    where
        F: Future<Error = hyper::Error>,
    {
        let timeout_duration = time::Duration::from_secs(self.timeout);
        let timeout = reactor::Timeout::new(timeout_duration, &self.core.handle())
            .map_err(|e| TestRequestError::IoError(e))?;

        let run_result = self.core.run(f.select2(timeout));
        match run_result {
            Ok(future::Either::A((item, _))) => Ok(item),
            Ok(future::Either::B(_)) => Err(TestRequestError::TimedOut),
            Err(future::Either::A((e, _))) => Err(TestRequestError::HyperError(e)),
            Err(future::Either::B((e, _))) => Err(TestRequestError::IoError(e)),
        }
    }

    /// Runs the event loop until the response body has been fully read. An `Ok(_)` response holds
    /// a buffer containing all bytes of the response body.
    pub fn read_body(&mut self, response: client::Response) -> hyper::Result<Vec<u8>> {
        let mut buf = Vec::new();

        let r = {
            let f: hyper::Body = response.body();
            let f = f.for_each(|chunk| future::ok(buf.extend(chunk.into_iter())));
            self.core.run(f)
        };

        r.map(|_| buf)
    }
}

/// `TestConnect` represents the connection between a test client and the `TestServer` instance
/// that created it. This type should never be used directly.
pub struct TestConnect {
    stream: cell::RefCell<Option<reactor::PollEvented<mio::net::TcpStream>>>,
}

impl client::Service for TestConnect {
    type Request = hyper::Uri;
    type Error = io::Error;
    type Response = reactor::PollEvented<mio::net::TcpStream>;
    type Future = future::FutureResult<Self::Response, Self::Error>;

    fn call(&self, _req: Self::Request) -> Self::Future {
        match self.stream.try_borrow_mut().map(|ref mut o| o.take()) {
            Ok(Some(stream)) => future::ok(stream),
            Ok(None) => future::err(io::Error::new(io::ErrorKind::Other, "stream already taken")),
            Err(_) => {
                future::err(io::Error::new(
                    io::ErrorKind::Other,
                    "stream.try_borrow_mut() failed",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use hyper::{StatusCode, Uri};
    use handler::{Handler, NewHandler, HandlerFuture};
    use state::{State, FromState, client_addr};

    #[derive(Clone)]
    struct TestService {
        response: String,
    }

    impl Handler for TestService {
        fn handle(self, state: State) -> Box<HandlerFuture> {
            let path = Uri::borrow_from(&state).path().to_owned();
            match path.as_str() {
                "/" => {
                    let response = server::Response::new()
                        .with_status(StatusCode::Ok)
                        .with_body(self.response.clone());

                    Box::new(future::ok((state, response)))
                }
                "/timeout" => Box::new(future::empty()),
                "/myaddr" => {
                    let response = server::Response::new()
                        .with_status(StatusCode::Ok)
                        .with_body(format!("{}", client_addr(&state).unwrap()));

                    Box::new(future::ok((state, response)))
                }
                _ => unreachable!(),
            }
        }
    }

    impl NewHandler for TestService {
        type Instance = Self;

        fn new_handler(&self) -> io::Result<Self> {
            Ok(self.clone())
        }
    }

    #[test]
    fn serves_requests() {
        let ticks = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_service = move || Ok(TestService { response: format!("time: {}", ticks) });
        let uri = "http://localhost/".parse().unwrap();

        let mut test_server = TestServer::new(new_service).unwrap();
        let response = test_server.client().get(uri);
        let response = test_server.run_request(response).unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        let buf = test_server.read_body(response).unwrap();
        assert_eq!(buf.as_slice(), format!("time: {}", ticks).as_bytes());
    }

    #[test]
    fn times_out() {
        let new_service = || Ok(TestService { response: "".to_owned() });
        let mut test_server = TestServer::new(new_service).unwrap().timeout(1);
        let uri = "http://localhost/timeout".parse().unwrap();
        let response = test_server.client().get(uri);

        match test_server.run_request(response) {
            Err(TestRequestError::TimedOut) => (),
            e @ Err(_) => {
                e.unwrap();
            }
            Ok(_) => panic!("expected timeout, but was Ok(_)"),
        }
    }

    #[test]
    fn sets_client_addr() {
        let ticks = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_service = move || Ok(TestService { response: format!("time: {}", ticks) });
        let client_addr = "9.8.7.6:58901".parse().unwrap();
        let uri = "http://localhost/myaddr".parse().unwrap();

        let mut test_server = TestServer::new(new_service).unwrap();
        let response = test_server.client_with_address(client_addr).get(uri);
        let response = test_server.run_request(response).unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        let buf = test_server.read_body(response).unwrap();
        let received_addr: net::SocketAddr = String::from_utf8(buf).unwrap().parse().unwrap();
        assert_eq!(received_addr, client_addr);
    }
}
