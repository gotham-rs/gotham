//! Defines the type `RouteMatcher` and default implementations.

use hyper::header::Accept;
use hyper::Request;
use hyper::Method;
use hyper::StatusCode;
use mime;

use state::{State, request_id};

/// Determines if pre-defined conditions required for the associated `Route` to be invoked by
/// the `Router` have been met.
pub trait RouteMatcher {
    /// Determines if the `Request` meets pre-defined conditions.
    fn is_match(&self, state: &State, req: &Request) -> Result<(), StatusCode>;
}

/// A `RouteMatcher` that succeeds when the `Request` has been made with one
/// or more acceptable HTTP request methods.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # fn main() {
/// # use hyper::{Method, Request, Uri};
/// # use std::str::FromStr;
/// # use gotham::state::State;
/// # use gotham::router::route::matcher::{RouteMatcher, MethodOnlyRouteMatcher};
///   let methods = vec![Method::Get, Method::Head];
///   let matcher = MethodOnlyRouteMatcher::new(methods);
///   let state = State::new();
///   let uri = Uri::from_str("https://example.com").unwrap();
///   let get_request = Request::new(Method::Get, uri.clone());
///   let post_request = Request::new(Method::Post, uri.clone());
///
///   assert!(matcher.is_match(&state, &get_request).is_ok());
///   assert!(matcher.is_match(&state, &post_request).is_err());
/// # }
/// ```
pub struct MethodOnlyRouteMatcher {
    methods: Vec<Method>,
}

impl MethodOnlyRouteMatcher {
    /// Creates a new `MethodOnlyRouteMatcher`.
    pub fn new(methods: Vec<Method>) -> Self {
        MethodOnlyRouteMatcher { methods }
    }
}

impl RouteMatcher for MethodOnlyRouteMatcher {
    /// Determines if the `Request` was made using a `Method` the instance contains.
    fn is_match(&self, state: &State, req: &Request) -> Result<(), StatusCode> {
        if self.methods.iter().any(|m| m == req.method()) {
            trace!(
                "[{}] matched request method {} to permitted method",
                request_id(&state),
                req.method()
            );
            Ok(())
        } else {
            trace!(
                "[{}] did not match request method {}",
                request_id(&state),
                req.method()
            );
            Err(StatusCode::MethodNotAllowed)
        }
    }
}

/// A `RouteMatcher` that succeeds when the `Request` has been made with one
/// or more acceptable HTTP request methods and has indicated an `Accept` header that includes 1 or
/// more supported media types (no `Accept` header value or the value of `*/*`
/// will also positvely match).
///
/// Quality values within `Accept` header values are not considered by the matcher.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// # fn main() {
/// # use hyper::{Method, Request, Uri};
/// # use hyper::header::{Accept};
/// # use std::str::FromStr;
/// # use gotham::state::State;
/// # use gotham::router::route::matcher::{RouteMatcher, MethodAndAcceptRouteMatcher};
///   let methods = vec![Method::Get, Method::Head];
///   let supported_media_types = vec![mime::APPLICATION_JSON];
///   let matcher = MethodAndAcceptRouteMatcher::new(methods, supported_media_types);
///   let state = State::new();
///   let uri = Uri::from_str("https://example.com").unwrap();
///   let get_request = Request::new(Method::Get, uri.clone());
///   let post_request = Request::new(Method::Post, uri.clone());
///
///   assert!(matcher.is_match(&state, &get_request).is_ok());
///   assert!(matcher.is_match(&state, &post_request).is_err());
///
///   // Request with no accept header
///   let accept_req1 = Request::new(Method::Get, uri.clone());
///   assert!(matcher.is_match(&state, &accept_req1).is_ok());
///
///   // Request with accept header of `*/*`
///   let mut accept_req2 = Request::new(Method::Get, uri.clone());
///   accept_req2.headers_mut().set(Accept::star());
///   assert!(matcher.is_match(&state, &accept_req2).is_ok());
///
///   // Request with accept header of `application/json`
///   let mut accept_req3 = Request::new(Method::Get, uri.clone());
///   accept_req3.headers_mut().set(Accept::json());
///   assert!(matcher.is_match(&state, &accept_req3).is_ok());
///
///   // Request with accept header of `text/*`
///   let mut accept_req4 = Request::new(Method::Get, uri.clone());
///   accept_req4.headers_mut().set(Accept::text());
///   assert!(matcher.is_match(&state, &accept_req4).is_err());
///
///   // Request with at least one supported accept header
///   let mut accept_req4 = Request::new(Method::Get, uri.clone());
///   accept_req4.headers_mut().set(Accept::text());
///   accept_req4.headers_mut().set(Accept::json());
///   assert!(matcher.is_match(&state, &accept_req4).is_ok());
/// # }
/// ```
pub struct MethodAndAcceptRouteMatcher {
    morm: MethodOnlyRouteMatcher,
    supported_media_types: Vec<mime::Mime>,
}

impl MethodAndAcceptRouteMatcher {
    /// Creates a new `MethodAndAcceptRouteMatcher`
    pub fn new(methods: Vec<Method>, supported_media_types: Vec<mime::Mime>) -> Self {
        let morm = MethodOnlyRouteMatcher::new(methods);
        MethodAndAcceptRouteMatcher {
            morm,
            supported_media_types,
        }
    }
}

impl RouteMatcher for MethodAndAcceptRouteMatcher {
    /// Determines if the `Request` was made using a `Method` AND provided an `Accept` header that
    /// includes 1 or more supported media types (no `Accept` header value or the value of `*/*`
    /// will also positvely match).
    ///
    /// Quality values within `Accept` header values are not considered by the matcher.
    fn is_match(&self, state: &State, req: &Request) -> Result<(), StatusCode> {
        self.morm.is_match(state, req)?;

        // Request method is valid, ensure valid Accept header
        match req.headers().get::<Accept>() {
            Some(accept) => {
                let acceptable_media_types = accept.iter().map(|qi| &qi.item).collect::<Vec<_>>();
                for ra in acceptable_media_types {
                    if *ra == mime::STAR_STAR || self.supported_media_types.contains(ra) {
                        return Ok(());
                    }
                }

                trace!(
                    "[{}] did not provide an Accept with media types supported by this Route",
                    request_id(&state)
                );
                Err(StatusCode::NotAcceptable)
            }
            // The client has not specified an `Accept` header, as we can now respond with any type
            // this is valid.
            None => Ok(()),
        }
    }
}
