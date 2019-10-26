//! Defines the `AcceptHeaderRouterMatcher`.

use hyper::header::{HeaderMap, HeaderValue, ACCEPT};
use hyper::StatusCode;
use log::trace;
use mime;

use crate::error;
use crate::router::non_match::RouteNonMatch;
use crate::router::route::RouteMatcher;
use crate::state::{request_id, FromState, State};

/// A `RouteMatcher` that succeeds when the `Request` has been made with an `Accept` header that
/// includes one or more supported media types. A missing `Accept` header, or the value of `*/*`
/// will also positively match.
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
/// let supported_media_types = vec![mime::TEXT_CSV, mime::APPLICATION_JSON, mime::IMAGE_STAR];
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
/// headers.append(ACCEPT, "application/json".parse().unwrap());
/// headers.append(ACCEPT, "text/pdf".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());

/// // Accept header of `image/*`
/// let mut headers = HeaderMap::new();
/// headers.insert(ACCEPT, "image/*".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());
///
/// // Accept header of `text/*`
/// let mut headers = HeaderMap::new();
/// headers.insert(ACCEPT, "text/*".parse().unwrap());
/// state.put(headers);
/// assert!(matcher.is_match(&state).is_ok());
///
/// // Accept header of `image/jpeg`
/// // This is because IMAGE_STAR was provided as a supported type.
/// // This might be useful when the available types will only be known at
/// // request time - the handler itself might still return
/// // `StatusCode::NOT_ACCEPTABLE`
/// let mut headers = HeaderMap::new();
/// headers.insert(ACCEPT, "image/jpeg".parse().unwrap());
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

        if self.supported_media_types.is_empty() {
            return Ok(())
        }

        let mut headers = HeaderMap::borrow_from(state).get_all(ACCEPT).iter().peekable();
        if headers.peek().is_none() {
            return Ok(())
        }

        if headers.any(|hv| self.one_match(hv)) {
            Ok(())
        } else {
            trace!(
                "[{}] did not provide an Accept with media types supported by this Route",
                request_id(&state)
                );
            Err(RouteNonMatch::new(StatusCode::NOT_ACCEPTABLE))
        }

    }
}

impl AcceptHeaderRouteMatcher {
    fn one_match(&self, accepted: &HeaderValue) -> bool {
        parse_mime_type(accepted)
            .map(|mime_type| {
                self.supported_media_types.iter().any(|supported| matches(supported, &mime_type))
            }).unwrap_or(false)
    }
}

fn parse_mime_type(hv: &HeaderValue) -> error::Result<mime::Mime> {
    Ok(hv.to_str()?.parse()?)
}

fn matches(provided: &mime::Mime, accepted: &mime::Mime) -> bool {
    match (provided.type_(), accepted.type_()) {
        (mime::STAR, _) | (_, mime::STAR) => true,
        (p, a) if p == a => {
            match (provided.subtype(), accepted.subtype()) {
            (mime::STAR, _) | (_, mime::STAR) => true,
            (ps, ac) if ps == ac => true,
            _ => false
            }
        },
        _ => false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::{HeaderMap, ACCEPT};

    fn setup(state: &mut State, supported_media_types: Vec<mime::Mime>, accept_headers: Vec<&str>) -> AcceptHeaderRouteMatcher {
        // Accept header of `text/*`
        let mut headers = HeaderMap::new();
        for mime in accept_headers {
            headers.append(ACCEPT, mime.parse().unwrap());
        }
        state.put(headers);
        AcceptHeaderRouteMatcher::new(supported_media_types)
    }

    fn matches(supported_media_types: Vec<mime::Mime>, accept_headers: Vec<&str>) -> bool {
        let mut res = false;
        State::with_new(|state| {
            let matcher = setup(state, supported_media_types, accept_headers);
            res = matcher.is_match(&state).is_ok();
        });
        res
    }

    fn doesnt_match(supported_media_types: Vec<mime::Mime>, accept_headers: Vec<&str>) -> bool {
        let mut res = false;
        State::with_new(|state| {
            let matcher = setup(state, supported_media_types, accept_headers);
            res = matcher.is_match(&state).is_err()
        });
        res
    }

    #[test]
    fn matches_empty_accept() {
        assert!(matches(vec![], vec![]));
        assert!(matches(vec![mime::STAR_STAR], vec![]));
        assert!(matches(vec![mime::TEXT_CSV, mime::APPLICATION_JSON, mime::IMAGE_STAR], vec![]));
    }

    #[test]
    fn matches_wildcard_accept() {
        assert!(matches(vec![], vec!["*/*"]));
        assert!(matches(vec![mime::STAR_STAR], vec!["*/*"]));
        assert!(matches(vec![mime::TEXT_CSV, mime::APPLICATION_JSON, mime::IMAGE_STAR], vec!["*/*"]));
    }

    #[test]
    fn matches_specific_headers() {
        assert!(matches(vec![mime::TEXT_PLAIN], vec!["text/plain"]));
        assert!(matches(vec![mime::APPLICATION_JSON], vec!["application/json"]));
    }

    #[test]
    fn matches_specific_to_star() {
        assert!(matches(vec![mime::TEXT_CSV], vec!["text/*"]));
        assert!(matches(vec![mime::APPLICATION_JSON], vec!["application/*"]));
        assert!(matches(vec![mime::TEXT_CSV, mime::APPLICATION_JSON, mime::IMAGE_STAR], vec!["application/*"]));
        assert!(matches(vec![mime::TEXT_CSV, mime::APPLICATION_JSON, mime::IMAGE_STAR], vec!["text/*"]));

        assert!(doesnt_match(vec![mime::TEXT_CSV], vec!["application/*"]));
        assert!(doesnt_match(vec![mime::APPLICATION_JSON], vec!["text/*"]));
    }

    #[test]
    fn matches_star_to_specific() {
        assert!(matches(vec![mime::STAR_STAR], vec!["text/plain"]));
        assert!(matches(vec![mime::TEXT_STAR], vec!["text/plain"]));
        assert!(matches(vec![mime::STAR_STAR], vec!["application/json"]));
        assert!(matches(vec![mime::IMAGE_STAR], vec!["image/jpeg"]));
    }

    #[test]
    fn matches_intersections() {
        assert!(matches(vec![mime::TEXT_CSV, mime::APPLICATION_JSON, mime::IMAGE_STAR], vec!["application/*", "text/plain"]));
        assert!(matches(vec![mime::TEXT_CSV, mime::IMAGE_STAR, mime::APPLICATION_JSON], vec!["application/*", "text/plain"]));
        assert!(matches(vec![mime::TEXT_CSV, mime::APPLICATION_JSON, mime::IMAGE_STAR], vec!["text/plain", "application/*"]));
        assert!(matches(vec![mime::TEXT_CSV, mime::APPLICATION_JSON, mime::IMAGE_STAR], vec!["text/csv", "application/flash"]));
        assert!(matches(vec![mime::TEXT_CSV, mime::APPLICATION_JSON, mime::IMAGE_STAR], vec!["image/png", "application/flash"]));
        assert!(doesnt_match(vec![mime::TEXT_CSV, mime::APPLICATION_JSON], vec!["image/png", "application/flash"]));
    }
}
