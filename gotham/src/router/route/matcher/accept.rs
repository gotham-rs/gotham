//! Defines the `AcceptHeaderRouterMatcher`.

use hyper::header::{HeaderMap, HeaderValue, ACCEPT};
use hyper::StatusCode;
use mime;

use error;
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
            supported_media_types,
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
        match HeaderMap::borrow_from(state).get(ACCEPT) {
            // The client has not specified an `Accept` header.
            None => Ok(()),

            // Or the header is any type, so it's fine.
            Some(header) if header == "*/*" => Ok(()),

            // Otherwise we have to validate the header is a match.
            Some(mime_header) => parse_mime_type(mime_header)
                .map_err(|_| RouteNonMatch::new(StatusCode::NOT_ACCEPTABLE))
                .and_then(|mime_type| {
                    if self.supported_media_types.contains(&mime_type) {
                        return Ok(());
                    }
                    trace!(
                        "[{}] did not provide an Accept with media types supported by this Route",
                        request_id(&state)
                    );
                    Err(RouteNonMatch::new(StatusCode::NOT_ACCEPTABLE))
                }),
        }
    }
}

fn parse_mime_type(hv: &HeaderValue) -> error::Result<mime::Mime> {
    Ok(hv.to_str()?.parse()?)
}
