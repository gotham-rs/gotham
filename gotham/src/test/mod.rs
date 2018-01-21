//! Contains helpers for Gotham applications to use during testing.
//!
//! `TestServer::new(_)` is the most useful entry point.

use std::{cell, io, net, time};
use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::Arc;

use futures::{future, Future, Stream};
use hyper::{self, Body, Method, Request, Response, Uri};
use hyper::client::{self, Client};
use hyper::error::UriError;
use hyper::header::ContentType;
use hyper::server::{self, Http};
use mime;
use mio;
use tokio_core::reactor::{Core, PollEvented, Timeout};

use handler::NewHandler;
use service::GothamService;
use router::Router;

mod request;

pub use self::request::RequestBuilder;

/// The `TestServer` type, which is used as a harness when writing test cases for Hyper services
/// (which Gotham's `Router` is). An instance of `TestServer` is run single-threaded and
/// asynchronous, and only accessible by a client returned from the `TestServer`.
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
/// #   (state, Response::new().with_status(StatusCode::Accepted))
/// # }
/// #
/// # fn main() {
/// use gotham::test::TestServer;
///
/// let test_server = TestServer::new(|| Ok(my_handler)).unwrap();
///
/// let response = test_server.client().get("http://localhost/").perform().unwrap();
/// assert_eq!(response.status(), StatusCode::Accepted);
/// # }
/// ```
pub struct TestServer<NH = Router>
where
    NH: NewHandler + 'static,
{
    data: Rc<TestServerData<NH>>,
}

struct TestServerData<NH = Router>
where
    NH: NewHandler + 'static,
{
    core: RefCell<Core>,
    http: Http,
    timeout: u64,
    gotham_service: GothamService<NH>,
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
    /// The URL could not be parsed when building the request
    UriError(UriError),
}

impl From<UriError> for TestRequestError {
    fn from(error: UriError) -> TestRequestError {
        TestRequestError::UriError(error)
    }
}

impl<NH> Clone for TestServer<NH>
where
    NH: NewHandler + 'static,
{
    fn clone(&self) -> TestServer<NH> {
        TestServer {
            data: self.data.clone(),
        }
    }
}

