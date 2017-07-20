//! Defines the type `RequestMatcher` and default implementations.

use hyper::server::Request;
use hyper::Method;
use hyper::StatusCode;

use state::{State, request_id};

/// Determines if a `Request` meets pre-defined conditions required for the associated `Route` to
/// be invoked by the `Router` when determining where to dispatch a `Request` to.
pub trait RequestMatcher {
    /// Determines if the `Request` meets pre-defined conditions.
    fn is_match(&self, state: &State, req: &Request) -> Result<(), StatusCode>;
}

/// A `RequestMatcher` that succeeds when the external request has been made with one
/// or more acceptable HTTP request methods.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # fn main() {
/// # use hyper::Method;
/// # use gotham::router::route::request_matcher::MethodOnlyRequestMatcher;
///   let methods = vec![Method::Get, Method::Head];
///   MethodOnlyRequestMatcher::new(methods);
/// # }
/// ```
pub struct MethodOnlyRequestMatcher {
    methods: Vec<Method>,
}

impl MethodOnlyRequestMatcher {
    /// Creates a new `MethodOnlyRequestMatcher`.
    pub fn new(methods: Vec<Method>) -> Self {
        MethodOnlyRequestMatcher { methods }
    }
}

impl RequestMatcher for MethodOnlyRequestMatcher {
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
