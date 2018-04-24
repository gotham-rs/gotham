//! Defines the type `AnyRouteMatcher`

use router::non_match::RouteNonMatch;
use router::route::RouteMatcher;
use state::State;

/// Matches any request without restriction (i.e. will accept any request which has already matched
/// the path to the current route). For example, this matcher is used when delegating a path prefix
/// to another router.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # fn main() {
/// #   use gotham::state::State;
/// #   use gotham::router::route::matcher::{AnyRouteMatcher, RouteMatcher};
/// #
/// #   State::with_new(|state| {
/// #
///   let matcher = AnyRouteMatcher::new();
///
///   assert!(matcher.is_match(&state).is_ok());
/// #
/// #   });
/// # }
/// ```
#[derive(Clone)]
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
