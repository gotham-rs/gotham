//! Defines the type `AcceptMatcher`

use hyper::StatusCode;
use hyper::header::{Accept, Headers};
use mime;

use router::non_match::RouteNonMatch;
use router::route::RouteMatcher;
use state::{request_id, FromState, State};

/// A `RouteMatcher` that succeeds when the `Request` has been made with an `Accept` header that
/// includes 1 or more supported media types. No `Accept` header value or the value of `*/*` will
/// also positvely match.
///
/// Quality values within `Accept` header values are not considered by the matcher.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate mime;
/// # fn main() {
/// # use hyper::header::{Headers, Accept};
/// # use gotham::state::State;
/// # use gotham::router::route::matcher::RouteMatcher;
/// # use gotham::router::route::matcher::accept::AcceptHeaderRouteMatcher;
/// #
///   let supported_media_types = vec![mime::APPLICATION_JSON, mime::IMAGE_STAR];
///   let matcher = AcceptHeaderRouteMatcher::new(supported_media_types);
///   let mut state = State::new();
///
///   // No accept header
///   state.put(Headers::new());
///   assert!(matcher.is_match(&state).is_ok());
///
///   // Accept header of `*/*`
///   let mut headers = Headers::new();
///   headers.set(Accept::star());
///   state.put(headers);
///   assert!(matcher.is_match(&state).is_ok());
///
///   // Accept header of `application/json`
///   let mut headers = Headers::new();
///   headers.set(Accept::json());
///   state.put(headers);
///   assert!(matcher.is_match(&state).is_ok());
///
///   // Not a valid Accept header
///   let mut headers = Headers::new();
///   headers.set(Accept::text());
///   state.put(headers);
///   assert!(matcher.is_match(&state).is_err());
///
///   // At least one supported accept header
///   let mut headers = Headers::new();
///   headers.set(Accept::text());
///   headers.set(Accept::json());
///   state.put(headers);
///   assert!(matcher.is_match(&state).is_ok());

///   // Accept header of `image/*`
///   let mut headers = Headers::new();
///   headers.set(Accept::image());
///   state.put(headers);
///   assert!(matcher.is_match(&state).is_ok());
/// # }
/// ```
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
    /// Determines if the `Request` was made using an `Accept` header that
    /// includes 1 or more supported media types. No `Accept` header value or the value of `*/*`
    /// will also positvely match.
    ///
    /// Quality values within `Accept` header values are not considered by the matcher.
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        // Request method is valid, ensure valid Accept header
        let headers = Headers::borrow_from(state);
        match headers.get::<Accept>() {
            Some(accept) => {
                let acceptable_media_types = accept.iter().map(|qi| &qi.item).collect::<Vec<_>>();
                for ra in acceptable_media_types {
                    if *ra == mime::STAR_STAR || self.supported_media_types.contains(ra) {
                        return Ok(());
                    }
                }

                trace!(
                    "[{}] did not provide an Accept with media types supported by this Route",
                    request_id(&state)
                );
                Err(RouteNonMatch::new(StatusCode::NotAcceptable))
            }
            // The client has not specified an `Accept` header, as we can now respond with any type
            // this is valid.
            None => Ok(()),
        }
    }
}
