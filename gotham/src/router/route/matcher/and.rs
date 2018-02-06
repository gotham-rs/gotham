//! Defines the type `AndRouteMatcher`

use router::non_match::RouteNonMatch;
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
/// # use hyper::Method;
/// # use hyper::header::{Headers, Accept};
/// # use gotham::state::State;
/// # use gotham::router::route::matcher::{RouteMatcher, MethodOnlyRouteMatcher};
/// # use gotham::router::route::matcher::and::AndRouteMatcher;
/// # use gotham::router::route::matcher::accept::AcceptHeaderRouteMatcher;
/// #
///   let methods = vec![Method::Get, Method::Head];
///   let supported_media_types = vec![mime::APPLICATION_JSON];
///   let method_matcher = MethodOnlyRouteMatcher::new(methods);
///	  let accept_matcher = AcceptHeaderRouteMatcher::new(supported_media_types);
///   let matcher = AndRouteMatcher::new(method_matcher, accept_matcher);
///
///   let mut state = State::new();
///   state.put(Method::Get);
///
///   // Request that matches both requirements
///   let mut headers = Headers::new();
///   headers.set(Accept::json());
///   state.put(headers);
///   assert!(matcher.is_match(&state).is_ok());
///
///   // Request that fails method requirements
///   state.put(Method::Post);
///   assert!(matcher.is_match(&state).is_err());
///
///   // Request that fails accept header
///   state.put(Method::Get);
///   let mut headers = Headers::new();
///   headers.set(Accept::text());
///   state.put(headers);
///   assert!(matcher.is_match(&state).is_err());
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
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        match (self.t.is_match(state), self.u.is_match(state)) {
            (Ok(_), Ok(_)) => Ok(()),
            (Err(e), Ok(_)) => Err(e),
            (Ok(_), Err(e)) => Err(e),
            (Err(e), Err(e1)) => Err(e.intersection(e1)),
        }
    }
}
