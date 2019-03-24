/// Test request behavior, shared between the tls::test and plain::test modules.
pub mod request;

use std::fmt;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};

use failure;

use futures::{future, Future, Stream};
use http::HttpTryFrom;
use hyper::client::{
    connect::{Connect, Connected, Destination},
    Client,
};
use hyper::header::CONTENT_TYPE;
use hyper::{Body, Method, Response, Uri};
use log::{info, warn};
use mime;
use tokio::net::{TcpStream};
use std::time::{Duration, Instant};
use tokio::timer::Delay;

use crate::error::*;

pub use request::TestRequest;

pub trait BodyReader {
    /// Runs the underlying event loop until the response body has been fully read. An `Ok(_)`
    /// response holds a buffer containing all bytes of the response body.
    fn read_body(&mut self, response: Response<Body>) -> Result<Vec<u8>>;
}

pub trait TestServer: Clone {
    fn run_future<F, R, E>(&self, future: F) -> Result<R>;

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
}

impl<T: TestServer> BodyReader for T {
    fn read_body(&mut self, response: Response<Body>) -> Result<Vec<u8>> {
        let f = response
            .into_body()
            .concat2()
            .map(|chunk| chunk.into_iter().collect());
        self.run_future(f)
    }
}

/// Client interface for issuing requests to a `TestServer`.
pub struct TestClient<TS: TestServer> {
    client: Client<TestConnect, Body>,
    test_server: TS,
}

impl<TS: TestServer> TestClient<TS> {
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
