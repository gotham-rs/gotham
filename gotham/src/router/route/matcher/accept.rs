//! Defines the `AcceptHeaderRouterMatcher`.

use hyper::header::{HeaderMap, ACCEPT};
use hyper::StatusCode;
use mime;

use router::non_match::RouteNonMatch;
use router::route::RouteMatcher;
use state::{request_id, FromState, State};

/// A `RouteMatcher` that succeeds when the `Request` has been made with an `Accept` header that
/// includes one or more supported media types. A missing `Accept` header, or the value of `*/*`
/// will also positvely match.
///
/// Quality values within `Accept` header values are not considered by this matcher.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// # fn main() {
/// #   use hyper::header::{HeaderMap, ACCEPT};
/// #   use gotham::state::State;
/// #   use gotham::router::route::matcher::{AcceptHeaderRouteMatcher, RouteMatcher};
/// #
/// #   State::with_new(|state| {
/// #
/// let supported_media_types = vec![mime::APPLICATION_JSON, mime::IMAGE_STAR];
/// let matcher = AcceptHeaderRouteMatcher::new(supported_media_types);
///
/// // No accept header
/// state.put(HeaderMap::new());
/// assert!(matcher.is_match(&state).is_ok());
///
/// // Accept header of `*/*`
/// let mut headers = HeaderMap::new();
/// headers.insert(ACCEPT, "*/*".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());
///
/// // Accept header of `application/json`
/// let mut headers = HeaderMap::new();
/// headers.insert(ACCEPT, "application/json".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());
///
/// // Not a valid Accept header
/// let mut headers = HeaderMap::new();
/// headers.insert(ACCEPT, "text/plain".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_err());
///
/// // At least one supported accept header
/// let mut headers = HeaderMap::new();
/// headers.insert(ACCEPT, "text/plain".parse().unwrap());
/// headers.insert(ACCEPT, "application/json".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());

/// // Accept header of `image/*`
/// let mut headers = HeaderMap::new();
/// headers.insert(ACCEPT, "image/*".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());
/// #
/// #   });
/// # }
/// ```
#[derive(Clone)]
pub struct AcceptHeaderRouteMatcher {
    supported_media_types: Vec<mime::Mime>,
}

impl AcceptHeaderRouteMatcher {
    /// Creates a new `AcceptHeaderRouteMatcher`
    pub fn new(supported_media_types: Vec<mime::Mime>) -> Self {
        AcceptHeaderRouteMatcher {
            supported_media_types: supported_media_types
                .iter()
                .map(|m| m.to_string())
                .collect(),
        }
    }
}

impl RouteMatcher for AcceptHeaderRouteMatcher {
    /// Determines if the `Request` was made using an `Accept` header that includes one or more
    /// supported media types. A missing `Accept` header, or the value of `*/*` will also positvely
    /// match.
    ///
    /// Quality values within `Accept` header values are not considered by the matcher, as the
    /// matcher is only able to indicate whether a successful match has been found.
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        // Request method is valid, ensure valid Accept header
        let headers = HeaderMap::borrow_from(state);
        match headers.get(ACCEPT) {
            Some(accept) => {
                if accept == "*/*" || self.supported_media_types.contains(accept) {
                    return Ok(());
                }

                trace!(
                    "[{}] did not provide an Accept with media types supported by this Route",
                    request_id(&state)
                );
                Err(RouteNonMatch::new(StatusCode::NOT_ACCEPTABLE))
            }
            // The client has not specified an `Accept` header, as we can now respond with any type
            // this is valid.
            None => Ok(()),
        }
    }
}
