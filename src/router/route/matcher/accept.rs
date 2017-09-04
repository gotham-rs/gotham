//! Defines the type `AcceptMatcher`

use hyper::{Request, StatusCode};
use hyper::header::Accept;
use mime;

use router::route::matcher::RouteMatcher;
use state::{State, request_id};

/// A `RouteMatcher` that succeeds when the `Request` has been made with an `Accept` header that
/// includes 1 or more supported media types. No `Accept` header value or the value of `*/*` will
/// also positvely match.
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
/// # use gotham::router::route::matcher::RouteMatcher;
/// # use gotham::router::route::matcher::accept::AcceptHeaderRouteMatcher;
/// #
///   let supported_media_types = vec![mime::APPLICATION_JSON];
///   let matcher = AcceptHeaderRouteMatcher::new(supported_media_types);
///   let state = State::new();
///   let uri = Uri::from_str("https://example.com").unwrap();
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
pub struct AcceptHeaderRouteMatcher {
    supported_media_types: Vec<mime::Mime>,
}

impl AcceptHeaderRouteMatcher {
    /// Creates a new `AcceptHeaderRouteMatcher`
    pub fn new(supported_media_types: Vec<mime::Mime>) -> Self {
        AcceptHeaderRouteMatcher { supported_media_types }
    }
}

impl RouteMatcher for AcceptHeaderRouteMatcher {
    /// Determines if the `Request` was made using an `Accept` header that
    /// includes 1 or more supported media types. No `Accept` header value or the value of `*/*`
    /// will also positvely match.
    ///
    /// Quality values within `Accept` header values are not considered by the matcher.
    fn is_match(&self, state: &State, req: &Request) -> Result<(), StatusCode> {
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
