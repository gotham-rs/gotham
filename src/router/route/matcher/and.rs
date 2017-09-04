//! Defines the type `AndRouteMatcher`

use hyper::{Request, StatusCode};

use router::route::RouteMatcher;
use state::State;

/// Allows multiple Route Matchers to be combined when accessing a request
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
/// # use gotham::router::route::matcher::{RouteMatcher, MethodOnlyRouteMatcher};
/// # use gotham::router::route::matcher::and::AndRouteMatcher;
/// # use gotham::router::route::matcher::accept::AcceptRouteMatcher;
/// #
///   let methods = vec![Method::Get, Method::Head];
///   let supported_media_types = vec![mime::APPLICATION_JSON];
///   let method_matcher = MethodOnlyRouteMatcher::new(methods);
///	  let accept_matcher = AcceptRouteMatcher::new(supported_media_types);
///   let matcher = AndRouteMatcher::new(method_matcher, accept_matcher);
///   let state = State::new();
///   let uri = Uri::from_str("https://example.com").unwrap();
///
///   // Request that matches both requirements
///   let mut req = Request::new(Method::Get, uri.clone());
///   req.headers_mut().set(Accept::json());
///   assert!(matcher.is_match(&state, &req).is_ok());
///
///   // Request that fails method requirements
///   let mut req = Request::new(Method::Post, uri.clone());
///   req.headers_mut().set(Accept::json());
///   assert!(matcher.is_match(&state, &req).is_err());
///
///   // Request that fails accept header
///   let mut req = Request::new(Method::Get, uri.clone());
///   req.headers_mut().set(Accept::text());
///   assert!(matcher.is_match(&state, &req).is_err());
/// # }
/// ```
pub struct AndRouteMatcher<T, U>
where
    T: RouteMatcher,
    U: RouteMatcher,
{
    t: T,
    u: U,
}

impl<T, U> AndRouteMatcher<T, U>
where
    T: RouteMatcher,
    U: RouteMatcher,
{
    /// Creates a new `AndRouteMatcher`
    pub fn new(t: T, u: U) -> Self {
        AndRouteMatcher { t, u }
    }
}

impl<T, U> RouteMatcher for AndRouteMatcher<T, U>
where
    T: RouteMatcher,
    U: RouteMatcher,
{
    fn is_match(&self, state: &State, req: &Request) -> Result<(), StatusCode> {
        self.t.is_match(state, req)?;
        self.u.is_match(state, req)?;

        Ok(())
    }
}
