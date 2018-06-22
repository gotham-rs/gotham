use hyper::header::{HeaderValue, IntoHeaderName};
use hyper::{Body, Method, Request, Uri};

use handler::NewHandler;
use test::{TestClient, TestRequestError, TestResponse};

/// Builder API for constructing `TestServer` requests. When the request is built,
/// `RequestBuilder::perform` will issue the request and provide access to the response.
#[must_use]
pub struct RequestBuilder<NH, B>
where
    NH: NewHandler<B> + 'static,
{
    client: TestClient<NH, B>,
    request: Result<Request<Body>, TestRequestError>,
}

impl<NH, B> RequestBuilder<NH, B>
where
    NH: NewHandler<B> + 'static,
{
    pub(super) fn new(
        client: TestClient<NH, B>,
        method: Method,
        uri: Uri,
    ) -> RequestBuilder<NH, B> {
        let request = match uri {
            Ok(uri) => Ok(Request::new(method, uri)),
            Err(e) => Err(e.into()),
        };

        RequestBuilder { client, request }
    }

    /// Adds the given header into the underlying `Request`, replacing any existing header of the
    /// same type.
    pub fn with_header<N>(self, name: N, value: HeaderValue) -> RequestBuilder<NH, B>
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
    pub fn with_body<T>(self, body: T) -> RequestBuilder<NH, B>
    where
        T: Into<Body>,
    {
        let mut request = self.request;

        if let Ok(ref mut req) = request {
            req.set_body(body);
        }

        RequestBuilder { request, ..self }
    }

    /// Send a constructed request using the `TestClient` used to create this builder, and await
    /// the response.
    pub fn perform(self) -> Result<TestResponse<NH, B>, TestRequestError> {
        self.client.perform(self.request?)
    }
}
