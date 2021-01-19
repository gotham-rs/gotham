/// Test request behavior, shared between the tls::test and plain::test modules.
pub mod request;

use std::convert::TryFrom;
use std::fmt;
use std::ops::{Deref, DerefMut};

use anyhow::anyhow;
use futures::prelude::*;
use hyper::client::connect::Connect;
use hyper::client::Client;
use hyper::header::CONTENT_TYPE;
use hyper::{body, Body, Method, Response, Uri};
use log::warn;
use mime;
use tokio::time::Sleep;

pub use crate::plain::test::TestServer;
use futures::TryFutureExt;
pub use request::TestRequest;

pub(crate) trait BodyReader {
    /// Runs the underlying event loop until the response body has been fully read. An `Ok(_)`
    /// response holds a buffer containing all bytes of the response body.
    fn read_body(&mut self, response: Response<Body>) -> Result<Vec<u8>, hyper::Error>;
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
                .into_future()
                .map_err(|error| error.into()),
        )
    }
}

impl<T: Server> BodyReader for T {
    fn read_body(&mut self, response: Response<Body>) -> Result<Vec<u8>, hyper::Error> {
        let f = body::to_bytes(response.into_body()).and_then(|b| future::ok(b.to_vec()));
        self.run_future(f).map_err(|error| error.into())
    }
}

/// Client interface for issuing requests to a `Server`.
pub struct TestClient<TS: Server, C: Connect> {
    pub(crate) client: Client<C, Body>,
    pub(crate) test_server: TS,
}

impl<TS: Server + 'static, C: Connect + Clone + Send + Sync + 'static> TestClient<TS, C> {
    /// Begin constructing a HEAD request using this `TestClient`.
    pub fn head<U>(&self, uri: U) -> TestRequest<TS, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request(Method::HEAD, uri)
    }

    /// Begin constructing a GET request using this `TestClient`.
    pub fn get<U>(&self, uri: U) -> TestRequest<TS, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request(Method::GET, uri)
    }

    /// Begin constructing an OPTIONS request using this `TestClient`.
    pub fn options<U>(&self, uri: U) -> TestRequest<TS, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request(Method::OPTIONS, uri)
    }

    /// Begin constructing a POST request using this `TestClient`.
    pub fn post<B, U>(&self, uri: U, body: B, mime: mime::Mime) -> TestRequest<TS, C>
    where
        B: Into<Body>,
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request_with_body(Method::POST, uri, body, mime)
    }

    /// Begin constructing a PUT request using this `TestClient`.
    pub fn put<B, U>(&self, uri: U, body: B, mime: mime::Mime) -> TestRequest<TS, C>
    where
        B: Into<Body>,
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request_with_body(Method::PUT, uri, body, mime)
    }

    /// Begin constructing a PATCH request using this `TestClient`.
    pub fn patch<B, U>(&self, uri: U, body: B, mime: mime::Mime) -> TestRequest<TS, C>
    where
        B: Into<Body>,
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request_with_body(Method::PATCH, uri, body, mime)
    }

    /// Begin constructing a DELETE request using this `TestClient`.
    pub fn delete<U>(&self, uri: U) -> TestRequest<TS, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.build_request(Method::DELETE, uri)
    }

    /// Begin constructing a request with the given HTTP method and URI.
    pub fn build_request<U>(&self, method: Method, uri: U) -> TestRequest<TS, C>
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
    ) -> TestRequest<TS, C>
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
    pub fn perform(&self, req: TestRequest<TS, C>) -> anyhow::Result<TestResponse> {
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
/// let response = test_server.client().get("http://localhost/").perform().unwrap();
/// assert_eq!(response.status(), StatusCode::OK);
/// let body = response.read_body().unwrap();
/// assert_eq!(&body[..], b"This is the body content.");
/// # }
/// ```
///
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TestResponse")
    }
}

impl Into<Response<Body>> for TestResponse {
    fn into(self) -> Response<Body> {
        self.response
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
