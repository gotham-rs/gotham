pub(crate) mod async_test;

/// Test request behavior, shared between the tls::test and plain::test modules.
pub mod request;

use std::convert::TryFrom;
use std::fmt;
use std::future::Future;
use std::ops::{Deref, DerefMut};

use anyhow::anyhow;
use futures_util::future::{self, FutureExt, TryFuture, TryFutureExt};
use hyper::client::connect::Connect;
use hyper::client::Client;
use hyper::header::CONTENT_TYPE;
use hyper::{body, http, Body, Method, Response, Uri};
use log::warn;
use tokio::time::{sleep, Sleep};

use crate::handler::NewHandler;
pub use crate::plain::test::TestServer;
pub use request::TestRequest;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;

// publicly reexport the AsyncTestServer helper types.
pub use async_test::{AsyncTestClient, AsyncTestRequestBuilder, AsyncTestResponse};

pub(crate) trait BodyReader {
    /// Runs the underlying event loop until the response body has been fully read. An `Ok(_)`
    /// response holds a buffer containing all bytes of the response body.
    fn read_body(&mut self, response: Response<Body>) -> Result<Vec<u8>, hyper::Error>;
}

pub(crate) struct TestServerData {
    pub(crate) addr: SocketAddr,
    pub(crate) timeout: u64,
    pub(crate) runtime: RwLock<Runtime>,
}

impl TestServerData {
    pub(crate) fn new<NH, F, Wrapped, Wrap>(
        new_handler: NH,
        timeout: u64,
        wrap: Wrap,
    ) -> anyhow::Result<Self>
    where
        NH: NewHandler + 'static,
        F: Future<Output = Result<Wrapped, ()>> + Unpin + Send + 'static,
        Wrapped: Unpin + AsyncRead + AsyncWrite + Send + 'static,
        Wrap: Fn(TcpStream) -> F + Send + 'static,
    {
        let runtime = Runtime::new()?;
        // TODO: Fix this into an async flow
        let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0".parse::<SocketAddr>()?))?;
        let addr = listener.local_addr()?;

        let service_stream = super::bind_server(listener, new_handler, wrap);
        runtime.spawn(service_stream); // Ignore the result

        Ok(TestServerData {
            addr,
            timeout,
            runtime: RwLock::new(runtime),
        })
    }

    pub(crate) fn client<TS, TestC>(&self, server: &TS) -> TestClient<TS, TestC>
    where
        TS: Server,
        TestC: From<SocketAddr> + Connect + Clone,
    {
        // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
        // it and then immediately discard the listener.
        let test_connect = TestC::from(self.addr);
        let client = Client::builder().build(test_connect);

        TestClient {
            client,
            test_server: server.clone(),
        }
    }

    pub(crate) fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime
            .write()
            .expect("unable to acquire read lock")
            .spawn(future);
    }
}

impl Server for Arc<TestServerData> {
    fn run_future<F, O>(&self, future: F) -> O
    where
        F: Future<Output = O>,
    {
        self.runtime
            .write()
            .expect("unable to acquire write lock")
            .block_on(future)
    }

    fn request_expiry(&self) -> Sleep {
        let runtime = self.runtime.write().unwrap();
        let _guard = runtime.enter();
        sleep(Duration::from_secs(self.timeout))
    }
}

/// An in memory server for testing purposes.
pub trait Server: Clone {
    /// Runs a Future until it resolves.
    fn run_future<F, O>(&self, future: F) -> O
    where
        F: Future<Output = O>;

    /// Returns a Delay that will expire when a request should.
    fn request_expiry(&self) -> Sleep;

    /// Runs the event loop until the response future is completed.
    ///
    /// If the future came from a different instance of `Server`, the event loop will run until
    /// the timeout is triggered.
    fn run_request<F>(&self, f: F) -> anyhow::Result<F::Ok>
    where
        F: TryFuture + Unpin + Send + 'static,
        F::Ok: Send,
        F::Error: Into<anyhow::Error> + Send,
    {
        // Note: tokio::time::Sleep does not implement Unpin, so we have to box this future
        let expiry_fut = self
            .request_expiry()
            .then(future::ok::<(), F::Error>)
            .boxed();
        self.run_future(
            // Race the timeout against the request future
            future::try_select(f, expiry_fut)
                // Map an error in either (though it can only occur in the request future)
                .map_err(|either| either.factor_first().0.into())
                // Finally, map the Ok(Either) (left = request, right = timeout) to Ok/Err
                .and_then(|might_expire| {
                    future::ready(match might_expire {
                        future::Either::Left((item, _)) => Ok(item),
                        future::Either::Right(_) => Err(anyhow!("timed out")),
                    })
                })
                .into_future(),
        )
    }
}

impl<T: Server> BodyReader for T {
    fn read_body(&mut self, response: Response<Body>) -> Result<Vec<u8>, hyper::Error> {
        let f = body::to_bytes(response.into_body()).and_then(|b| future::ok(b.to_vec()));
        self.run_future(f)
    }
}

