//! Contains helpers for Gotham applications to use during testing.
//!
//! [`TestServer::new(_)`][TestServer::new] is the most useful entry point.
//!
//! [TestServer::new]: struct.TestServer.html#method.new

use std::{cell, io, net, time};
// TODO: Cross platform
use std::os::unix::net::UnixStream;
use std::os::unix::io::AsRawFd;
use hyper::{self, client, server};
use futures::{future, Future, Async, Stream};
use tokio_core::reactor;
use tokio_io::{AsyncRead, AsyncWrite};
use mio;

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
/// # use hyper::{server, StatusCode};
/// # use futures::{future, Future};
/// #
/// # struct MyService;
/// #
/// # impl server::Service for MyService {
/// #     type Request = server::Request;
/// #     type Response = server::Response;
/// #     type Error = hyper::Error;
/// #     type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;
/// #
/// #     fn call(&self, _req: Self::Request) -> Self::Future {
/// #         future::ok(server::Response::new().with_status(StatusCode::Accepted)).boxed()
/// #     }
/// # }
/// #
/// # impl MyService {
/// #     fn new() -> MyService {
/// #         MyService
/// #     }
/// # }
/// #
/// # fn main() {
/// use gotham::test::TestServer;
///
/// let mut test_server = TestServer::new(|| Ok(MyService::new())).unwrap();
///
/// let uri = "http://localhost/".parse().unwrap();
/// let client_addr = "127.0.0.1:15100".parse().unwrap();
///
/// let future = test_server.client(client_addr).unwrap().get(uri);
/// let response = test_server.run_request(future).unwrap();
///
/// assert_eq!(response.status(), StatusCode::Accepted);
/// # }
/// ```
pub struct TestServer<S> {
    core: reactor::Core,
    http: server::Http,
    timeout: u64,
    new_service: S,
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

impl<S> TestServer<S>
    where S: server::NewService<Request = server::Request,
                                Response = server::Response,
                                Error = hyper::Error>,
          S::Instance: 'static
{
    /// Creates a `TestServer` instance for the service spawned by `new_service`. This server has
    /// the same guarantee given by `hyper::server::Http::bind`, that a new service will be spawned
    /// for each connection.
    pub fn new(new_service: S) -> Result<TestServer<S>, io::Error> {
        reactor::Core::new().map(|core| {
            TestServer {
                core: core,
                http: server::Http::new(),
                timeout: 10,
                new_service: new_service,
            }
        })
    }

    /// Sets the request timeout to `t` seconds and returns a new `TestServer`. The default timeout
    /// value is 10 seconds.
    pub fn timeout(self, t: u64) -> TestServer<S> {
        TestServer { timeout: t, ..self }
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see `client_addr` as the source address for the connection. The
    /// `client_addr` can be any value, and need not be contactable.
    pub fn client(&self, client_addr: net::SocketAddr) -> io::Result<client::Client<TestConnect>> {
        let handle = self.core.handle();

        let (cs, ss) = AsyncUnixStream::pair()?;
        let cs = reactor::PollEvented::new(cs, &handle)?;
        let ss = reactor::PollEvented::new(ss, &handle)?;

        let service = self.new_service.new_service()?;
        self.http.bind_connection(&handle, ss, client_addr, service);
        Ok(client::Client::configure()
               .connector(TestConnect { stream: cell::RefCell::new(Some(cs)) })
               .build(&self.core.handle()))
    }

    /// Runs the event loop until the response future is completed.
    ///
    /// If the future came from a different instance of `TestServer`, the event loop will run until
    /// the timeout is triggered.
    // TODO: Ensure this is impossible to trigger in the more ergonomic client interface to
    // `TestServer`, when such a thing is written.
    pub fn run_request<F>(&mut self, f: F) -> Result<F::Item, TestRequestError>
        where F: Future<Error = hyper::Error>
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
    stream: cell::RefCell<Option<reactor::PollEvented<AsyncUnixStream>>>,
}

impl client::Service for TestConnect {
    type Request = hyper::Uri;
    type Error = io::Error;
    type Response = reactor::PollEvented<AsyncUnixStream>;
    type Future = future::FutureResult<Self::Response, Self::Error>;

    fn call(&self, _req: Self::Request) -> Self::Future {
        match self.stream.try_borrow_mut().map(|ref mut o| o.take()) {
            Ok(Some(stream)) => future::ok(stream),
            Ok(None) => future::err(io::Error::new(io::ErrorKind::Other, "stream already taken")),
            Err(_) => {
                future::err(io::Error::new(io::ErrorKind::Other, "stream.try_borrow_mut() failed"))
            }
        }
    }
}

/// Wrapping type for an asynchronous `std::os::unix::net::UnixStream`. This type should never be
/// used directly.
pub struct AsyncUnixStream {
    stream: UnixStream,
}

impl AsyncUnixStream {
    fn new(stream: UnixStream) -> Result<AsyncUnixStream, io::Error> {
        stream.set_nonblocking(true)?;
        Ok(AsyncUnixStream { stream: stream })
    }

    fn pair() -> Result<(AsyncUnixStream, AsyncUnixStream), io::Error> {
        let (cs, ss) = UnixStream::pair()?;
        let cs = AsyncUnixStream::new(cs)?;
        let ss = AsyncUnixStream::new(ss)?;
        Ok((cs, ss))
    }
}

fn io_error_to_async_io_error<T>(r: Result<T, io::Error>) -> Result<T, io::Error> {
    // Here, we trap the EAGAIN (35) error that is reported by nonblocking unix sockets, and return
    // the `WouldBlock` that tokio/hyper expect to work with.
    //
    // From: https://tokio.rs/docs/going-deeper/core-low-level/ (as at 2017-03-30)
    //
    // All I/O with tokio-core consistently adheres to two properties:
    //
    // * Operations are non-blocking. If an operation would otherwise block an error of the
    //   WouldBlock error kind is returned.
    // * When a WouldBlock error is returned, the current future task is scheduled to receive a
    //   notification when the I/O object would otherwise be ready.
    r.map_err(|e| match e.raw_os_error() {
                  Some(35) => io::Error::new(io::ErrorKind::WouldBlock, "test socket would block"),
                  _ => e,
              })
}

impl io::Read for AsyncUnixStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let result = self.stream.read(buf);
        io_error_to_async_io_error(result)
    }
}

