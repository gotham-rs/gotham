//! Contains helpers for Gotham applications to use during testing.
//!
//! See the `TestServer` type for example usage.

use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{io, net};

use failure;

use futures::{future, sync::oneshot, Future, Stream};
use futures_timer::Delay;
use hyper::client::Client;
use hyper::header::CONTENT_TYPE;
use hyper::server::conn::Http;
use hyper::service;
use hyper::{self, Body, Method, Request, Response, Uri};
use mime;
use mio;
use tokio::reactor::PollEvented2;
use tokio::runtime::Runtime;
use tokio_core::reactor::Core;

use handler::{IntoHandlerFuture, NewHandler};
use router::Router;
use service::GothamService;

use state;

mod request;

pub use self::request::RequestBuilder;

/// The `TestServer` type, which is used as a harness when writing test cases for Hyper services
/// (which Gotham's `Router` is). An instance of `TestServer` is run asynchronously within the
/// current thread, and is only accessible by a client returned from the `TestServer`.
///
/// # Examples
///
/// ```rust
/// # extern crate hyper;
/// # extern crate gotham;
/// #
/// # use gotham::state::State;
/// # use hyper::{Response, StatusCode};
/// #
/// # fn my_handler(state: State) -> (State, Response) {
/// #   (state, Response::new().with_status(StatusCode::ACCEPTED))
/// # }
/// #
/// # fn main() {
/// use gotham::test::TestServer;
///
/// let test_server = TestServer::new(|| Ok(my_handler)).unwrap();
///
/// let response = test_server.client().get("http://localhost/").perform().unwrap();
/// assert_eq!(response.status(), StatusCode::ACCEPTED);
/// # }
/// ```
pub struct TestServer<NH = Router>
where
    NH: NewHandler + Send + 'static,
{
    data: Rc<TestServerData<NH>>,
}

struct TestServerData<NH = Router>
where
    NH: NewHandler + Send + 'static,
{
    http: Http,
    timeout: u64,
    runtime: RwLock<Runtime>,
    gotham_service: GothamService<NH>,
}

/// The `TestRequestError` type represents all error states that can result from evaluating a
/// response future. See `TestServer::run_request` for usage.
#[derive(Debug)]
pub enum TestRequestError {
    /// The response was not received before the timeout duration elapsed.
    TimedOut,
    /// A `std::io::Error` occurred before a response was received.
    IoError(io::Error),
    /// A `hyper::Error` occurred before a response was received.
    HyperError(hyper::Error),
}

impl<NH> Clone for TestServer<NH>
where
    NH: NewHandler + Send + 'static,
{
    fn clone(&self) -> TestServer<NH> {
        TestServer {
            data: self.data.clone(),
        }
    }
}

impl<NH> TestServer<NH>
where
    NH: NewHandler + Send + 'static,
{
    /// Creates a `TestServer` instance for the `Handler` spawned by `new_handler`. This server has
    /// the same guarantee given by `hyper::server::Http::bind`, that a new service will be spawned
    /// for each connection.
    ///
    /// Timeout will be set to 10 seconds.
    pub fn new(new_handler: NH) -> Result<TestServer<NH>, io::Error> {
        TestServer::with_timeout(new_handler, 10)
    }

    /// Sets the request timeout to `timeout` seconds and returns a new `TestServer`.
    pub fn with_timeout(new_handler: NH, timeout: u64) -> Result<TestServer<NH>, io::Error> {
        let data = TestServerData {
            http: Http::new(),
            timeout,
            runtime: RwLock::new(Runtime::new().unwrap()),
            gotham_service: GothamService::new(Arc::new(new_handler)),
        };

        Ok(TestServer {
            data: Rc::new(data),
        })
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see a default socket address of `127.0.0.1:10000` as the source address for
    /// the connection.
    pub fn client(&self) -> TestClient<NH> {
        self.client_with_address(SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 10000))
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see `client_addr` as the source address for the connection. The
    /// `client_addr` can be any valid `SocketAddr`, and need not be contactable.
    pub fn client_with_address(&self, client_addr: net::SocketAddr) -> TestClient<NH> {
        self.try_client_with_address(client_addr)
            .expect("TestServer: unable to spawn client")
    }

    fn try_client_with_address(&self, client_addr: net::SocketAddr) -> io::Result<TestClient<NH>> {
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
        let cs = PollEvented2::new(cs);

        let ss = mio::net::TcpStream::from_stream(ss)?;
        let ss = PollEvented2::new(ss);

        let service = self.data.gotham_service.connect(client_addr);
        let f = self.data
            .http
            .serve_connection(ss, service)
            .map(|_| ())
            .map_err(|_| ());

        {
            self.data.runtime.read().unwrap().spawn(f);
        }

        let client = Core::new()
            .map(|core| {
                Client::builder().build(TestConnect {
                    stream: RefCell::new(Some(cs)),
                })
            })
            .unwrap();

        Ok(TestClient {
            client,
            test_server: self.clone(),
        })
    }

    /// Runs the event loop until the response future is completed.
    ///
    /// If the future came from a different instance of `TestServer`, the event loop will run until
    /// the timeout is triggered.
    fn run_request<F>(&self, f: F) -> Result<F::Item, TestRequestError>
    where
        F: Future<Error = hyper::Error> + Send + 'static,
        F::Item: Send,
    {
        let timeout_duration = Duration::from_secs(self.data.timeout);
        let timeout = Delay::new(timeout_duration);

        match self.run_future(f.select2(timeout)) {
            Ok(future::Either::A((item, _))) => Ok(item),
            Ok(future::Either::B(_)) => Err(TestRequestError::TimedOut),
            Err(future::Either::A((e, _))) => Err(TestRequestError::HyperError(e)),
            Err(future::Either::B((e, _))) => Err(TestRequestError::IoError(e)),
        }
    }
    /// Runs a future inside of the internal runtime.
    ///
    /// This blocks on the result of the future and behaves like a synchronous
    /// polling call of the future, even if it might be on another thread.
    fn run_future<F, R, E>(&mut self, future: F) -> Result<R, E>
    where
        F: Send + 'static + Future<Item = R, Error = E>,
        R: Send + 'static,
        E: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        {
            self.data
                .runtime
                .read()
                .unwrap()
                .spawn(future.then(move |r| tx.send(r).map_err(|_| unreachable!())));
        }
        rx.wait().unwrap()
    }
}

