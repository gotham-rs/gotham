//! Contains helpers for Gotham applications to use during testing.
//!
//! See the `TestServer` type for example usage.

use std::fmt;
use std::net::{self, IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use failure;

use futures::{future, Future, Stream};
use http::HttpTryFrom;
use hyper::client::{
    connect::{Connect, Connected, Destination},
    Client,
};
use hyper::header::CONTENT_TYPE;
use hyper::{Body, Method, Response, Uri};
use mime;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::timer::Delay;

use handler::NewHandler;

use error::*;

mod request;

pub use self::request::TestRequest;

struct TestServerData {
    addr: SocketAddr,
    timeout: u64,
    runtime: RwLock<Runtime>,
}

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
/// # use hyper::{Body, Response, StatusCode};
/// #
/// # fn my_handler(state: State) -> (State, Response<Body>) {
/// #   (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
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
pub struct TestServer {
    data: Arc<TestServerData>,
}

impl Clone for TestServer {
    fn clone(&self) -> TestServer {
        TestServer {
            data: self.data.clone(),
        }
    }
}

impl TestServer {
    /// Creates a `TestServer` instance for the `Handler` spawned by `new_handler`. This server has
    /// the same guarantee given by `hyper::server::Http::bind`, that a new service will be spawned
    /// for each connection.
    ///
    /// Timeout will be set to 10 seconds.
    pub fn new<NH: NewHandler + 'static>(new_handler: NH) -> Result<TestServer> {
        TestServer::with_timeout(new_handler, 10)
    }

    /// Sets the request timeout to `timeout` seconds and returns a new `TestServer`.
    pub fn with_timeout<NH: NewHandler + 'static>(
        new_handler: NH,
        timeout: u64,
    ) -> Result<TestServer> {
        let mut runtime = Runtime::new()?;
        let listener = TcpListener::bind(&"127.0.0.1:0".parse()?)?;
        let addr = listener.local_addr()?;

        let service_stream = super::bind_server(listener, new_handler);
        runtime.spawn(service_stream);

        let data = TestServerData {
            addr,
            timeout,
            runtime: RwLock::new(runtime),
        };

        Ok(TestServer {
            data: Arc::new(data),
        })
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see a default socket address of `127.0.0.1:10000` as the source address for
    /// the connection.
    pub fn client(&self) -> TestClient {
        self.client_with_address(SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 10000))
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see `client_addr` as the source address for the connection. The
    /// `client_addr` can be any valid `SocketAddr`, and need not be contactable.
    pub fn client_with_address(&self, client_addr: net::SocketAddr) -> TestClient {
        self.try_client_with_address(client_addr)
            .expect("TestServer: unable to spawn client")
    }

    fn try_client_with_address(&self, _client_addr: net::SocketAddr) -> Result<TestClient> {
        // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
        // it and then immediately discard the listener.

        let client = Client::builder().build(TestConnect {
            addr: self.data.addr,
        });

        Ok(TestClient {
            client,
            test_server: self.clone(),
        })
    }

    /// Runs the event loop until the response future is completed.
    ///
    /// If the future came from a different instance of `TestServer`, the event loop will run until
    /// the timeout is triggered.
    fn run_request<F>(&self, f: F) -> Result<F::Item>
    where
        F: Future + Send + 'static,
        F::Error: failure::Fail + Sized,
        F::Item: Send,
    {
        let timeout = Delay::new(Instant::now() + Duration::from_secs(self.data.timeout));
        let might_expire = self.run_future(f.select2(timeout).map_err(|either| {
            let e: failure::Error = match either {
                future::Either::A((req_err, _)) => {
                    warn!("run_request request error: {:?}", req_err);
                    req_err.into()
                }
                future::Either::B((times_up, _)) => {
                    warn!("run_request timed out");
                    times_up.into()
                }
            };
            e.compat()
        }))?;

        match might_expire {
            future::Either::A((item, _)) => Ok(item),
            future::Either::B(_) => Err(failure::err_msg("timed out")),
        }
    }
    /// Runs a future inside of the internal runtime.
    ///
    /// This blocks on the result of the future and behaves like a synchronous
    /// polling call of the future, even if it might be on another thread.
    fn run_future<F, R, E>(&self, future: F) -> Result<R>
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
            .map_err(|e| {
                warn!("Error in test server: {:?}", e);
                e.into()
            })
    }
}

impl BodyReader for TestServer {
    fn read_body(&mut self, response: Response<Body>) -> Result<Vec<u8>> {
        let f = response
            .into_body()
            .concat2()
            .map(|chunk| chunk.into_iter().collect());
        self.run_future(f)
    }
}

/// Client interface for issuing requests to a `TestServer`.
pub struct TestClient {
    client: Client<TestConnect, Body>,
    test_server: TestServer,
}

impl TestClient {
    /// Begin constructing a HEAD request using this `TestClient`.
    pub fn head<U>(&self, uri: U) -> TestRequest
    where
        Uri: HttpTryFrom<U>,
    {
        self.build_request(Method::HEAD, uri)
    }

    /// Begin constructing a GET request using this `TestClient`.
    pub fn get<U>(&self, uri: U) -> TestRequest
    where
        Uri: HttpTryFrom<U>,
    {
        self.build_request(Method::GET, uri)
    }

