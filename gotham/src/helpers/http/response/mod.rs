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
///         mime::TEXT_PLAIN,
///         BODY,
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
pub fn create_response<B>(state: &State, status: StatusCode, mime: Mime, body: B) -> Response<Body>
where
    B: Into<Body>,
{
    // use the basic empty response as a base
    let mut res = create_empty_response(state, status);

    // insert the content type header
    res.headers_mut()
        .insert(CONTENT_TYPE, mime.as_ref().parse().unwrap());

    // add the body on non-HEAD requests
    if Method::borrow_from(state) != Method::HEAD {
        *res.body_mut() = body.into();
    }

    res
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
    // new builder for the response
    let mut builder = Response::builder();

    // always add status and req-id
    builder.status(status);
    builder.header(X_REQUEST_ID, request_id(state));

    // attach an empty body by default
    let built = builder.body(Body::empty());

    // this expect should be safe due to generic bounds
    built.expect("Response built from a compatible type")
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
