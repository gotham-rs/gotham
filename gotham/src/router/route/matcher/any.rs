//! Defines the type `AnyRouteMatcher`

use router::non_match::RouteNonMatch;
use router::route::RouteMatcher;
use state::State;

/// Simply matches any Request. Useful when modular applications and wanting to delegate all
/// request handling to a sub-router.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # fn main() {
/// # use gotham::state::State;
/// # use gotham::router::route::matcher::RouteMatcher;
/// # use gotham::router::route::matcher::any::AnyRouteMatcher;
/// #
///   let matcher = AnyRouteMatcher::new();
///   let state = State::new();
///
///   assert!(matcher.is_match(&state).is_ok());
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
    fn is_match(&self, _state: &State) -> Result<(), RouteNonMatch> {
        Ok(())
    }
}