impl<NH> BodyReader for TestServer<NH>
where
    NH: NewHandler + Send + 'static,
{
    fn read_body(&self, response: Response<Body>) -> Result<Vec<u8>, failure::Error> {
        let mut buf = Vec::new();

        let r = {
            let f = response.body();
            let f = f.for_each(|chunk| future::ok(buf.extend(chunk.into_iter())));

            self.run_future(f)
        };

        Ok(r.map(|_| buf)?)
    }
}

/// Client interface for issuing requests to a `TestServer`.
pub struct TestClient<NH>
where
    NH: NewHandler + Send + 'static,
{
    client: Client<TestConnect>,
    test_server: TestServer<NH>,
}

impl<NH, H, IHF> TestClient<NH>
where
    NH: NewHandler + Send + 'static + Fn() -> io::Result<H>,
    H: Send + FnOnce(state::State) -> IHF,
    IHF: IntoHandlerFuture + Sized,
{
    /// Parse the URI and begin constructing a HEAD request using this `TestClient`.
    pub fn head(self, uri: &str) -> RequestBuilder<NH> {
        self.build_request(Method::HEAD, uri)
    }

    /// Begin constructing a HEAD request using this `TestClient`.
    pub fn head_uri(self, uri: Uri) -> RequestBuilder<NH> {
        self.build_request_uri(Method::HEAD, uri)
    }

    /// Parse the URI and begin constructing a GET request using this `TestClient`.
    pub fn get(self, uri: &str) -> RequestBuilder<NH> {
        self.build_request(Method::GET, uri)
    }

    /// Begin constructing a GET request using this `TestClient`.
    pub fn get_uri(self, uri: Uri) -> RequestBuilder<NH> {
        self.build_request_uri(Method::GET, uri)
    }

    /// Parse the URI and begin constructing a POST request using this `TestClient`.
    pub fn post<T>(self, uri: &str, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request(Method::POST, uri)
            .with_body(body)
            .with_header(CONTENT_TYPE, content_type.to_string().parse().unwrap())
    }

    /// Begin constructing a POST request using this `TestClient`.
    pub fn post_uri<T, QB>(self, uri: Uri, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request_uri(Method::POST, uri)
            .with_body(body)
            .with_header(CONTENT_TYPE, content_type.to_string().parse().unwrap())
    }

    /// Parse the URI and begin constructing a PUT request using this `TestClient`.
    pub fn put<T>(self, uri: &str, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request(Method::PUT, uri)
            .with_body(body)
            .with_header(CONTENT_TYPE, content_type.to_string().parse().unwrap())
    }

    /// Begin constructing a PUT request using this `TestClient`.
    pub fn put_uri<T, QB>(self, uri: Uri, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request_uri(Method::PUT, uri)
            .with_body(body)
            .with_header(CONTENT_TYPE, content_type.to_string().parse().unwrap())
    }

    /// Parse the URI and begin constructing a PATCH request using this `TestClient`.
    pub fn patch<T, QB>(self, uri: &str, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request(Method::PATCH, uri)
            .with_body(body)
            .with_header(CONTENT_TYPE, content_type.to_string().parse().unwrap())
    }

    /// Begin constructing a PATCH request using this `TestClient`.
    pub fn patch_uri<T, QB>(self, uri: Uri, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request_uri(Method::PATCH, uri)
            .with_body(body)
            .with_header(CONTENT_TYPE, content_type.to_string().parse().unwrap())
    }

    /// Parse the URI and begin constructing a DELETE request using this `TestClient`.
    pub fn delete(self, uri: &str) -> RequestBuilder<NH> {
        self.build_request(Method::DELETE, uri)
    }

    /// Begin constructing a DELETE request using this `TestClient`.
    pub fn delete_uri(self, uri: Uri) -> RequestBuilder<NH> {
        self.build_request_uri(Method::DELETE, uri)
    }

    /// Parse the URI and begin constructing a request with the given HTTP method.
    pub fn build_request(self, method: Method, uri: &str) -> RequestBuilder<NH> {
        RequestBuilder::new(self, method, uri.parse().unwrap())
    }

    /// Begin constructing a request with the given HTTP method and Uri.
    pub fn build_request_uri(self, method: Method, uri: Uri) -> RequestBuilder<NH> {
        RequestBuilder::new(self, method, uri)
    }

    /// Send a constructed request using this `TestClient`, and await the response.
    pub fn perform<QB>(self, req: Request<QB>) -> Result<TestResponse, TestRequestError> {
        self.test_server
            .run_request(self.client.request(req))
            .map(|response| TestResponse {
                response,
                reader: Box::new(self.test_server.clone()),
            })
    }
}

