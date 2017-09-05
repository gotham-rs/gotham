//! Defines the type `AnyRouteMatcher`

use hyper::StatusCode;

use router::route::RouteMatcher;
use state::State;

/// Simply matches any Request. Useful when modular applications and wanting to delegate all
/// request handling to a sub-router.
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
/// # use gotham::router::route::matcher::RouteMatcher;
/// # use gotham::router::route::matcher::any::AnyRouteMatcher;
/// #
///   let matcher = AnyRouteMatcher::new();
///   let state = State::new();
///   let uri = Uri::from_str("https://example.com").unwrap();
///
///   let req = Request::new(Method::Get, uri.clone());
///   assert!(matcher.is_match(&state, &req).is_ok());
/// # }
/// ```
pub struct AnyRouteMatcher {}

impl AnyRouteMatcher {
    /// Creates a new `AnyRouteMatcher`
    pub fn new() -> Self {
        AnyRouteMatcher {}
    }
}

impl RouteMatcher for AnyRouteMatcher {
    fn is_match(&self, _state: &State) -> Result<(), StatusCode> {
        Ok(())
    }
}
