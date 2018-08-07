//! Defines the type `AndRouteMatcher`

use router::non_match::RouteNonMatch;
use router::route::RouteMatcher;
use state::State;

/// Allows multiple `RouteMatcher` values to be combined when accessing a request.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// # fn main() {
/// #   use hyper::Method;
/// #   use hyper::header::{HeaderMap, ACCEPT};
/// #   use gotham::state::State;
/// #   use gotham::router::route::matcher::{RouteMatcher, MethodOnlyRouteMatcher,
///                                          AndRouteMatcher, AcceptHeaderRouteMatcher};
/// #
/// #   State::with_new(|state| {
/// #
///   let methods = vec![Method::GET, Method::HEAD];
///   let supported_media_types = vec![mime::APPLICATION_JSON];
///   let method_matcher = MethodOnlyRouteMatcher::new(methods);
///	  let accept_matcher = AcceptHeaderRouteMatcher::new(supported_media_types);
///   let matcher = AndRouteMatcher::new(method_matcher, accept_matcher);
///
///   state.put(Method::GET);
///
///   // Request that matches both requirements
///   let mut headers = HeaderMap::new();
///   headers.insert(ACCEPT, mime::APPLICATION_JSON.to_string().parse().unwrap());
///   state.put(headers);
///   assert!(matcher.is_match(&state).is_ok());
///
///   // Request that fails method requirements
///   state.put(Method::POST);
///   assert!(matcher.is_match(&state).is_err());
///
///   // Request that fails accept header
///   state.put(Method::GET);
///   let mut headers = HeaderMap::new();
///   headers.insert(ACCEPT, mime::TEXT.to_string().parse().unwrap());
///   state.put(headers);
///   assert!(matcher.is_match(&state).is_err());
/// #
/// #   });
/// # }
/// ```
#[derive(Clone)]
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
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        match (self.t.is_match(state), self.u.is_match(state)) {
            (Ok(_), Ok(_)) => Ok(()),
            (Err(e), Ok(_)) => Err(e),
            (Ok(_), Err(e)) => Err(e),
            (Err(e), Err(e1)) => Err(e.intersection(e1)),
        }
    }
}
