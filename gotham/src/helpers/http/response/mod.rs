//! Helpers for HTTP response generation

use http::response;
use hyper::header::{
    HeaderMap, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE, LOCATION, X_CONTENT_TYPE_OPTIONS,
    X_FRAME_OPTIONS, X_XSS_PROTECTION,
};
use hyper::{Body, Method, Response, StatusCode};
use mime::Mime;
use std::borrow::Cow;

use helpers::http::header::X_REQUEST_ID;
use state::{request_id, FromState, State};

// constant strings to be used as header values
const XFO_VALUE: &'static str = "DENY";
const XXP_VALUE: &'static str = "1; mode=block";
const XCTO_VALUE: &'static str = "nosniff";

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
    set_redirect_headers(state, &mut res, location);
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
    set_redirect_headers(state, &mut res, location);
    res
}

/// Extends a `response::Builder` struct with an optional body and set of default headers that help to
/// improve security and conformance to best practice.
///
/// `extend_response` delegates to `set_headers` for setting security headers. See `set_headers`
/// for information about the headers which are populated.
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
/// # use gotham::helpers::http::response::extend_response;
/// # use gotham::test::TestServer;
/// #
/// static BODY: &'static [u8] = b"Hello, world!";
///
/// fn handler(state: State) -> (State, Response<Body>) {
///     let mut response = Response::builder();
///
///     extend_response(
///         &state,
///         StatusCode::OK,
///         &mut response,
///         Some(mime::TEXT_PLAIN),
///     );
///
///     (state, response.body(BODY.into()).unwrap())
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
pub fn extend_response(
    state: &State,
    status: StatusCode,
    builder: &mut response::Builder,
    mime: Option<Mime>,
) {
    if usize::max_value() > u64::max_value() as usize {
        error!(
            "[{}] unable to handle content_length of response, outside u64 bounds",
            request_id(state)
        );
        panic!(
            "[{}] unable to handle content_length of response, outside u64 bounds",
            request_id(state)
        );
    }

    let builder = if let Some(mime) = mime {
        builder.header(CONTENT_TYPE, mime.as_ref())
    } else {
        builder
    };

    builder
        .header(X_REQUEST_ID, request_id(state))
        .status(status);
}