/// Client interface for issuing requests to a `Server`.
pub struct TestClient<TS: Server, C: Connect> {
    pub(crate) client: Client<C, Body>,
    pub(crate) test_server: TS,
}

impl<TS: Server + 'static, C: Connect + Clone + Send + Sync + 'static> TestClient<TS, C> {
    /// Begin constructing a HEAD request using this `TestClient`.
    pub fn head<U>(&self, uri: U) -> TestRequest<'_, TS, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request(Method::HEAD, uri)
    }

    /// Begin constructing a GET request using this `TestClient`.
    pub fn get<U>(&self, uri: U) -> TestRequest<'_, TS, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request(Method::GET, uri)
    }

    /// Begin constructing an OPTIONS request using this `TestClient`.
    pub fn options<U>(&self, uri: U) -> TestRequest<'_, TS, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request(Method::OPTIONS, uri)
    }

    /// Begin constructing a POST request using this `TestClient`.
    pub fn post<B, U>(&self, uri: U, body: B, mime: mime::Mime) -> TestRequest<'_, TS, C>
    where
        B: Into<Body>,
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request_with_body(Method::POST, uri, body, mime)
    }

    /// Begin constructing a PUT request using this `TestClient`.
    pub fn put<B, U>(&self, uri: U, body: B, mime: mime::Mime) -> TestRequest<'_, TS, C>
    where
        B: Into<Body>,
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request_with_body(Method::PUT, uri, body, mime)
    }

    /// Begin constructing a PATCH request using this `TestClient`.
    pub fn patch<B, U>(&self, uri: U, body: B, mime: mime::Mime) -> TestRequest<'_, TS, C>
    where
        B: Into<Body>,
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request_with_body(Method::PATCH, uri, body, mime)
    }

    /// Begin constructing a DELETE request using this `TestClient`.
    pub fn delete<U>(&self, uri: U) -> TestRequest<'_, TS, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request(Method::DELETE, uri)
    }

    /// Begin constructing a request with the given HTTP method and URI.
    pub fn build_request<U>(&self, method: Method, uri: U) -> TestRequest<'_, TS, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
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
    ) -> TestRequest<'_, TS, C>
    where
        B: Into<Body>,
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
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
    pub fn perform(&self, req: TestRequest<'_, TS, C>) -> anyhow::Result<TestResponse> {
        let req_future = self.client.request(req.request()).map_err(|e| {
            warn!("Error from test client request {:?}", e);
            e
        });

        self.test_server
            .run_request(req_future)
            .map(|response| TestResponse {
                response,
                reader: Box::new(self.test_server.clone()),
            })
    }
}

/// Wrapping struct for the `Response` returned by a `TestClient`. Provides access to the
/// `Response` value via the `Deref`, `DerefMut` and `Into` traits, and also provides a function for
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
/// let response = test_server
///     .client()
///     .get("http://localhost/")
///     .perform()
///     .unwrap();
/// assert_eq!(response.status(), StatusCode::OK);
/// let body = response.read_body().unwrap();
/// assert_eq!(&body[..], b"This is the body content.");
/// # }
/// ```
pub struct TestResponse {
    response: Response<Body>,
    reader: Box<dyn BodyReader>,
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TestResponse")
    }
}

impl From<TestResponse> for Response<Body> {
    fn from(response: TestResponse) -> Response<Body> {
        response.response
    }
}

impl TestResponse {
    /// Awaits the body of the underlying `Response`, and returns it. This will cause the event
    /// loop to execute until the `Response` body has been fully read into the `Vec<u8>`.
    pub fn read_body(mut self) -> Result<Vec<u8>, hyper::Error> {
        self.reader.read_body(self.response)
    }

    /// Awaits the UTF-8 encoded body of the underlying `Response`, and returns the `String`. This
    /// will cause the event loop to execute until the `Response` body has been fully read and the
    /// `String` created.
    pub fn read_utf8_body(self) -> anyhow::Result<String> {
        let buf = self.read_body()?;
        let s = String::from_utf8(buf)?;
        Ok(s)
    }
}

#[cfg(test)]
pub(crate) mod helper {
    use crate::handler::{Handler, HandlerFuture, NewHandler};
    use crate::helpers::http::response::create_response;
    use crate::hyper::Body;
    use crate::state::{client_addr, FromState, State};
    use futures_util::{future, FutureExt};
    use hyper::{body, Response, StatusCode, Uri};
    use log::info;
    use std::pin::Pin;

    #[derive(Default, Clone)]
    pub(crate) struct TestHandler {
        pub(crate) response: String,
    }

    impl<T: Into<String>> From<T> for TestHandler {
        fn from(response: T) -> Self {
            Self {
                response: response.into(),
            }
        }
    }