    /// Begin constructing an OPTIONS request using this `TestClient`.
    pub fn options<U>(&self, uri: U) -> TestRequest
    where
        Uri: HttpTryFrom<U>,
    {
        self.build_request(Method::OPTIONS, uri)
    }

    /// Begin constructing a POST request using this `TestClient`.
    pub fn post<B, U>(&self, uri: U, body: B, mime: mime::Mime) -> TestRequest
    where
        B: Into<Body>,
        Uri: HttpTryFrom<U>,
    {
        self.build_request_with_body(Method::POST, uri, body, mime)
    }

    /// Begin constructing a PUT request using this `TestClient`.
    pub fn put<B, U>(&self, uri: U, body: B, mime: mime::Mime) -> TestRequest
    where
        B: Into<Body>,
        Uri: HttpTryFrom<U>,
    {
        self.build_request_with_body(Method::PUT, uri, body, mime)
    }

    /// Begin constructing a PATCH request using this `TestClient`.
    pub fn patch<B, U>(&self, uri: U, body: B, mime: mime::Mime) -> TestRequest
    where
        B: Into<Body>,
        Uri: HttpTryFrom<U>,
    {
        self.build_request_with_body(Method::PATCH, uri, body, mime)
    }

    /// Begin constructing a DELETE request using this `TestClient`.
    pub fn delete<U>(&self, uri: U) -> TestRequest
    where
        Uri: HttpTryFrom<U>,
    {
        self.build_request(Method::DELETE, uri)
    }

    /// Begin constructing a request with the given HTTP method and URI.
    pub fn build_request<U>(&self, method: Method, uri: U) -> TestRequest
    where
        Uri: HttpTryFrom<U>,
    {
        TestRequest::new(self, method, uri)
    }

    /// Begin constructing a request with the given HTTP method, URI and body.
    pub fn build_request_with_body<B, U>(
        &self,
        method: Method,
        uri: U,
        body: B,
        mime: mime::Mime,
    ) -> TestRequest
    where
        B: Into<Body>,
        Uri: HttpTryFrom<U>,
    {
        let mut request = self.build_request(method, uri);

        {
            let headers = request.headers_mut();
            headers.insert(CONTENT_TYPE, mime.to_string().parse().unwrap());
        }

        *request.body_mut() = body.into();

        request
    }

    /// Send a constructed request using this `TestClient`, and await the response.
    pub fn perform(&self, req: TestRequest) -> Result<TestResponse> {
        let req_future = self.client.request(req.request()).map_err(|e| {
            warn!("Error from test client request {:?}", e);
            failure::err_msg("request failed").compat()
        });

        self.test_server
            .run_request(req_future)
            .map(|response| TestResponse {
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
/// # use hyper::{Body, Response, StatusCode};
/// #
/// # fn my_handler(state: State) -> (State, Response<Body>) {
/// #   let body = "This is the body content.".to_string();
/// #   let response = create_response(&state,
/// #                                  StatusCode::OK,
/// #                                  mime::TEXT_PLAIN,
/// #                                  body);
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
///
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

impl fmt::Debug for TestResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TestResponse")
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
    addr: SocketAddr,
}

impl Connect for TestConnect {
    type Transport = TcpStream;
    type Error = CompatError;
    type Future =
        Box<Future<Item = (Self::Transport, Connected), Error = Self::Error> + Send + Sync>;

    fn connect(&self, _dst: Destination) -> Self::Future {
        Box::new(
            TcpStream::connect(&self.addr)
                .inspect(|s| info!("Client TcpStream connected: {:?}", s))
                .map(|s| (s, Connected::new()))
                .map_err(|e| Error::from(e).compat()),
        )
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
                    info!("TestHandler responding to /");
                    let response = Response::builder()
                        .status(StatusCode::OK)
                        .body(self.response.clone().into())
                        .unwrap();

                    Box::new(future::ok((state, response)))
                }
                "/timeout" => {
                    info!("TestHandler responding to /timeout");
                    Box::new(future::empty())
                }
                "/myaddr" => {
                    info!("TestHandler responding to /myaddr");
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
    #[ignore] // XXX I don't understand why this doesn't work.
              // It seems like Hyper is treating the future::empty() as an empty body...
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
            e @ Err(_) => {
                e.unwrap();
            }
            Ok(_) => panic!("expected timeout, but was Ok(_)"),
        }
    }

    #[test]
    #[ignore] // We trade using the mainline server setup code for this behavior.
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
                        let resp_data = body.to_vec();
                        let res =
                            create_response(&state, StatusCode::OK, mime::TEXT_PLAIN, resp_data);
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
            let mime = res.headers().get(CONTENT_TYPE).expect("ContentType");
            assert_eq!(mime, mime::TEXT_PLAIN.as_ref());
        }

        let content_length = {
            let content_length = res.headers().get(CONTENT_LENGTH).expect("ContentLength");
            assert_eq!(content_length, &format!("{}", data.as_bytes().len()));
            content_length.clone()
        };

        let buf =
            String::from_utf8(res.read_body().expect("readable response")).expect("UTF8 response");

        assert_eq!(content_length, &format!("{}", buf.len()));
        assert_eq!(data, &buf);
    }
}
