use hyper::{Request, Uri, Method, Body};
use hyper::error::UriError;
use hyper::header::Header;

use handler::NewHandler;
use test::{TestClient, TestResponse, TestRequestError};

/// Builder API for constructing `TestServer` requests.
#[must_use]
pub struct RequestBuilder<'a, NH>
where
    NH: NewHandler + 'static,
{
    client: TestClient<'a, NH>,
    request: Result<Request, TestRequestError>,
}

impl<'a, NH> RequestBuilder<'a, NH>
where
    NH: NewHandler + 'static,
{
    pub(super) fn new(
        client: TestClient<'a, NH>,
        method: Method,
        uri: Result<Uri, UriError>,
    ) -> RequestBuilder<'a, NH> {
        let request = match uri {
            Ok(uri) => Ok(Request::new(method, uri)),
            Err(e) => Err(e.into()),
        };

        RequestBuilder { client, request }
    }

    /// Adds the given header into the underlying `Request`, replacing any existing header of the
    /// same type.
    pub fn with_header<H>(self, header: H) -> RequestBuilder<'a, NH>
    where
        H: Header,
    {
        let mut request = self.request;

        if let Ok(ref mut req) = request {
            req.headers_mut().set(header);
        }

        RequestBuilder { request, ..self }
    }

    /// Adds the given body into the underlying `Request`, replacing any existing body.
    pub fn with_body<T>(self, body: T) -> RequestBuilder<'a, NH>
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
    pub fn perform(self) -> Result<TestResponse<'a>, TestRequestError> {
        self.client.perform(self.request?)
    }
}
