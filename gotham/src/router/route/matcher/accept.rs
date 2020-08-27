//! Defines the `AcceptHeaderRouterMatcher`.

use hyper::header::{HeaderMap, ACCEPT};
use hyper::StatusCode;
use log::trace;
use mime;
use mime::Mime;

use super::{LookupTable, LookupTableFromTypes};
use crate::router::route::RouteMatcher;
use crate::router::RouteNonMatch;
use crate::state::{request_id, FromState, State};

/// A mime type that is optionally weighted with a quality.
struct QMime {
    mime: Mime,
    _weight: Option<f32>,
}

impl QMime {
    fn new(mime: Mime, weight: Option<f32>) -> Self {
        Self {
            mime,
            _weight: weight,
        }
    }
}

impl core::str::FromStr for QMime {
    type Err = anyhow::Error;

    fn from_str(str: &str) -> anyhow::Result<Self> {
        match str.find(";q=") {
            None => Ok(Self::new(str.parse()?, None)),
            Some(index) => {
                let mime = str[..index].parse()?;
                let weight = str[index + 3..].parse()?;
                Ok(Self::new(mime, Some(weight)))
            }
        }
    }
}

/// A `RouteMatcher` that succeeds when the `Request` has been made with an `Accept` header that
/// includes one or more supported media types. A missing `Accept` header, or the value of `*/*`
/// will also positvely match. It supports the quality weighted syntax, but does not take the quality
/// into consideration when matching.
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
    lookup_table: LookupTable,
}

impl AcceptHeaderRouteMatcher {
    /// Creates a new `AcceptHeaderRouteMatcher`
    pub fn new(supported_media_types: Vec<mime::Mime>) -> Self {
        let lookup_table = LookupTable::from_types(supported_media_types.iter(), true);
        Self {
            supported_media_types,
            lookup_table,
        }
    }
}

#[inline]
fn err(state: &State) -> RouteNonMatch {
    trace!(
        "[{}] did not provide an Accept with media types supported by this Route",
        request_id(&state)
    );

    RouteNonMatch::new(StatusCode::NOT_ACCEPTABLE)
}

impl RouteMatcher for AcceptHeaderRouteMatcher {
    /// Determines if the `Request` was made using an `Accept` header that includes one or more
    /// supported media types. A missing `Accept` header, or the value of `*/*` will also positvely
    /// match.
    ///
    /// Quality values within `Accept` header values are not considered by the matcher, as the
    /// matcher is only able to indicate whether a successful match has been found.
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        HeaderMap::borrow_from(state)
            .get(ACCEPT)
            .map(|header| {
                // parse mime types from the accept header
                let acceptable = header
                    .to_str()
                    .map_err(|_| err(state))?
                    .split(',')
                    .map(|str| str.trim().parse())
                    .collect::<Result<Vec<QMime>, _>>()
                    .map_err(|_| err(state))?;

                for qmime in acceptable {
                    // get mime type candidates from the lookup table
                    let essence = qmime.mime.essence_str();
                    let candidates = match self.lookup_table.get(essence) {
                        Some(candidates) => candidates,
                        None => continue,
                    };
                    for i in candidates {
                        let candidate = &self.supported_media_types[*i];

                        // check that the candidates have the same suffix - this is not included in the
                        // essence string
                        if candidate.suffix() != qmime.mime.suffix() && qmime.mime.subtype() != "*"
                        {
                            continue;
                        }

                        // this candidate matches - params don't play a role in accept header matching
                        return Ok(());
                    }
                }

                // no candidates found
                Err(err(state))
            })
            .unwrap_or_else(|| {
                // no accept header - assume all types are acceptable
                Ok(())
            })
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
                headers.insert(ACCEPT, acc.parse().unwrap());
            }
            state.put(headers);
            block(state);
        });
    }

    #[test]
    fn no_accept_header() {
        let matcher = AcceptHeaderRouteMatcher::new(vec![mime::TEXT_PLAIN]);
        with_state(None, |state| assert!(matcher.is_match(&state).is_ok()));
    }

    #[test]
    fn single_mime_type() {
        let matcher = AcceptHeaderRouteMatcher::new(vec![mime::TEXT_PLAIN, mime::IMAGE_PNG]);
        with_state(Some("text/plain"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
        with_state(Some("text/html"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });
        with_state(Some("image/png"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
        with_state(Some("image/webp"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });
    }

    #[test]
    fn star_star() {
        let matcher = AcceptHeaderRouteMatcher::new(vec![mime::IMAGE_PNG]);
        with_state(Some("*/*"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
    }

    #[test]
    fn image_star() {
        let matcher = AcceptHeaderRouteMatcher::new(vec![mime::IMAGE_PNG]);
        with_state(Some("image/*"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
    }

    #[test]
    fn suffix_matched_by_wildcard() {
        let matcher = AcceptHeaderRouteMatcher::new(vec!["application/rss+xml".parse().unwrap()]);
        with_state(Some("*/*"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
        with_state(Some("application/*"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
    }

    #[test]
    fn complex_header() {
        let matcher = AcceptHeaderRouteMatcher::new(vec![mime::IMAGE_PNG]);
        with_state(Some("text/html,image/webp;q=0.8"), |state| {
            assert!(matcher.is_match(&state).is_err())
        });
        with_state(Some("text/html,image/webp;q=0.8,*/*;q=0.1"), |state| {
            assert!(matcher.is_match(&state).is_ok())
        });
    }
}
