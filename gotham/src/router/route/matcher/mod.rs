//! Defines the type `RouteMatcher` and default implementations.

pub mod any;
pub mod and;
pub mod accept;

use std::panic::RefUnwindSafe;

use hyper::{Method, StatusCode};

use state::{request_id, FromState, State};

/// Determines if pre-defined conditions required for the associated `Route` to be invoked by
/// the `Router` have been met.
pub trait RouteMatcher: RefUnwindSafe {
    /// Determines if the `Request` meets pre-defined conditions.
    fn is_match(&self, state: &State) -> Result<(), StatusCode>;

    /// Determines the set of HTTP methods which should be added into a 405 response that
    /// considered this `RouteMatcher`.
    ///
    /// This is **only** used to inform the `Allow` header which is sent for a 405 response, and
    /// may not be suitable for other purposes. In particular, matchers which don't restrict the
    /// HTTP method may return an empty `Vec`.
    fn allow_header_method_list(&self) -> Vec<Method>;
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
/// # use gotham::state::State;
/// # use gotham::router::route::matcher::{RouteMatcher, MethodOnlyRouteMatcher};
///   let methods = vec![Method::Get, Method::Head];
///   let matcher = MethodOnlyRouteMatcher::new(methods);
///   let mut state = State::new();
///
///   state.put(Method::Get);
///   assert!(matcher.is_match(&state).is_ok());
///
///   state.put(Method::Post);
///   assert!(matcher.is_match(&state).is_err());
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
    fn is_match(&self, state: &State) -> Result<(), StatusCode> {
        let method = Method::borrow_from(state);
        if self.methods.iter().any(|m| m == method) {
            trace!(
                "[{}] matched request method {} to permitted method",
                request_id(&state),
                method
            );
            Ok(())
        } else {
            trace!(
                "[{}] did not match request method {}",
                request_id(&state),
                method
            );
            Err(StatusCode::MethodNotAllowed)
        }
    }

    fn allow_header_method_list(&self) -> Vec<Method> {
        self.methods.clone()
    }
}
