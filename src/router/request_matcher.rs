//! Defines the type `RequestMatcher` and default implementations.

use hyper::server::Request;
use hyper::Method;

/// A type that determines if a `Request` meets pre-defined conditions.
pub trait RequestMatcher {
    /// Determines if the `Request` meets pre-defined conditions.
    fn is_match(&self, req: &Request) -> bool;
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
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
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
    fn is_match(&self, req: &Request) -> bool {
        self.methods.iter().any(|m| m == req.method())
    }
}
