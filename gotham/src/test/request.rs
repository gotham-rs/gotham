use std::ops::Deref;
use std::ops::DerefMut;

use http::HttpTryFrom;
use hyper::header::{HeaderValue, IntoHeaderName};
use hyper::{Body, Method, Request, Uri};

use test::{TestClient, TestResponse};

use error::*;

/// Builder API for constructing `TestServer` requests. When the request is built,
/// `RequestBuilder::perform` will issue the request and provide access to the response.
pub struct TestRequest<'a> {
    client: &'a TestClient,
    request: Request<Body>,
}

impl<'a> Deref for TestRequest<'a> {
    type Target = Request<Body>;

    fn deref(&self) -> &Request<Body> {
        &self.request
    }
}

impl<'a> DerefMut for TestRequest<'a> {
    fn deref_mut(&mut self) -> &mut Request<Body> {
        &mut self.request
    }
}

impl<'a> TestRequest<'a> {
    pub(super) fn new<U>(client: &'a TestClient, method: Method, uri: U) -> Self
    where
        Uri: HttpTryFrom<U>,
    {
        TestRequest {
            client,
            request: Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        }
    }

    /// Send a constructed request using the `TestClient`, and await the response.
    pub fn perform(self) -> Result<TestResponse> {
        self.client.perform(self)
    }

    /// Extracts the request from this `TestRequest`.
    pub(super) fn request(self) -> Request<Body> {
        self.request
    }

    /// Adds the given header into the underlying `Request`.
    pub fn with_header<N>(mut self, name: N, value: HeaderValue) -> Self
    where
        N: IntoHeaderName,
    {
        self.headers_mut().insert(name, value);
        self
    }
}
