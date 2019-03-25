use std::ops::Deref;
use std::ops::DerefMut;

use http::HttpTryFrom;
use hyper::header::{HeaderValue, IntoHeaderName};
use hyper::{Body, Method, Request, Uri};

use super::TestServer;
use super::{TestClient, TestResponse};

use crate::error::*;

/// Builder API for constructing `TestServer` requests. When the request is built,
/// `RequestBuilder::perform` will issue the request and provide access to the response.
pub struct TestRequest<'a, S: TestServer> {
    client: &'a TestClient<S>,
    request: Request<Body>,
}

impl<'a, S: TestServer> Deref for TestRequest<'a, S> {
    type Target = Request<Body>;

    fn deref(&self) -> &Request<Body> {
        &self.request
    }
}

impl<'a, S: TestServer> DerefMut for TestRequest<'a, S> {
    fn deref_mut(&mut self) -> &mut Request<Body> {
        &mut self.request
    }
}

impl<'a, S: TestServer + 'static> TestRequest<'a, S> {
    pub(crate) fn new<U>(client: &'a TestClient<S>, method: Method, uri: U) -> Self
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
    pub(crate) fn request(self) -> Request<Body> {
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