trait BodyReader {
    /// Runs the underlying event loop until the response body has been fully read. An `Ok(_)`
    /// response holds a buffer containing all bytes of the response body.
    fn read_body(&self, response: Response<Body>) -> Result<Vec<u8>, failure::Error>;
}

/// Wrapping struct for the `Response` returned by a `TestClient`. Provides access to the
/// `Response` value via the `Deref` and `DerefMut` traits, and also provides a function for
/// awaiting a completed response body.
///
/// # Examples
///
/// ```rust
/// # extern crate hyper;
/// # extern crate gotham;
/// # extern crate mime;
/// #
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::create_response;
/// # use hyper::{Response, StatusCode};
/// #
/// # fn my_handler(state: State) -> (State, Response) {
/// #   let body = "This is the body content.".to_string().into_bytes();;
/// #   let response = create_response(&state,
/// #                                  StatusCode::OK,
/// #                                  Some((body, mime::TEXT_PLAIN)));
/// #
/// #   (state, response)
/// # }
/// #
/// # fn main() {
/// use gotham::test::TestServer;
///
/// let test_server = TestServer::new(|| Ok(my_handler)).unwrap();
///
/// let response = test_server.client().get("http://localhost/").perform().unwrap();
/// assert_eq!(response.status(), StatusCode::OK);
/// let body = response.read_body().unwrap();
/// assert_eq!(&body[..], b"This is the body content.");
/// # }
/// ```
pub struct TestResponse {
    response: Response<Body>,
    reader: Box<BodyReader>,
}

impl Deref for TestResponse {
    type Target = Response<Body>;

    fn deref(&self) -> &Response<Body> {
        &self.response
    }
}

impl DerefMut for TestResponse {
    fn deref_mut(&mut self) -> &mut Response<Body> {
        &mut self.response
    }
}

impl TestResponse {
    /// Awaits the body of the underlying `Response`, and returns it. This will cause the event
    /// loop to execute until the `Response` body has been fully read into the `Vec<u8>`.
    pub fn read_body(self) -> Result<Vec<u8>, failure::Error> {
        self.reader.read_body(self.response)
    }

    /// Awaits the UTF-8 encoded body of the underlying `Response`, and returns the `String`. This
    /// will cause the event loop to execute until the `Response` body has been fully read and the
    /// `String` created.
    pub fn read_utf8_body(self) -> Result<String, failure::Error> {
        let buf = self.read_body()?;
        let s = String::from_utf8(buf)?;
        Ok(s)
    }
}

/// `TestConnect` represents the connection between a test client and the `TestServer` instance
/// that created it. This type should never be used directly.
struct TestConnect {
    stream: RefCell<Option<PollEvented2<mio::net::TcpStream>>>,
}

