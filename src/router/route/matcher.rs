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
/// # use hyper::Method;
/// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
///   let methods = vec![Method::Get, Method::Head];
///   MethodOnlyRouteMatcher::new(methods);
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