impl AsyncRead for AsyncUnixStream {}

impl io::Write for AsyncUnixStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        let result = self.stream.write(buf);
        io_error_to_async_io_error(result)
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        let result = self.stream.flush();
        io_error_to_async_io_error(result)
    }
}

impl AsyncWrite for AsyncUnixStream {
    fn shutdown(&mut self) -> Result<Async<()>, io::Error> {
        self.stream.shutdown(net::Shutdown::Both).map(|_| Async::Ready(()))
    }
}

impl mio::event::Evented for AsyncUnixStream {
    fn register(&self,
                poll: &mio::Poll,
                token: mio::Token,
                ready: mio::Ready,
                poll_opt: mio::PollOpt)
                -> Result<(), io::Error> {
        mio::unix::EventedFd(&self.stream.as_raw_fd()).register(poll, token, ready, poll_opt)
    }

    fn reregister(&self,
                  poll: &mio::Poll,
                  token: mio::Token,
                  ready: mio::Ready,
                  poll_opt: mio::PollOpt)
                  -> Result<(), io::Error> {
        mio::unix::EventedFd(&self.stream.as_raw_fd()).reregister(poll, token, ready, poll_opt)
    }

    fn deregister(&self, poll: &mio::Poll) -> Result<(), io::Error> {
        mio::unix::EventedFd(&self.stream.as_raw_fd()).deregister(poll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use hyper::StatusCode;

    #[derive(Clone)]
    struct TestService {
        response: String,
    }

    impl server::Service for TestService {
        type Request = server::Request;
        type Response = server::Response;
        type Error = hyper::Error;
        type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

        fn call(&self, req: Self::Request) -> Self::Future {
            match req.path() {
                "/" => {
                    let response = server::Response::new()
                        .with_status(StatusCode::Ok)
                        .with_body(self.response.clone());

                    future::ok(response).boxed()
                }
                "/timeout" => future::empty().boxed(),
                "/myaddr" => {
                    let response = server::Response::new()
                        .with_status(StatusCode::Ok)
                        .with_body(format!("{}", req.remote_addr().unwrap()));

                    future::ok(response).boxed()
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn serves_requests() {
        let ticks = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let new_service = move || Ok(TestService { response: format!("time: {}", ticks) });
        let uri = "http://localhost/".parse().unwrap();

        let mut test_server = TestServer::new(new_service).unwrap();
        let response = test_server.client("127.0.0.1:0".parse().unwrap()).unwrap().get(uri);
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
        let response = test_server.client("127.0.0.1:0".parse().unwrap()).unwrap().get(uri);

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
        let ticks = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let new_service = move || Ok(TestService { response: format!("time: {}", ticks) });
        let client_addr = "9.8.7.6:58901".parse().unwrap();
        let uri = "http://localhost/myaddr".parse().unwrap();

        let mut test_server = TestServer::new(new_service).unwrap();
        let response = test_server.client(client_addr).unwrap().get(uri);
        let response = test_server.run_request(response).unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        let buf = test_server.read_body(response).unwrap();
        let received_addr: net::SocketAddr = String::from_utf8(buf).unwrap().parse().unwrap();
        assert_eq!(received_addr, client_addr);
    }
}
