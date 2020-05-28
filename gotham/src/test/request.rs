use std::convert::TryFrom;
use std::ops::Deref;
use std::ops::DerefMut;

use hyper::client::connect::Connect;
use hyper::header::{HeaderValue, IntoHeaderName};
use hyper::{Body, Method, Request, Uri};

use super::Server;
use super::{TestClient, TestResponse};

/// Builder API for constructing `Server` requests. When the request is built,
/// `RequestBuilder::perform` will issue the request and provide access to the response.
pub struct TestRequest<'a, S: Server, C: Connect> {
    client: &'a TestClient<S, C>,
    request: Request<Body>,
}

impl<'a, S: Server, C: Connect> Deref for TestRequest<'a, S, C> {
    type Target = Request<Body>;

    fn deref(&self) -> &Request<Body> {
        &self.request
    }
}

impl<'a, S: Server, C: Connect> DerefMut for TestRequest<'a, S, C> {
    fn deref_mut(&mut self) -> &mut Request<Body> {
        &mut self.request
    }
}

impl<'a, S: Server + 'static, C: Connect + Clone + Send + Sync + 'static> TestRequest<'a, S, C> {
    pub(crate) fn new<U>(client: &'a TestClient<S, C>, method: Method, uri: U) -> Self
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
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
    pub fn perform(self) -> anyhow::Result<TestResponse> {
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
