//! Helpers for HTTP Response generation

use hyper::{Method, Response, StatusCode};
use hyper::header::{ContentLength, ContentType};
use mime::Mime;

use state::{request_id, FromState, State};
use http::header::{XContentTypeOptions, XFrameOptions, XRequestId, XXssProtection};

type Body = (Vec<u8>, Mime);

/// Creates a `Response` object and populates it with a set of default headers that ensure
/// security and conformance to best practice.
///
/// Internally utilises `extend_response`. Output matches the documented examples for that
/// function.
///
/// The created `Response` should be extended by `Middleware` and `Handler` developers as
/// neceesary.
pub fn create_response(state: &State, status: StatusCode, body: Option<Body>) -> Response {
    let mut res = Response::new();
    extend_response(state, &mut res, status, body);
    res
}

/// Extends a `Response` object with an optional body and set of default headers that ensure
/// security and conformance to best practice.
///
/// # Examples
///
/// ## With body content
///
/// ``` rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// #
/// # use hyper::{Response, StatusCode};
/// # use hyper::header::{ContentType, ContentLength};
/// # use gotham::state::{State, request_id};
/// # use gotham::http::response::extend_response;
/// # use gotham::http::header::XRequestId;
/// # use gotham::test::TestServer;
/// #
/// # fn handler(state: State) -> (State, Response) {
///     let mut res = Response::new();
///     let status = StatusCode::Ok;
///     let body = String::from("Hello world!").into_bytes();
///
///     extend_response(&state, &mut res, status, Some((body.clone(), mime::TEXT_PLAIN)));
///
///     assert!(res.body_ref().is_some());
///     assert_eq!(res.headers().get::<XRequestId>().unwrap().as_str(),
///                request_id(&state));
///     assert_eq!(*res.headers().get::<ContentType>().unwrap(),
///                ContentType(mime::TEXT_PLAIN));
///     assert_eq!(*res.headers().get::<ContentLength>().unwrap(),
///                ContentLength(body.len() as u64));
///
/// #   (state, res)
/// # }
/// #
/// # fn main() {
/// #   let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// #   let response = test_server.client().get("http://example.com/").perform().unwrap();
/// #   assert_eq!(response.status(), StatusCode::Ok);
/// # }
/// ```
pub fn extend_response(state: &State, res: &mut Response, status: StatusCode, body: Option<Body>) {
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
                Method::Head => (),
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
/// ## With ContentLength
///
/// ``` rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// #
/// # use hyper::{Response, StatusCode};
/// # use hyper::header::{ContentType, ContentLength};
/// # use gotham::state::{State, request_id};
/// # use gotham::http::response::set_headers;
/// # use gotham::http::header::XRequestId;
/// # use gotham::test::TestServer;
/// #
/// # fn handler(state: State) -> (State, Response) {
/// #   let mut res = Response::new().with_status(StatusCode::Ok);
///     set_headers(&state, &mut res, Some(mime::TEXT_HTML), Some(100));
///
///     assert_eq!(res.headers().get::<XRequestId>().unwrap().as_str(),
///                request_id(&state));
///     assert_eq!(*res.headers().get::<ContentType>().unwrap(),
///                ContentType(mime::TEXT_HTML));
///     assert_eq!(*res.headers().get::<ContentLength>().unwrap(),
///                ContentLength(100));
/// #
/// #   (state, res)
/// # }
/// #
/// # fn main() {
/// #   let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// #   let response = test_server.client().get("http://example.com/").perform().unwrap();
/// #   assert_eq!(response.status(), StatusCode::Ok);
/// # }
/// ```
///
/// ## Without Mime / ContentLength
///
/// ``` rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Response, StatusCode};
/// # use hyper::header::{ContentType, ContentLength};
/// # use gotham::state::State;
/// # use gotham::state::request_id;
/// # use gotham::http::response::set_headers;
/// # use gotham::http::header::XRequestId;
/// # use gotham::test::TestServer;
/// #
/// # fn handler(state: State) -> (State, Response) {
/// #   let mut res = Response::new().with_status(StatusCode::Accepted);
/// #   {
/// #   let req_id = request_id(&state).clone();
///     set_headers(&state, &mut res, None, None);
///     assert_eq!(res.headers().get::<XRequestId>().unwrap().as_str(), req_id);
///     assert!(res.headers().get::<ContentType>().is_none());
///     assert_eq!(*res.headers().get::<ContentLength>().unwrap(), ContentLength(0));
/// #   }
/// #   (state, res)
/// # }
/// #
/// # fn main() {
/// #   let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// #   let response = test_server.client().get("http://example.com/").perform().unwrap();
/// #   assert_eq!(response.status(), StatusCode::Accepted);
/// # }
/// ```
pub fn set_headers(state: &State, res: &mut Response, mime: Option<Mime>, length: Option<u64>) {
    let headers = res.headers_mut();

    match length {
        Some(length) => headers.set(ContentLength(length)),
        None => headers.set(ContentLength(0)),
    }

    match mime {
        Some(mime) => headers.set(ContentType(mime)),
        None => (),
    };

    headers.set(XRequestId(request_id(state).into()));
    headers.set(XFrameOptions::Deny);
    headers.set(XXssProtection::EnableBlock);
    headers.set(XContentTypeOptions::NoSniff);
}