/// Sets a number of default headers in a `Response` that ensure security and conformance to
/// best practice.
///
/// # Examples
///
/// When `Content-Type` and `Content-Length` are not provided, only the security headers are set on
/// the response.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use hyper::header::{X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS, X_XSS_PROTECTION};
/// # use gotham::state::State;
/// # use gotham::helpers::http::header::X_REQUEST_ID;
/// # use gotham::helpers::http::response::set_headers;
/// # use gotham::test::TestServer;
/// #
/// fn handler(state: State) -> (State, Response<Body>) {
///     let mut response = Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap();
///
///     set_headers(
///         &state,
///         &mut response,
///         None,
///         None,
///     );
///
///     (state, response)
/// }
///
/// # fn main() {
/// // Demonstrate the returned headers by making a request to the handler.
/// let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// let response = test_server
///     .client()
///     .get("http://example.com/")
///     .perform()
///     .unwrap();
///
/// assert_eq!(response.status(), StatusCode::ACCEPTED);
///
/// // e.g.:
/// // X-Request-Id: 848c651a-fdd8-4859-b671-3f221895675e
/// # assert!(response.headers().get(X_REQUEST_ID).is_some());
///
/// // X-Frame-Options: DENY
/// assert_eq!(
///     *response.headers().get(X_FRAME_OPTIONS).unwrap(),
///     "DENY"
/// );
///
/// // X-XSS-Protection: 1; mode=block
/// assert_eq!(
///     *response.headers().get(X_XSS_PROTECTION).unwrap(),
///     "1; mode=block"
/// );
///
/// // X-Content-Type-Options: nosniff
/// assert_eq!(
///     *response.headers().get(X_CONTENT_TYPE_OPTIONS).unwrap(),
///     "nosniff"
/// );
/// # }
/// ```
///
/// When the `Content-Type` and `Content-Length` are included, the headers are set in addition to
/// the security headers.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use hyper::header::{X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS, X_XSS_PROTECTION, CONTENT_LENGTH, CONTENT_TYPE};
/// # use gotham::state::State;
/// # use gotham::helpers::http::header::X_REQUEST_ID;
/// # use gotham::helpers::http::response::set_headers;
/// # use gotham::test::TestServer;
/// #
/// static BODY: &'static [u8] = b"Hello, world!";
///
/// fn handler(state: State) -> (State, Response<Body>) {
///     let mut response = Response::builder().status(StatusCode::OK).body(BODY.into()).unwrap();
///
///     set_headers(
///         &state,
///         &mut response,
///         Some(mime::TEXT_PLAIN),
///         Some(BODY.len() as u64),
///     );
///
///     (state, response)
/// }
///
/// # fn main() {
/// // Demonstrate the returned headers by making a request to the handler.
/// let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// let response = test_server
///     .client()
///     .get("http://example.com/")
///     .perform()
///     .unwrap();
///
/// assert_eq!(response.status(), StatusCode::OK);
///
/// assert_eq!(
///     *response.headers().get(CONTENT_TYPE).unwrap(),
///     mime::TEXT_PLAIN.to_string()
/// );
///
/// assert_eq!(
///     *response.headers().get(CONTENT_LENGTH).unwrap(),
///     format!("{}", BODY.len() as u64)
/// );
/// #
/// # // e.g.:
/// # // X-Request-Id: 848c651a-fdd8-4859-b671-3f221895675e
/// # assert!(response.headers().get(X_REQUEST_ID).is_some());
/// #
/// # // X-Frame-Options: DENY
/// # assert_eq!(
/// #     *response.headers().get(X_FRAME_OPTIONS).unwrap(),
/// #     "DENY"
/// # );
/// #
/// # // X-XSS-Protection: 1; mode=block
/// # assert_eq!(
/// #     *response.headers().get(X_XSS_PROTECTION).unwrap(),
/// #     "1; mode=block"
/// # );
/// #
/// # // X-Content-Type-Options: nosniff
/// # assert_eq!(
/// #     *response.headers().get(X_CONTENT_TYPE_OPTIONS).unwrap(),
/// #     "nosniff"
/// # );
/// # }
/// ```
pub fn set_headers<B>(
    state: &State,
    res: &mut Response<B>,
    mime: Option<Mime>,
    length: Option<u64>,
) {
    let headers = res.headers_mut();
    let content_length = length.unwrap_or(0).to_string();

    headers.insert(CONTENT_LENGTH, content_length.parse().unwrap());

    if let Some(mime) = mime {
        headers.insert(CONTENT_TYPE, mime.to_string().parse().unwrap());
    }

    set_request_id(state, headers);

    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static(XFO_VALUE));
    headers.insert(X_XSS_PROTECTION, HeaderValue::from_static(XXP_VALUE));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static(XCTO_VALUE));
}

/// Sets redirect headers on a given `Response`.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use hyper::header::LOCATION;
/// # use gotham::state::State;
/// # use gotham::helpers::http::header::X_REQUEST_ID;
/// # use gotham::helpers::http::response::set_redirect_headers;
/// # use gotham::test::TestServer;
/// fn handler(state: State) -> (State, Response<Body>) {
///     let mut response = Response::builder().status(StatusCode::PERMANENT_REDIRECT).body(Body::empty()).unwrap();
///
///     set_redirect_headers(
///         &state,
///         &mut response,
///         "http://example.com/somewhere-else"
///     );
///
///     (state, response)
/// }
///
/// # fn main() {
/// // Demonstrate the returned headers by making a request to the handler.
/// let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// let response = test_server
///     .client()
///     .get("http://example.com/")
///     .perform()
///     .unwrap();
///
/// assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
///
/// assert_eq!(
///     *response.headers().get(LOCATION).unwrap(),
///     "http://example.com/somewhere-else"
/// );
/// # assert!(response.headers().get(X_REQUEST_ID).is_some());
/// # }
/// ```
pub fn set_redirect_headers<B, L: Into<Cow<'static, str>>>(
    state: &State,
    res: &mut Response<B>,
    location: L,
) {
    let headers = res.headers_mut();
    set_request_id(state, headers);
    headers.insert(LOCATION, location.into().to_string().parse().unwrap());
}

/// Simple response construction.
fn construct_response<B: Into<Body>>(
    state: &State,
    status: StatusCode,
    body: Option<B>,
    mime: Option<Mime>,
) -> Response<Body> {
    let mut builder = Response::builder();

    extend_response(state, status, &mut builder, mime);

    let built = if body.is_some() && Method::borrow_from(state) != Method::HEAD {
        builder.body(body.unwrap().into())
    } else {
        builder.body(Body::empty())
    };

    built.expect("Response built from a compatible type")
}

/// Sets the request id inside a given `HeaderMap`.
fn set_request_id(state: &State, headers: &mut HeaderMap) {
    headers.insert(X_REQUEST_ID, request_id(state).parse().unwrap());
}
