//! Helpers for HTTP response generation

use hyper::header::{CONTENT_TYPE, LOCATION};
use hyper::{Body, Method, Response, StatusCode};
use mime::Mime;
use std::borrow::Cow;

use helpers::http::header::X_REQUEST_ID;
use state::{request_id, FromState, State};

/// Creates a `Response` object and populates it with a set of default headers that help to improve
/// security and conformance to best practice.
///
/// `create_response` utilises `extend_response`, which delegates to `set_headers` for setting
/// security headers. See `set_headers` for information about the headers which are populated.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
/// # use gotham::state::State;
/// # use gotham::helpers::http::header::X_REQUEST_ID;
/// # use gotham::helpers::http::response::create_response;
/// # use gotham::test::TestServer;
/// #
/// static BODY: &'static [u8] = b"Hello, world!";
///
/// fn handler(state: State) -> (State, Response<Body>) {
///     let response = create_response(
///         &state,
///         StatusCode::OK,
///         (BODY, mime::TEXT_PLAIN),
///     );
///
///     (state, response)
/// }
/// #
/// # fn main() {
/// #     let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// #     let response = test_server
/// #         .client()
/// #         .get("http://example.com/")
/// #         .perform()
/// #         .unwrap();
/// #
/// #     assert_eq!(response.status(), StatusCode::OK);
/// #     assert!(response.headers().get(X_REQUEST_ID).is_some());
/// #
/// #     assert_eq!(
/// #         *response.headers().get(CONTENT_TYPE).unwrap(),
/// #         mime::TEXT_PLAIN.to_string()
/// #     );
/// #
/// #     assert_eq!(
/// #         *response.headers().get(CONTENT_LENGTH).unwrap(),
/// #         format!("{}", BODY.len() as u64)
/// #     );
/// # }
/// ```
pub fn create_response<B: Into<Body>>(
    state: &State,
    status: StatusCode,
    data: (B, Mime),
) -> Response<Body> {
    let (body, mime) = data;
    construct_response(state, status, Some(body), Some(mime))
}

/// Produces a simple empty `Response` with a provided status.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::create_empty_response;
/// # use gotham::test::TestServer;
/// fn handler(state: State) -> (State, Response<Body>) {
///     let resp = create_empty_response(&state, StatusCode::NO_CONTENT);
///
///     (state, resp)
/// }
/// # fn main() {
/// #     let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// #     let response = test_server
/// #         .client()
/// #         .get("http://example.com/")
/// #         .perform()
/// #         .unwrap();
/// #
/// #     assert_eq!(response.status(), StatusCode::NO_CONTENT);
/// # }
/// ```
pub fn create_empty_response(state: &State, status: StatusCode) -> Response<Body> {
    construct_response::<&str>(state, status, None, None)
}

/// Produces a simple empty `Response` with a `Location` header and a 301
/// status.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::create_permanent_redirect;
/// # use gotham::test::TestServer;
/// # use hyper::header::LOCATION;
/// fn handler(state: State) -> (State, Response<Body>) {
///     let resp = create_permanent_redirect(&state, "/over-there");
///
///     (state, resp)
/// }
/// # fn main() {
/// #     let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// #     let response = test_server
/// #         .client()
/// #         .get("http://example.com/")
/// #         .perform()
/// #         .unwrap();
/// #
/// #     assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
/// #     assert_eq!(
/// #         response.headers().get(LOCATION).unwrap(),
/// #         "/over-there"
/// #     );
/// # }
/// ```
pub fn create_permanent_redirect<L: Into<Cow<'static, str>>>(
    state: &State,
    location: L,
) -> Response<Body> {
    let mut res = create_empty_response(state, StatusCode::PERMANENT_REDIRECT);
    res.headers_mut()
        .insert(LOCATION, location.into().to_string().parse().unwrap());
    res
}

/// Produces a simple empty `Response` with a `Location` header and a 302
/// status.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::create_temporary_redirect;
/// # use gotham::test::TestServer;
/// # use hyper::header::LOCATION;
/// fn handler(state: State) -> (State, Response<Body>) {
///     let resp = create_temporary_redirect(&state, "/quick-detour");
///
///     (state, resp)
/// }
/// # fn main() {
/// #     let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// #     let response = test_server
/// #         .client()
/// #         .get("http://example.com/")
/// #         .perform()
/// #         .unwrap();
/// #
/// #     assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
/// #     assert_eq!(
/// #         response.headers().get(LOCATION).unwrap(),
/// #         "/quick-detour"
/// #     );
/// # }
/// ```
pub fn create_temporary_redirect<L: Into<Cow<'static, str>>>(
    state: &State,
    location: L,
) -> Response<Body> {
    let mut res = create_empty_response(state, StatusCode::TEMPORARY_REDIRECT);
    res.headers_mut()
        .insert(LOCATION, location.into().to_string().parse().unwrap());
    res
}

/// Simple response construction.
fn construct_response<B: Into<Body>>(
    state: &State,
    status: StatusCode,
    body: Option<B>,
    mime: Option<Mime>,
) -> Response<Body> {
    let mut builder = Response::builder();

    // always add status and req-id
    builder.status(status);
    builder.header(X_REQUEST_ID, request_id(state));

    // attach mime type when available
    if let Some(mime) = mime {
        builder.header(CONTENT_TYPE, mime.as_ref());
    }

    // attach body when available, but not on HEAD requests (which have no content)
    let built = if body.is_some() && Method::borrow_from(state) != Method::HEAD {
        builder.body(body.unwrap().into())
    } else {
        builder.body(Body::empty())
    };

    // this expect should be safe due to generic bounds
    built.expect("Response built from a compatible type")
}
