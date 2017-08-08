//! Helpers for HTTP Response generation

use hyper::{Response, StatusCode, Method};
use hyper::header::{ContentType, ContentLength};
use mime::Mime;

use state::{State, FromState, request_id};
use http::header::{XRequestId, XFrameOptions, XXssProtection, XContentTypeOptions};

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

/// Extends a `Response` object with an optional body and  set of default headers that ensure
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
/// # use std::str::FromStr;
/// # use hyper::{Request, Response, Method, Uri, Body, StatusCode};
/// # use hyper::header::{ContentType, ContentLength};
/// # use gotham::state::State;
/// # use gotham::state::set_request_id;
/// # use gotham::http::response::extend_response;
/// # use gotham::http::header::XRequestId;
/// #
/// # fn main() {
/// #   let mut state = State::new();
/// #   let u = "https://example.com";
/// #   let m = Method::Get;
/// #   let uri = Uri::from_str(u).unwrap();
/// #   let req: Request<Body> = Request::new(m.clone(), uri);
/// #   let req_id = String::from(set_request_id(&mut state, &req));
/// #   state.put(m);
///     let status = StatusCode::Ok;
///     let mime = mime::TEXT_PLAIN;
///     let expected_mime = mime.clone();
///     let body = String::from("Hello world!");
///     let expected_body = body.clone();
///     let mut res = Response::new();
///     extend_response(&state, &mut res, status, Some((body.into_bytes(), mime)));
///     assert!(res.body_ref().is_some());
///     assert_eq!(res.headers().get::<XRequestId>().unwrap().as_str(), req_id);
///     assert_eq!(*res.headers().get::<ContentType>().unwrap(), ContentType(expected_mime));
///     assert_eq!(*res.headers().get::<ContentLength>().unwrap(), ContentLength(expected_body.into_bytes().len() as u64));
/// # }
/// ```
pub fn extend_response(state: &State, res: &mut Response, status: StatusCode, body: Option<Body>) {
    if usize::max_value() > u64::max_value() as usize {
        error!("[{}] unable to handle content_length of response, outside u64 bounds",
               request_id(state));
        panic!("[{}] unable to handle content_length of response, outside u64 bounds",
               request_id(state));
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
/// # use std::str::FromStr;
/// # use hyper::{Request, Response, Method, Uri, Body};
/// # use hyper::header::{ContentType, ContentLength};
/// # use gotham::state::State;
/// # use gotham::state::set_request_id;
/// # use gotham::http::response::set_headers;
/// # use gotham::http::header::XRequestId;
/// #
/// # fn main() {
/// #   let mut state = State::new();
/// #   let u = "https://example.com";
/// #   let m = Method::Get;
/// #   let uri = Uri::from_str(u).unwrap();
/// #   let req: Request<Body> = Request::new(m, uri);
/// #   let req_id = String::from(set_request_id(&mut state, &req));
/// #   let mut res = Response::new();
///     let mime = mime::TEXT_HTML;
///     let expected_mime = mime.clone();
///     set_headers(&state, &mut res, Some(mime), Some(100));
///     assert_eq!(res.headers().get::<XRequestId>().unwrap().as_str(), req_id);
///     assert_eq!(*res.headers().get::<ContentType>().unwrap(), ContentType(expected_mime));
///     assert_eq!(*res.headers().get::<ContentLength>().unwrap(), ContentLength(100));
/// # }
/// ```
///
/// ## Without Mime / ContentLength
///
/// ``` rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use std::str::FromStr;
/// # use hyper::{Request, Response, Method, Uri, Body};
/// # use hyper::header::{ContentType, ContentLength};
/// # use gotham::state::State;
/// # use gotham::state::set_request_id;
/// # use gotham::http::response::set_headers;
/// # use gotham::http::header::XRequestId;
/// #
/// # fn main() {
/// #   let mut state = State::new();
/// #   let u = "https://example.com";
/// #   let m = Method::Get;
/// #   let uri = Uri::from_str(u).unwrap();
/// #   let req: Request<Body> = Request::new(m, uri);
/// #   let req_id = String::from(set_request_id(&mut state, &req));
/// #   let mut res = Response::new();
///     set_headers(&state, &mut res, None, None);
///     assert_eq!(res.headers().get::<XRequestId>().unwrap().as_str(), req_id);
///     assert!(res.headers().get::<ContentType>().is_none());
///     assert_eq!(*res.headers().get::<ContentLength>().unwrap(), ContentLength(0));
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
