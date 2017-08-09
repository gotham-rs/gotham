//! Defines the type `RouteMatcher` and default implementations.

use hyper::Request;
use hyper::Method;
use hyper::StatusCode;

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
            trace!("[{}] matched request method {} to permitted method",
                   request_id(&state),
                   req.method());
            Ok(())
        } else {
            trace!("[{}] did not match request method {}",
                   request_id(&state),
                   req.method());
            Err(StatusCode::MethodNotAllowed)
        }
    }
}
