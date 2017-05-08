//! Defines a type and default implementations that determine if a segment of the
//! `Request` path is a match for a `Node` in the `Tree`.

/// A type that is used to determine if a segment of the [`Request`][request] path is a
/// match for a [`Node`][node] in the [`Tree`][tree].
///
/// Consider a [`Tree`][tree] with [`Nodes`][node] represented by the following segments:
///
/// ```text
///     /
///     |--segment1      -> (Route)
///     |--segment2
///     |  |--segment2a  -> (Route)
/// ```
///
/// and a [`Request`][request] path of `/segment2/segment2a`.
///
/// In this case the SegmentMatcher **must** discount the [`Node`][node] represented by the
/// segment `segment1` whilst successfully matching against the other segments in order to return
/// a valid response.
///
/// [node]: ../node/struct.Node.html
/// [tree]: ../struct.Tree.html
/// [router]: ../../struct.Router.html
/// [route]: ../../route/trait.Route.html
/// [request]: ../../../../hyper/server/struct.Request.html

pub trait SegmentMatcher {
    /// Returns a positive result if `req_segment` is a match for some pre-existing condition.
    fn is_match(&self, node_segment: &str, req_segment: &str) -> bool;
}

/// A [`SegmentMatcher`][SegmentMatcher] that requires String equivalence.
///
/// ``` rust
/// # use gotham::router::tree::segment_matcher::{SegmentMatcher, StaticSegmentMatcher};
/// let ssm = StaticSegmentMatcher::new();
/// assert!(ssm.is_match("segment", "segment"));
/// assert!(!ssm.is_match("segment", "segment2"));
/// ```
///
/// [SegmentMatcher]: trait.SegmentMatcher.html
#[derive(Clone)]
pub struct StaticSegmentMatcher {}

impl StaticSegmentMatcher {
    /// Creates a new `StaticSegmentMatcher`.
    pub fn new() -> Self {
        StaticSegmentMatcher {}
    }
}

impl SegmentMatcher for StaticSegmentMatcher {
    /// Will return a positive result if `node_segment` and `req_segment` are equivalent.
    fn is_match(&self, node_segment: &str, req_segment: &str) -> bool {
        node_segment == req_segment
    }
}

/// A [`SegmentMatcher`][SegmentMatcher] that matches any provided `req_segment`.
///
/// Facilitates segments whose values:
///
/// * Can not be known at compile time; and
/// * Are of interest to `Handler` implementations via the `State` object for further processing.
///
///
/// # Examples
///
/// ``` rust
/// # use gotham::router::tree::segment_matcher::{SegmentMatcher, DynamicSegmentMatcher};
/// let dsm = DynamicSegmentMatcher::new();
/// assert!(dsm.is_match(":variable", "value"));
/// ```
///
/// [SegmentMatcher]: trait.SegmentMatcher.html
#[derive(Clone)]
pub struct DynamicSegmentMatcher {}

impl DynamicSegmentMatcher {
    /// Creates a new `DynamicSegmentMatcher`.
    pub fn new() -> Self {
        DynamicSegmentMatcher {}
    }
}

impl SegmentMatcher for DynamicSegmentMatcher {
    /// Will always return a positive result.
    fn is_match(&self, _node_segment: &str, _req_segment: &str) -> bool {
        true
    }
}

/// A [`SegmentMatcher`][SegmentMatcher] that matches a subset of provided `req_segment`.
///
/// Facilitates segments whose values:
///
/// * Are partially known at compile time and can be constrained by a regular expression; and
/// * Are of interest to `Handler` implementations via the `State` object for further processing.
///
/// [SegmentMatcher]: trait.SegmentMatcher.html
#[derive(Clone)]
pub struct RegexSegmentMatcher<'a> {
    regex: &'a str,
}

impl<'a> RegexSegmentMatcher<'a> {
    /// Creates a new `RegexSegmentMatcher`
    pub fn new(regex: &'a str) -> Self {
        RegexSegmentMatcher { regex }
    }
}

impl<'a> SegmentMatcher for RegexSegmentMatcher<'a> {
    /// Will return a positive result for `req_segments` that can be matched by the
    /// internally stored regular expression.
    fn is_match(&self, _node_segment: &str, _req_segment: &str) -> bool {
        unimplemented!()
    }
}