impl<NH> TestServer<NH>
where
    NH: NewHandler + 'static,
{
    /// Creates a `TestServer` instance for the service spawned by `new_service`. This server has
    /// the same guarantee given by `hyper::server::Http::bind`, that a new service will be spawned
    /// for each connection.
    ///
    /// Timeout will be set to 10 seconds.
    pub fn new(new_handler: NH) -> Result<TestServer<NH>, io::Error> {
        TestServer::with_timeout(new_handler, 10)
    }

    /// Sets the request timeout to `timeout` seconds and returns a new `TestServer`.
    pub fn with_timeout(new_handler: NH, timeout: u64) -> Result<TestServer<NH>, io::Error> {
        Core::new().map(|core| {
            let handle = core.handle();

            let data = TestServerData {
                core: RefCell::new(core),
                http: server::Http::new(),
                timeout,
                gotham_service: GothamService::new(Arc::new(new_handler), handle),
            };

            TestServer {
                data: Rc::new(data),
            }
        })
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see a default value as the source address for the connection.
    pub fn client(&self) -> TestClient<NH> {
        self.client_with_address(SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 10000))
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see `client_addr` as the source address for the connection. The
    /// `client_addr` can be any value, and need not be contactable.
    pub fn client_with_address(&self, client_addr: net::SocketAddr) -> TestClient<NH> {
        self.try_client_with_address(client_addr)
            .expect("TestServer: unable to spawn client")
    }

    fn try_client_with_address(&self, client_addr: net::SocketAddr) -> io::Result<TestClient<NH>> {
        let handle = self.data.core.borrow().handle();

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
        let cs = PollEvented::new(cs, &handle)?;

        let ss = mio::net::TcpStream::from_stream(ss)?;
        let ss = PollEvented::new(ss, &handle)?;

        let service = self.data.gotham_service.connect(client_addr);
        let f = self.data
            .http
            .serve_connection(ss, service)
            .map(|_| ())
            .map_err(|_| ());

        handle.spawn(f);

        let client = Client::configure()
            .connector(TestConnect {
                stream: cell::RefCell::new(Some(cs)),
            })
            .build(&self.data.core.borrow().handle());

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
        F: Future<Error = hyper::Error>,
    {
        let timeout_duration = time::Duration::from_secs(self.data.timeout);
        let timeout = Timeout::new(timeout_duration, &self.data.core.borrow().handle())
            .map_err(|e| TestRequestError::IoError(e))?;

        let run_result = {
            let mut core = self.data.core.borrow_mut();
            core.run(f.select2(timeout))
        };

        match run_result {
            Ok(future::Either::A((item, _))) => Ok(item),
            Ok(future::Either::B(_)) => Err(TestRequestError::TimedOut),
            Err(future::Either::A((e, _))) => Err(TestRequestError::HyperError(e)),
            Err(future::Either::B((e, _))) => Err(TestRequestError::IoError(e)),
        }
    }
}

impl<NH> BodyReader for TestServer<NH>
where
    NH: NewHandler + 'static,
{
    fn read_body(&self, response: Response) -> hyper::Result<Vec<u8>> {
        let mut buf = Vec::new();

        let r = {
            let f: hyper::Body = response.body();
            let f = f.for_each(|chunk| future::ok(buf.extend(chunk.into_iter())));

            let mut core = self.data.core.borrow_mut();
            core.run(f)
        };

        r.map(|_| buf)
    }
}

/// Client interface for issuing requests to a `TestServer`.
pub struct TestClient<NH>
where
    NH: NewHandler + 'static,
{
    client: Client<TestConnect>,
    test_server: TestServer<NH>,
}

impl<NH> TestClient<NH>
where
    NH: NewHandler + 'static,
{
    /// Parse the URI and begin constructing a HEAD request using this `TestClient`.
    pub fn head(self, uri: &str) -> RequestBuilder<NH> {
        self.build_request(Method::Head, uri)
    }

    /// Begin constructing a HEAD request using this `TestClient`.
    pub fn head_uri(self, uri: Uri) -> RequestBuilder<NH> {
        self.build_request_uri(Method::Head, uri)
    }

    /// Parse the URI and begin constructing a GET request using this `TestClient`.
    pub fn get(self, uri: &str) -> RequestBuilder<NH> {
        self.build_request(Method::Get, uri)
    }

    /// Begin constructing a GET request using this `TestClient`.
    pub fn get_uri(self, uri: Uri) -> RequestBuilder<NH> {
        self.build_request_uri(Method::Get, uri)
    }

    /// Parse the URI and begin constructing a POST request using this `TestClient`.
    pub fn post<T>(self, uri: &str, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request(Method::Post, uri)
            .with_body(body)
            .with_header(ContentType(content_type))
    }

    /// Begin constructing a POST request using this `TestClient`.
    pub fn post_uri<T>(self, uri: Uri, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request_uri(Method::Post, uri)
            .with_body(body)
            .with_header(ContentType(content_type))
    }

    /// Parse the URI and begin constructing a PUT request using this `TestClient`.
    pub fn put<T>(self, uri: &str, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request(Method::Put, uri)
            .with_body(body)
            .with_header(ContentType(content_type))
    }

    /// Begin constructing a PUT request using this `TestClient`.
    pub fn put_uri<T>(self, uri: Uri, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request_uri(Method::Put, uri)
            .with_body(body)
            .with_header(ContentType(content_type))
    }

    /// Parse the URI and begin constructing a PATCH request using this `TestClient`.
    pub fn patch<T>(self, uri: &str, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request(Method::Patch, uri)
            .with_body(body)
            .with_header(ContentType(content_type))
    }

    /// Begin constructing a PATCH request using this `TestClient`.
    pub fn patch_uri<T>(self, uri: Uri, body: T, content_type: mime::Mime) -> RequestBuilder<NH>
    where
        T: Into<Body>,
    {
        self.build_request_uri(Method::Patch, uri)
            .with_body(body)
            .with_header(ContentType(content_type))
    }

    /// Parse the URI and begin constructing a DELETE request using this `TestClient`.
    pub fn delete(self, uri: &str) -> RequestBuilder<NH> {
        self.build_request(Method::Delete, uri)
    }

    /// Begin constructing a DELETE request using this `TestClient`.
    pub fn delete_uri(self, uri: Uri) -> RequestBuilder<NH> {
        self.build_request_uri(Method::Delete, uri)
    }

    /// Parse the URI and begin constructing a request with the given HTTP method.
    pub fn build_request(self, method: Method, uri: &str) -> RequestBuilder<NH> {
        RequestBuilder::new(self, method, uri.parse())
    }

    /// Begin constructing a request with the given HTTP method and Uri.
    pub fn build_request_uri(self, method: Method, uri: Uri) -> RequestBuilder<NH> {
        RequestBuilder::new(self, method, Ok(uri))
    }

    /// Send a constructed request using this `TestClient`, and await the response.
    pub fn perform(self, req: Request) -> Result<TestResponse, TestRequestError> {
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
    fn read_body(&self, response: Response) -> hyper::Result<Vec<u8>>;
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
/// # use gotham::http::response::create_response;
/// # use hyper::{Response, StatusCode};
/// #
/// # fn my_handler(state: State) -> (State, Response) {
/// #   let body = "This is the body content.".to_string().into_bytes();;
/// #   let response = create_response(&state,
/// #                                  StatusCode::Ok,
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
/// assert_eq!(response.status(), StatusCode::Ok);
/// let body = response.read_body().unwrap();
/// assert_eq!(&body[..], b"This is the body content.");
/// # }
/// ```
pub struct TestResponse {
    response: Response,
    reader: Box<BodyReader>,
}

impl Deref for TestResponse {
    type Target = Response;

    fn deref(&self) -> &Response {
        &self.response
    }
}

impl DerefMut for TestResponse {
    fn deref_mut(&mut self) -> &mut Response {
        &mut self.response
    }
}

impl TestResponse {
    /// Awaits the body of the underlying `Response`, and returns it. This will cause the event
    /// loop to execute until the `Response` body has been fully read into the `Vec<u8>`.
    pub fn read_body(self) -> hyper::Result<Vec<u8>> {
        self.reader.read_body(self.response)
    }

    /// Awaits the UTF-8 encoded body of the underlying `Response`, and returns the `String`. This
    /// will cause the event loop to execute until the `Response` body has been fully read and the
    /// `String` created.
    pub fn read_utf8_body(self) -> hyper::Result<String> {
        let buf = self.read_body()?;
        let s = String::from_utf8(buf)?;
        Ok(s)
    }
}

/// `TestConnect` represents the connection between a test client and the `TestServer` instance
/// that created it. This type should never be used directly.
pub struct TestConnect {
    stream: cell::RefCell<Option<PollEvented<mio::net::TcpStream>>>,
}

impl client::Service for TestConnect {
    type Request = hyper::Uri;
    type Error = io::Error;
    type Response = PollEvented<mio::net::TcpStream>;
    type Future = future::FutureResult<Self::Response, Self::Error>;

    fn call(&self, _req: Self::Request) -> Self::Future {
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

    use hyper::{Body, StatusCode, Uri};
    use hyper::header::{ContentLength, ContentType};
    use mime;

    use handler::{Handler, HandlerFuture, IntoHandlerError, NewHandler};
    use http::response::create_response;
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

        assert_eq!(response.status(), StatusCode::Ok);
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

        assert_eq!(response.status(), StatusCode::Ok);
        let buf = response.read_body().unwrap();
        let received_addr: net::SocketAddr = String::from_utf8(buf).unwrap().parse().unwrap();
        assert_eq!(received_addr, client_addr);
    }

    #[test]
    fn async_echo() {
        fn handler(mut state: State) -> Box<HandlerFuture> {
            let f = Body::take_from(&mut state)
                .concat2()
                .then(move |full_body| match full_body {
                    Ok(body) => {
                        debug!("test");
                        let resp_data = body.to_vec();
                        let res = create_response(
                            &state,
                            StatusCode::Ok,
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

        assert_eq!(res.status(), StatusCode::Ok);

        {
            let content_type = res.headers().get::<ContentType>().expect("ContentType");
            assert_eq!(content_type.0, mime::TEXT_PLAIN);
        }

        let content_length = {
            let content_length = res.headers().get::<ContentLength>().expect("ContentLength");
            assert_eq!(content_length.0, data.as_bytes().len() as u64);

            content_length.0
        };

        let buf =
            String::from_utf8(res.read_body().expect("readable response")).expect("UTF8 response");

        assert_eq!(content_length, buf.len() as u64);
        assert_eq!(data, &buf);
    }
}
