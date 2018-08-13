//! Defines the `ContentTypeHeaderRouteMatcher`.

use hyper::header::{HeaderMap, CONTENT_TYPE};
use hyper::StatusCode;
use mime;

use router::non_match::RouteNonMatch;
use router::route::RouteMatcher;
use state::{request_id, FromState, State};

/// A `RouteMatcher` that succeeds when the `Request` has been made with a `Content-Type` header
/// that includes a supported media type. The matcher will fail if the Content-Type
/// header is missing.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// # fn main() {
/// #   use hyper::header::{HeaderMap, CONTENT_TYPE};
/// #   use gotham::state::State;
/// #   use gotham::router::route::matcher::RouteMatcher;
/// #   use gotham::router::route::matcher::content_type::ContentTypeHeaderRouteMatcher;
/// #
/// #   State::with_new(|state| {
/// #
/// let supported_media_types = vec![mime::APPLICATION_JSON];
/// let matcher = ContentTypeHeaderRouteMatcher::new(supported_media_types);
///
/// // No content type header
/// state.put(HeaderMap::new());
/// assert!(matcher.is_match(&state).is_err());
///
/// // Content type header of `application/json`
/// let mut headers = HeaderMap::new();
/// headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());
///
/// // Not a valid Content-Type header
/// let mut headers = HeaderMap::new();
/// headers.insert(CONTENT_TYPE, "text/plain".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_err());
///
/// // At least one supported content type header
/// let mut headers = HeaderMap::new();
/// headers.insert(CONTENT_TYPE, "text/plain".parse().unwrap());
/// headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());
/// #
/// #   });
/// # }
/// ```
#[derive(Clone)]
pub struct ContentTypeHeaderRouteMatcher {
    supported_media_types: Vec<mime::Mime>,
}
impl ContentTypeHeaderRouteMatcher {
    /// Creates a new `ContentTypeHeaderRouteMatcher`
    pub fn new(supported_media_types: Vec<mime::Mime>) -> Self {
        ContentTypeHeaderRouteMatcher {
            supported_media_types,
        }
    }
}

impl RouteMatcher for ContentTypeHeaderRouteMatcher {
    /// Determines if the `Request` was made using a `Content-Type` header that includes a
    /// supported media type. A missing `Content-Type` header will not match.
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        match HeaderMap::borrow_from(state).get(CONTENT_TYPE) {
            // The client has not specified a `Content-Type` header.
            None => Err(RouteNonMatch::new(StatusCode::UNSUPPORTED_MEDIA_TYPE)),

            // Header was provided.
            Some(content_type) => {
                let mime = content_type.to_str().unwrap().parse().unwrap();

                if self.supported_media_types.contains(&mime) {
                    return Ok(());
                }

                trace!(
                    "[{}] did not specify a Content-Type with a media type supported by this Route",
                    request_id(&state)
                );

                Err(RouteNonMatch::new(StatusCode::UNSUPPORTED_MEDIA_TYPE))
            }
        }
    }
}
