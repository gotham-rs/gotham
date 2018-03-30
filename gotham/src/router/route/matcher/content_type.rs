//! Defines the `ContentTypeHeaderRouteMatcher`.

use hyper::StatusCode;
use hyper::header::{ContentType, Headers};
use mime;

use router::non_match::RouteNonMatch;
use router::route::RouteMatcher;
use state::{request_id, FromState, State};

/// A `RouteMatcher` that succeeds when the `Request` has been made with a `Content-Type` header
/// that includes one or more supported media types. If the Content-Type is missing
/// the matcher will fail.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// # fn main() {
/// #   use hyper::header::{Headers, ContentType};
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
/// state.put(Headers::new());
/// assert!(matcher.is_match(&state).is_err());
///
/// // Content type header of `application/json`
/// let mut headers = Headers::new();
/// headers.set(ContentType::json());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());
///
/// // Not a valid Conent-Type header
/// let mut headers = Headers::new();
/// headers.set(ContentType::text());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_err());
///
/// // At least one supported content type header
/// let mut headers = Headers::new();
/// headers.set(ContentType::text());
/// headers.set(ContentType::json());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());
/// #
/// #   });
/// # }
/// ```
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
    /// Determines if the `Request` was made using a `Content-Type` header that includes one or more
    /// supported media types. A missing `Content-Type` header, or the value of `*/*` will not match.
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        // Request method is valid, ensure valid Accept header
        let headers = Headers::borrow_from(state);
        match headers.get::<ContentType>() {
            Some(content_type) => {
                if self.supported_media_types.contains(&content_type.0) {
                    return Ok(());
                }

                trace!(
                    "[{}] did not specify a Content-Type with media types supported by this Route",
                    request_id(&state)
                );
                Err(RouteNonMatch::new(StatusCode::UnsupportedMediaType))
            }
            // The client has not specified a `Content-Type` header.
            None => Err(RouteNonMatch::new(StatusCode::UnsupportedMediaType)),
        }
    }
}