    impl Handler for TestHandler {
        fn handle(self, mut state: State) -> Pin<Box<HandlerFuture>> {
            let path = Uri::borrow_from(&state).path().to_owned();
            match path.as_str() {
                "/" => {
                    info!("TestHandler responding to /");
                    let response = Response::builder()
                        .status(StatusCode::OK)
                        .body(self.response.into())
                        .unwrap();

                    future::ok((state, response)).boxed()
                }
                "/timeout" => {
                    info!("TestHandler responding to /timeout");
                    future::pending().boxed()
                }
                "/myaddr" => {
                    info!("TestHandler responding to /myaddr");
                    let response = Response::builder()
                        .status(StatusCode::OK)
                        .body(format!("{}", client_addr(&state).unwrap()).into())
                        .unwrap();

                    future::ok((state, response)).boxed()
                }
                "/echo" => async move {
                    let body = Body::take_from(&mut state);
                    match body::to_bytes(body).await {
                        Ok(body) => {
                            let response_data = body.to_vec();
                            let response = create_response(
                                &state,
                                StatusCode::OK,
                                mime::TEXT_PLAIN,
                                response_data,
                            );
                            Ok((state, response))
                        }
                        Err(error) => Err((state, error.into())),
                    }
                }
                .boxed(),
                _ => unreachable!(),
            }
        }
    }

    impl NewHandler for TestHandler {
        type Instance = Self;

        fn new_handler(&self) -> anyhow::Result<Self> {
            Ok(self.clone())
        }
    }
}

#[cfg(test)]
pub(crate) mod common_tests {
    use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
    use hyper::StatusCode;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::test::helper::TestHandler;

    pub(crate) fn serves_requests<TS, C>(
        server_factory: fn(TestHandler) -> anyhow::Result<TS>,
        client_factory: fn(&TS) -> TestClient<TS, C>,
    ) where
        TS: Server + 'static,
        C: Connect + Clone + Send + Sync + 'static,
    {
        let ticks = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let test_server = server_factory(TestHandler::from(format!("time: {}", ticks))).unwrap();
        let response = client_factory(&test_server)
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let buf = response.read_utf8_body().unwrap();
        assert_eq!(buf, format!("time: {}", ticks));
    }

    pub(crate) fn times_out<TS, C>(
        server_factory: fn(TestHandler, u64) -> anyhow::Result<TS>,
        client_factory: fn(&TS) -> TestClient<TS, C>,
    ) where
        TS: Server + 'static,
        C: Connect + Clone + Send + Sync + 'static,
    {
        // sadly it seems nearly impossible to use `tokio::time::advance` to test this
        let test_server = server_factory(TestHandler::default(), 1).unwrap();
        let result = client_factory(&test_server)
            .get("http://localhost/timeout")
            .perform();
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    pub(crate) fn async_echo<TS, C>(
        server_factory: fn(TestHandler) -> anyhow::Result<TS>,
        client_factory: fn(&TS) -> TestClient<TS, C>,
    ) where
        TS: Server + 'static,
        C: Connect + Clone + Send + Sync + 'static,
    {
        let server = server_factory(TestHandler::default()).unwrap();

        let client = client_factory(&server);
        let data = "This text should get reflected back to us. Even this fancy piece of unicode: \
                    \u{3044}\u{308d}\u{306f}\u{306b}\u{307b}";

        let res = client
            .post("http://example.com/echo", data, mime::TEXT_PLAIN)
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

    pub(crate) fn supports_multiple_servers<TS, C>(
        server_factory: fn(TestHandler) -> anyhow::Result<TS>,
        client_factory: fn(&TS) -> TestClient<TS, C>,
    ) where
        TS: Server + 'static,
        C: Connect + Clone + Send + Sync + 'static,
    {
        let server_a = server_factory(TestHandler::from("A")).unwrap();
        let server_b = server_factory(TestHandler::from("B")).unwrap();

        let client_a = client_factory(&server_a);
        let client_b = client_factory(&server_b);

        let response_a = client_a
            .get("http://localhost/")
            .perform()
            .unwrap()
            .read_utf8_body()
            .unwrap();
        let response_b = client_b
            .get("http://localhost/")
            .perform()
            .unwrap()
            .read_utf8_body()
            .unwrap();

        assert_eq!(response_a, "A");
        assert_eq!(response_b, "B");
    }

    pub(crate) fn adds_client_address_to_state<TS, C>(
        server_factory: fn(TestHandler) -> anyhow::Result<TS>,
        client_factory: fn(&TS) -> TestClient<TS, C>,
    ) where
        TS: Server + 'static,
        C: Connect + Clone + Send + Sync + 'static,
    {
        let server = server_factory(TestHandler::default()).unwrap();
        let client = client_factory(&server);

        let client_address = client
            .get("http://localhost/myaddr")
            .perform()
            .unwrap()
            .read_utf8_body()
            .unwrap();
        assert!(client_address.starts_with("127.0.0.1"));
    }
}
