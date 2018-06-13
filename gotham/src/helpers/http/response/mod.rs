//! Helpers for HTTP response generation

use hyper::header::{HeaderMap, HeaderName, CONTENT_LENGTH, CONTENT_TYPE, LOCATION,
                    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS, X_XSS_PROTECTION};
use hyper::{Method, Response, StatusCode};
use mime::Mime;
use std::borrow::Cow;

use state::{request_id, FromState, State};

type Body = (Vec<u8>, Mime);

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
/// # use hyper::{Response, StatusCode};
/// # use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::create_response;
/// # use gotham::test::TestServer;
/// #
/// static BODY: &'static [u8] = b"Hello, world!";
///
/// fn handler(state: State) -> (State, Response) {
///     let response = create_response(
///         &state,
///         StatusCode::OK,
///         Some((BODY.to_vec(), mime::TEXT_PLAIN)),
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
/// #     assert!(response.headers().get_raw("X-Request-ID").is_some());
/// #
/// #     assert_eq!(response.headers()[CONTENT_TYPE], "text/plain");
/// #     assert_eq!(response.headers()[CONTENT_LENGTH], BODY.len().to_string());
/// # }
/// ```
pub fn create_response<B>(state: &State, status: StatusCode, body: Option<Body>) -> Response<B> {
    let mut res = Response::new();
    extend_response(state, &mut res, status, body);
    res
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
/// # use hyper::{Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::create_permanent_redirect;
/// # use gotham::test::TestServer;
/// # use hyper::header::Location;
/// fn handler(state: State) -> (State, Response) {
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
/// #         response.headers().get::<Location>(),
/// #         Some(&Location::new("/over-there"))
/// #     );
/// # }
/// ```
pub fn create_permanent_redirect<B, L: Into<Cow<'static, str>>>(
    state: &State,
    location: L,
) -> Response<B> {
    let mut res = Response::new().with_status(StatusCode::PERMANENT_REDIRECT);
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
/// # use hyper::{Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::create_temporary_redirect;
/// # use gotham::test::TestServer;
/// # use hyper::header::Location;
/// fn handler(state: State) -> (State, Response) {
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
/// #         response.headers().get::<Location>(),
/// #         Some(&Location::new("/quick-detour"))
/// #     );
/// # }
/// ```
pub fn create_temporary_redirect<B, L: Into<Cow<'static, str>>>(
    state: &State,
    location: L,
) -> Response<B> {
    let mut res = Response::new().with_status(StatusCode::TEMPORARY_REDIRECT);
    set_redirect_headers(state, &mut res, location);
    res
}

/// Extends a `Response` object with an optional body and set of default headers that help to
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
/// # use hyper::{Response, StatusCode};
/// # use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::extend_response;
/// # use gotham::test::TestServer;
/// #
/// static BODY: &'static [u8] = b"Hello, world!";
///
/// fn handler(state: State) -> (State, Response) {
///     let mut response = Response::new();
///
///     extend_response(
///         &state,
///         &mut response,
///         StatusCode::OK,
///         Some((BODY.to_vec(), mime::TEXT_PLAIN)),
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
/// #     assert!(response.headers().get_raw("X-Request-ID").is_some());
/// #
/// #     assert_eq!(response.headers()[CONTENT_TYPE], "text/plain");
/// #     assert_eq!(response.headers()[CONTENT_LENGTH], BODY.len().to_string());
/// # }
/// ```
pub fn extend_response<B>(
    state: &State,
    res: &mut Response<B>,
    status: StatusCode,
    body: Option<Body>,
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

    match body {
        Some((body, mime)) => {
            set_headers(state, res, Some(mime), Some(body.len() as u64));
            res.set_status(status);

            match *Method::borrow_from(state) {
                Method::HEAD => (),
                _ => res.set_body(body),
            };
        }
        None => {
            set_headers(state, res, None, None);
            res.set_status(status);
        }
    };
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
/// # use hyper::{Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::set_headers;
/// # use gotham::test::TestServer;
/// #
/// fn handler(state: State) -> (State, Response) {
///     let mut response = Response::new().with_status(StatusCode::ACCEPTED);
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
/// assert!(response.headers().get_raw("X-Request-ID").is_some());
///
/// // X-Frame-Options: DENY
/// assert_eq!(
///     response.headers().get_raw("X-Frame-Options").unwrap(),
///     "DENY",
/// );
///
/// // X-XSS-Protection: 1; mode=block
/// assert_eq!(
///     response.headers().get_raw("X-XSS-Protection").unwrap(),
///     "1; mode=block",
/// );
///
/// // X-Content-Type-Options: nosniff
/// assert_eq!(
///     response.headers().get_raw( "X-Content-Type-Options").unwrap(),
///     "nosniff",
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
/// # use hyper::{Response, StatusCode};
/// # use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::set_headers;
/// # use gotham::test::TestServer;
/// #
/// static BODY: &'static [u8] = b"Hello, world!";
///
/// fn handler(state: State) -> (State, Response) {
///     let mut response = Response::new().with_status(StatusCode::OK).with_body(BODY.to_vec());
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
/// assert_eq!(response.headers()[CONTENT_TYPE], "text/plain");
/// assert_eq!(response.headers()[CONTENT_LENGTH], BODY.len().to_string());
/// #
/// # // e.g.:
/// # // X-Request-Id: 848c651a-fdd8-4859-b671-3f221895675e
/// # assert!(response.headers().get_raw("X-Request-ID").is_some());
/// #
/// # // X-Frame-Options: DENY
/// # assert_eq!(
/// #     response.headers().get_raw("X-Frame-Options").unwrap(),
/// #     "DENY",
/// # );
/// #
/// # // X-XSS-Protection: 1; mode=block
/// # assert_eq!(
/// #     response.headers().get_raw("X-XSS-Protection").unwrap(),
/// #     "1; mode=block",
/// # );
/// #
/// # // X-Content-Type-Options: nosniff
/// # assert_eq!(
/// #     response.headers().get_raw("X-Content-Type-Options").unwrap(),
/// #     "nosniff",
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

    headers.insert(X_FRAME_OPTIONS, "DENY".parse().unwrap());
    headers.insert(X_XSS_PROTECTION, "1; mode=block".parse().unwrap());
    headers.insert(X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
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
/// # use hyper::{Response, StatusCode};
/// # use hyper::header::Location;
/// # use gotham::state::State;
/// # use gotham::helpers::http::response::set_redirect_headers;
/// # use gotham::test::TestServer;
/// fn handler(state: State) -> (State, Response) {
///     let mut response = Response::new().with_status(StatusCode::PERMANENT_REDIRECT);
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
///     *response.headers().get::<Location>().unwrap(),
///     Location::new("http://example.com/somewhere-else")
/// );
/// # assert!(response.headers().get_raw("X-Request-ID").is_some());
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

/// Sets the request id inside a given `HeaderMap`.
fn set_request_id(state: &State, headers: &mut HeaderMap) {
    let request_id = HeaderName::from_lowercase(b"x-request-id").unwrap();
    headers.insert(request_id, request_id(state).parse().unwrap());
}