impl service::Service for TestConnect {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = io::Error;
    type Future = future::FutureResult<Response<Self::ResBody>, Self::Error>;

    fn call(&mut self, _req: hyper::Request<Self::ReqBody>) -> Self::Future {
        match self.stream.try_borrow_mut().map(|ref mut o| o.take()) {
            Ok(Some(stream)) => future::ok(stream),
            Ok(None) => future::err(io::Error::new(io::ErrorKind::Other, "stream already taken")),
            Err(_) => future::err(io::Error::new(
                io::ErrorKind::Other,
                "stream.try_borrow_mut() failed",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::{SystemTime, UNIX_EPOCH};

    use hyper::header::CONTENT_LENGTH;
    use hyper::{Body, Response, StatusCode, Uri};
    use mime;

    use handler::{Handler, HandlerFuture, IntoHandlerError, NewHandler};
    use helpers::http::response::create_response;
    use state::{client_addr, FromState, State};

    #[derive(Clone)]
    struct TestHandler {
        response: String,
    }

    impl Handler for TestHandler {
        fn handle(self, state: State) -> Box<HandlerFuture> {
            let path = Uri::borrow_from(&state).path().to_owned();
            match path.as_str() {
                "/" => {
                    let response = Response::builder()
                        .status(StatusCode::OK)
                        .body(self.response.clone().into())
                        .unwrap();

                    Box::new(future::ok((state, response)))
                }
                "/timeout" => Box::new(future::empty()),
                "/myaddr" => {
                    let response = Response::builder()
                        .status(StatusCode::OK)
                        .body(format!("{}", client_addr(&state).unwrap()).into())
                        .unwrap();

                    Box::new(future::ok((state, response)))
                }
                _ => unreachable!(),
            }
        }
    }

    impl NewHandler for TestHandler {
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
        let new_service = move || {
            Ok(TestHandler {
                response: format!("time: {}", ticks),
            })
        };

        let test_server = TestServer::new(new_service).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let buf = response.read_utf8_body().unwrap();
        assert_eq!(buf, format!("time: {}", ticks));
    }

    #[test]
    fn times_out() {
        let new_service = || {
            Ok(TestHandler {
                response: "".to_owned(),
            })
        };
        let test_server = TestServer::with_timeout(new_service, 1).unwrap();

        let res = test_server
            .client()
            .get("http://localhost/timeout")
            .perform();

        match res {
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
        let new_service = move || {
            Ok(TestHandler {
                response: format!("time: {}", ticks),
            })
        };
        let client_addr = "9.8.7.6:58901".parse().unwrap();

        let test_server = TestServer::new(new_service).unwrap();
        let response = test_server
            .client_with_address(client_addr)
            .get("http://localhost/myaddr")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let buf = response.read_body().unwrap();
        let received_addr: net::SocketAddr = String::from_utf8(buf).unwrap().parse().unwrap();
        assert_eq!(received_addr, client_addr);
    }

    #[test]
    fn async_echo() {
        fn handler<B>(mut state: State) -> Box<HandlerFuture> {
            let f = Body::take_from(&mut state)
                .concat2()
                .then(move |full_body| match full_body {
                    Ok(body) => {
                        debug!("test");
                        let resp_data = body.to_vec();
                        let res = create_response(
                            &state,
                            StatusCode::OK,
                            Some((resp_data, mime::TEXT_PLAIN)),
                        );
                        future::ok((state, res))
                    }

                    Err(e) => future::err((state, e.into_handler_error())),
                });

            Box::new(f)
        }

        let server = TestServer::new(|| Ok(handler)).unwrap();

        let client = server.client();
        let data = "This text should get reflected back to us. Even this fancy piece of unicode: \
                    \u{3044}\u{308d}\u{306f}\u{306b}\u{307b}";

        let res = client
            .post("http://host/echo", data, mime::TEXT_PLAIN)
            .perform()
            .expect("request successful");

        assert_eq!(res.status(), StatusCode::OK);

        {
            let content_type = res.headers().get(CONTENT_TYPE).expect("ContentType");
            assert_eq!(content_type, mime::TEXT_PLAIN.as_ref());
        }

        let content_length = {
            let content_length = res.headers().get(CONTENT_LENGTH).expect("ContentLength");
            assert_eq!(content_length, &format!("{}", data.as_bytes().len()));

            content_length
        };

        let buf =
            String::from_utf8(res.read_body().expect("readable response")).expect("UTF8 response");

        assert_eq!(content_length, &format!("{}", buf.len()));
        assert_eq!(data, &buf);
    }
}
