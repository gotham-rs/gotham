use hyper::header::{HeaderValue, IntoHeaderName};
use hyper::{Body, Method, Request, Uri};

use handler::NewHandler;
use test::{TestClient, TestRequestError, TestResponse};

/// Builder API for constructing `TestServer` requests. When the request is built,
/// `RequestBuilder::perform` will issue the request and provide access to the response.
#[must_use]
pub struct RequestBuilder<NH>
where
    NH: NewHandler + 'static,
{
    client: TestClient<NH>,
    request: Result<Request<Body>, TestRequestError>,
}

impl<NH> RequestBuilder<NH>
where
    NH: NewHandler + 'static,
{
    pub(super) fn new(client: TestClient<NH>, method: Method, uri: Uri) -> RequestBuilder<NH> {
        let request = Ok(Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap());

        RequestBuilder { client, request }
    }

    /// Adds the given header into the underlying `Request`, replacing any existing header of the
    /// same type.
    pub fn with_header<N>(self, name: N, value: HeaderValue) -> RequestBuilder<NH>
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
    pub fn with_body<T>(self, body: T) -> RequestBuilder<NH>
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
    pub fn perform(self) -> Result<TestResponse<NH>, TestRequestError> {
        self.client.perform(self.request?)
    }
}
