//! Defines the `ContentTypeHeaderRouteMatcher`.

use hyper::header::{HeaderMap, CONTENT_TYPE};
use hyper::StatusCode;
use log::trace;
use mime;
use mime::Mime;

use super::{LookupTable, LookupTableFromTypes};
use crate::router::route::RouteMatcher;
use crate::router::RouteNonMatch;
use crate::state::{request_id, FromState, State};

/// A `RouteMatcher` that succeeds when the `Request` has been made with a `Content-Type` header
/// that includes a supported media type. The matcher will fail if the Content-Type
/// header is missing, unless you call `allow_no_type` on it.
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
    supported_media_types: Vec<Mime>,
    lookup_table: LookupTable,
    allow_no_type: bool,
}

impl ContentTypeHeaderRouteMatcher {
    /// Creates a new `ContentTypeHeaderRouteMatcher` that does not allow requests
    /// that don't include a content-type header.
    pub fn new(supported_media_types: Vec<Mime>) -> Self {
        let lookup_table = LookupTable::from_types(supported_media_types.iter(), false);
        Self {
            supported_media_types,
            lookup_table,
            allow_no_type: false,
        }
    }

    /// Modify this matcher to allow requests that don't include a content-type header.
    pub fn allow_no_type(mut self) -> Self {
        self.allow_no_type = true;
        self
    }
}

#[inline]
fn err(state: &State) -> RouteNonMatch {
    trace!(
        "[{}] did not specify a Content-Type with a media type supported by this Route",
        request_id(&state)
    );

    RouteNonMatch::new(StatusCode::UNSUPPORTED_MEDIA_TYPE)
}

impl RouteMatcher for ContentTypeHeaderRouteMatcher {
    /// Determines if the `Request` was made using a `Content-Type` header that includes a
    /// supported media type.
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        HeaderMap::borrow_from(state)
            .get(CONTENT_TYPE)
            .map(|ty| {
                // parse mime type from the content type header
                let mime: Mime = ty
                    .to_str()
                    .map_err(|_| err(state))?
                    .parse()
                    .map_err(|_| err(state))?;

                // get mime type candidates from the lookup table
                let essence = mime.essence_str();
                let candidates = self.lookup_table.get(essence).ok_or_else(|| err(state))?;
                for i in candidates {
                    let candidate = &self.supported_media_types[*i];

                    // check that the candidates have the same suffix - this is not included in the
                    // essence string
                    if candidate.suffix() != mime.suffix() {
                        continue;
                    }

                    // check that this candidate has at least the parameters that the content type
                    // has and that their values are equal
                    if candidate
                        .params()
                        .any(|(key, value)| mime.get_param(key) != Some(value))
                    {
                        continue;
                    }

                    // this candidate matches
                    return Ok(());
                }

                // no candidates found
                Err(err(state))
            })
            .unwrap_or_else(|| {
                // no type present
                if self.allow_no_type {
                    Ok(())
                } else {
                    Err(err(state))
                }
            })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn with_state<F>(content_type: Option<&str>, block: F)
    where
        F: FnOnce(&mut State) -> (),
    {
        State::with_new(|state| {
            let mut headers = HeaderMap::new();
            if let Some(ty) = content_type {
                headers.insert(CONTENT_TYPE, ty.parse().unwrap());
            }
            state.put(headers);
            block(state);
        });
    }

    #[test]
    fn empty_type_list() {
        let matcher = ContentTypeHeaderRouteMatcher::new(Vec::new());
        with_state(None, |state| assert!(matcher.is_match(&state).is_err()));
        with_state(Some("text/plain"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });

        let matcher = matcher.allow_no_type();
        with_state(None, |state| assert!(matcher.is_match(&state).is_ok()));
    }

    #[test]
    fn simple_type() {
        let matcher = ContentTypeHeaderRouteMatcher::new(vec![mime::TEXT_PLAIN]);
        with_state(None, |state| assert!(matcher.is_match(&state).is_err()));
        with_state(Some("text/plain"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
        with_state(Some("text/plain; charset=utf-8"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
    }

    #[test]
    fn complex_type() {
        let matcher = ContentTypeHeaderRouteMatcher::new(vec!["image/svg+xml; charset=utf-8"
            .parse()
            .unwrap()]);
        with_state(Some("image/svg"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });
        with_state(Some("image/svg+xml"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });
        with_state(Some("image/svg+xml; charset=utf-8"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
        with_state(Some("image/svg+xml; charset=utf-8; eol=lf"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
        with_state(Some("image/svg+xml; charset=us-ascii"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });
        with_state(Some("image/svg+json; charset=utf-8"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });
    }

    #[test]
    fn type_mismatch() {
        let matcher = ContentTypeHeaderRouteMatcher::new(vec![mime::TEXT_HTML]);
        with_state(Some("text/plain"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });
    }
}
