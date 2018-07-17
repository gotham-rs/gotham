//! Contains helpers for Gotham applications to use during testing.
//!
//! See the `TestServer` type for example usage.

use std::net::{self, IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use failure;

use futures::{future::{self, FutureResult},
              Future,
              Stream};
use futures_timer::Delay;
use hyper::client::{connect::{Connect, Connected, Destination},
                    Client};
use hyper::header::CONTENT_TYPE;
use hyper::server::conn::Http;
use hyper::{Body, Method, Request, Response, Uri};
use mime;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;

use handler::NewHandler;
use router::Router;
use service::GothamService;

use error::*;

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
    data: Arc<TestServerData<NH>>,
}

struct TestServerData<NH = Router>
where
    NH: NewHandler + Send + 'static,
{
    http: Http,
    timeout: u64,
    runtime: RwLock<Runtime>,
    gotham_service: Arc<GothamService<NH>>,
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
    pub fn new(new_handler: NH) -> Result<TestServer<NH>> {
        TestServer::with_timeout(new_handler, 10)
    }

    /// Sets the request timeout to `timeout` seconds and returns a new `TestServer`.
    pub fn with_timeout(new_handler: NH, timeout: u64) -> Result<TestServer<NH>> {
        let data = TestServerData {
            http: Http::new(),
            timeout,
            runtime: RwLock::new(Runtime::new().unwrap()),
            gotham_service: Arc::new(GothamService::new(new_handler)),
        };

        Ok(TestServer {
            data: Arc::new(data),
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

    fn try_client_with_address(&self, client_addr: net::SocketAddr) -> Result<TestClient<NH>> {
        let (cs, ss) = {
            // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
            // it and then immediately discard the listener.
            let listener = TcpListener::bind(&"localhost:0".parse()?)?;
            let listener_addr = listener.local_addr()?;
            let client = TcpStream::connect(&listener_addr);
            let server = listener.incoming();
            (client, server)
        };

        {
            let data = self.data.clone();
            let service = data.gotham_service.clone();
            let f = self.data
                .http
                //.serve_connection(ss, service)
                .serve_incoming(ss, move || {
                    let ok: FutureResult<_, CompatError> = future::ok(service.connect(client_addr));
                    ok
                })
                .into_future()
                .then(|_| future::ok(()));
            self.data.runtime.write().unwrap().spawn(f);
        };

        let connect = Box::new(
            cs.and_then(|stream| future::ok(TestConnect { stream }))
                .and_then(|co| Ok(Client::builder().build(co)))
                .map_err(|e| Error::from(e).compat()),
        ).shared();

        Ok(TestClient {
            connect,
            test_server: self.clone(),
        })
    }

    /// Runs the event loop until the response future is completed.
    ///
    /// If the future came from a different instance of `TestServer`, the event loop will run until
    /// the timeout is triggered.
    fn run_request<F>(&mut self, f: F) -> Result<F::Item>
    where
        F: Future + Send + 'static,
        F::Error: failure::Fail + Sized,
        F::Item: Send,
    {
        let timeout_duration = Duration::from_secs(self.data.timeout);
        let timeout = Delay::new(timeout_duration);

        match self.run_future(f.select2(timeout).map_err(|either| {
            let e: failure::Error = match either {
                future::Either::A((req_err, _)) => req_err.into(),
                future::Either::B((times_up, _)) => times_up.into(),
            };
            e.compat()
        }))? {
            future::Either::A((item, _)) => Ok(item),
            future::Either::B(_) => Err(failure::err_msg("timed out")),
        }
    }
    /// Runs a future inside of the internal runtime.
    ///
    /// This blocks on the result of the future and behaves like a synchronous
    /// polling call of the future, even if it might be on another thread.
    fn run_future<F, R, E>(&mut self, future: F) -> Result<R>
    where
        F: Send + 'static + Future<Item = R, Error = E>,
        R: Send + 'static,
        E: failure::Fail,
    {
        self.data
            .runtime
            .write()
            .unwrap()
            .block_on(future)
            .map_err(|e| e.into())
    }
}

impl<NH> BodyReader for TestServer<NH>
where
    NH: NewHandler + Send + 'static,
{
    fn read_body(&mut self, response: Response<Body>) -> Result<Vec<u8>> {
        let f = response.into_body();
        let f = f.concat2();

        self.run_future(f).map(|chunk| chunk.into_iter().collect())
    }
}

/// Client interface for issuing requests to a `TestServer`.
pub struct TestClient<NH>
where
    NH: NewHandler + Send + 'static,
{
    connect: future::Shared<
        Box<
            Future<Item = Client<TestConnect, Body>, Error = future::SharedError<CompatError>>
                + Send
                + Sync,
        >,
    >,
    test_server: TestServer<NH>,
}

impl<NH> TestClient<NH>
where
    NH: NewHandler + Send + 'static,
    // + Fn() -> io::Result<H>,
    //H: Send + FnOnce(state::State) -> IHF,
    //IHF: IntoHandlerFuture + Sized,
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
    pub fn perform(mut self, req: Request<Body>) -> Result<TestResponse> {
        let req_future = self.connect
            .clone()
            .map_err(|e| Error::from(e.into()).compat())
            .and_then(|cl| {
                cl.request(req)
                    .map_err(|_| failure::err_msg("request failed").compat())
            });

        self.test_server
            .run_request(req_future)
            .map(move |response| TestResponse {
                response,
                reader: Box::new(self.test_server.clone()),
            })
    }
}

trait BodyReader {
    /// Runs the underlying event loop until the response body has been fully read. An `Ok(_)`
    /// response holds a buffer containing all bytes of the response body.
    fn read_body(&mut self, response: Response<Body>) -> Result<Vec<u8>>;
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
    pub fn read_body(mut self) -> Result<Vec<u8>> {
        self.reader.read_body(self.response)
    }

    /// Awaits the UTF-8 encoded body of the underlying `Response`, and returns the `String`. This
    /// will cause the event loop to execute until the `Response` body has been fully read and the
    /// `String` created.
    pub fn read_utf8_body(self) -> Result<String> {
        let buf = self.read_body()?;
        let s = String::from_utf8(buf)?;
        Ok(s)
    }
}

/// `TestConnect` represents the connection between a test client and the `TestServer` instance
/// that created it. This type should never be used directly.
struct TestConnect {
    stream: TcpStream,
}

impl Connect for TestConnect {
    type Transport = TcpStream;
    type Error = failure::Compat<failure::Error>;
    type Future = Box<Future<Item = (Self::Transport, Connected), Error = Self::Error> + Send>;

    fn connect(&self, _dst: Destination) -> Self::Future {
        Box::new(future::ok((self.stream, Connected::new())))
    }
}

/*
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
*/

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

        fn new_handler(&self) -> Result<Self> {
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
            //Err("timed out") => (),
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
        fn handler(mut state: State) -> Box<HandlerFuture> {
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
