use hyper::header::{HeaderValue, IntoHeaderName};
use hyper::{Body, Method, Request, Uri};

use test::{TestClient, TestResponse};

use error::*;

/// Builder API for constructing `TestServer` requests. When the request is built,
/// `RequestBuilder::perform` will issue the request and provide access to the response.
#[must_use]
pub struct RequestBuilder {
    client: TestClient,
    request: Result<Request<Body>>,
}

impl RequestBuilder {
    pub(super) fn new(client: TestClient, method: Method, uri: Uri) -> RequestBuilder {
        let request = Ok(Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap());

        RequestBuilder { client, request }
    }

    /// Adds the given header into the underlying `Request`, replacing any existing header of the
    /// same type.
    pub fn with_header<N>(self, name: N, value: HeaderValue) -> RequestBuilder
    where
        N: IntoHeaderName,
    {
        let mut request = self.request;

        if let Ok(ref mut req) = request {
            req.headers_mut().insert(name, value);
        }

        RequestBuilder { request, ..self }
    }

    /// Adds the given body into the underlying `Request`, replacing any existing body.
    pub fn with_body<T>(self, body: T) -> RequestBuilder
    where
        T: Into<Body>,
    {
        let mut request = self.request;

        if let Ok(ref mut req) = request {
            *req.body_mut() = body.into();
        }

        RequestBuilder { request, ..self }
    }

    /// Send a constructed request using the `TestClient` used to create this builder, and await
    /// the response.
    pub fn perform(self) -> Result<TestResponse> {
        self.client.perform(self.request?)
    }
}
