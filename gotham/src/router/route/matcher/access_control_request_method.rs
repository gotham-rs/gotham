//! Defines the `AccessControlRequestMethodMatcher`.

use crate::{
    router::{non_match::RouteNonMatch, route::matcher::RouteMatcher},
    state::{FromState, State},
};
use hyper::{
    header::{HeaderMap, ACCESS_CONTROL_REQUEST_METHOD},
    Method, StatusCode,
};

/// A route matcher that checks whether the value of the `Access-Control-Request-Method` header matches the defined value.
///
/// Usage:
///
/// ```rust
/// # use gotham::{helpers::http::response::create_empty_response,
/// #   hyper::{header::ACCESS_CONTROL_ALLOW_METHODS, Method, StatusCode},
/// #   router::{builder::*, route::matcher::AccessControlRequestMethodMatcher}
/// # };
/// let matcher = AccessControlRequestMethodMatcher::new(Method::PUT);
///
/// # build_simple_router(|route| {
/// // use the matcher for your request
/// route.options("/foo")
/// 	.extend_route_matcher(matcher)
/// 	.to(|state| {
/// 		// we know that this is a CORS preflight for a PUT request
/// 		let mut res = create_empty_response(&state, StatusCode::NO_CONTENT);
/// 		res.headers_mut().insert(ACCESS_CONTROL_ALLOW_METHODS, "PUT".parse().unwrap());
/// 		(state, res)
/// 	});
/// # });
/// ```
#[derive(Clone, Debug)]
pub struct AccessControlRequestMethodMatcher {
    method: Method,
}

impl AccessControlRequestMethodMatcher {
    /// Construct a new matcher that matches if the `Access-Control-Request-Method` header matches `method`.
    /// Note that during matching the method is normalized according to the fetch specification, that is,
    /// byte-uppercased. This means that when using a custom `method` instead of a predefined one, make sure
    /// it is uppercased or this matcher will never succeed.
    pub fn new(method: Method) -> Self {
        Self { method }
    }
}

impl RouteMatcher for AccessControlRequestMethodMatcher {
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        // according to the fetch specification, methods should be normalized by byte-uppercase
        // https://fetch.spec.whatwg.org/#concept-method
        match HeaderMap::borrow_from(state)
            .get(ACCESS_CONTROL_REQUEST_METHOD)
            .and_then(|value| value.to_str().ok())
            .and_then(|str| str.to_ascii_uppercase().parse::<Method>().ok())
        {
            Some(m) if m == self.method => Ok(()),
            _ => Err(RouteNonMatch::new(StatusCode::NOT_FOUND)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn with_state<F>(accept: Option<&str>, block: F)
    where
        F: FnOnce(&mut State) -> (),
    {
        State::with_new(|state| {
            let mut headers = HeaderMap::new();
            if let Some(acc) = accept {
                headers.insert(ACCESS_CONTROL_REQUEST_METHOD, acc.parse().unwrap());
            }
            state.put(headers);
            block(state);
        });
    }

    #[test]
    fn no_acrm_header() {
        let matcher = AccessControlRequestMethodMatcher::new(Method::PUT);
        with_state(None, |state| assert!(matcher.is_match(&state).is_err()));
    }

    #[test]
    fn correct_acrm_header() {
        let matcher = AccessControlRequestMethodMatcher::new(Method::PUT);
        with_state(Some("PUT"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
        with_state(Some("put"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
    }

    #[test]
    fn incorrect_acrm_header() {
        let matcher = AccessControlRequestMethodMatcher::new(Method::PUT);
        with_state(Some("DELETE"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });
    }
}
