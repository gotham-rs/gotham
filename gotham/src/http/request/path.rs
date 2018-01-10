//! Defines helper functions for the Request path

use std::sync::Arc;

use http::PercentDecoded;

const EXCLUDED_SEGMENTS: [&str; 1] = [""];

/// Holder for `Request` uri path segments that have been split into individual segments.
///
/// Used with `Tree` traversal.
#[derive(Clone, PartialEq)]
pub struct RequestPathSegments {
    offset: usize,
    segments: Arc<Vec<PercentDecoded>>,
}

impl RequestPathSegments {
    /// Creates a new RequestPathSegments instance.
    ///
    /// * path: A `Request` uri path that will be split into indivdual segments with
    ///         a leading "/" to represent the root. Empty segments are removed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate gotham;
    /// #
    /// # use gotham::http::request::path::RequestPathSegments;
    /// #
    /// # pub fn main() {
    ///     let srp = RequestPathSegments::new("/%61ctiv%61te//workflow");
    ///     assert_eq!("/", srp.segments()[0].val());
    ///     assert_eq!("activate", srp.segments()[1].val());
    ///     assert_eq!("workflow", srp.segments()[2].val());
    /// # }
    /// ```
    pub fn new<'r>(path: &'r str) -> Self {
        let mut segments = vec!["/"];
        segments.extend(
            path.split('/')
                .filter(|s| !EXCLUDED_SEGMENTS.contains(s))
                .collect::<Vec<&'r str>>(),
        );

        let segments = Arc::new(
            segments
                .iter()
                .filter_map(|s| PercentDecoded::new(s))
                .collect::<Vec<PercentDecoded>>(),
        );

        RequestPathSegments {
            offset: 0,
            segments,
        }
    }

    /// Provide segments that still need to be processed.
    ///
    /// This will always include a "/" node to represent the root as well as all segments
    /// that remain as of the current offset.
    ///
    /// The offset starts at 0 meaning all segments of the initial Request path will be provided
    /// until the offset is updated.
    ///
    /// ```rust
    /// # extern crate gotham;
    /// #
    /// # use gotham::http::request::path::RequestPathSegments;
    /// #
    /// # pub fn main() {
    ///     let mut srp = RequestPathSegments::new("/activate/workflow");
    ///     assert_eq!("/", srp.segments()[0].val());
    ///     assert_eq!("activate", srp.segments()[1].val());
    ///     assert_eq!("workflow", srp.segments()[2].val());
    ///     srp.increase_offset(1);
    ///     assert_eq!("/", srp.segments()[0].val());
    ///     assert_eq!("workflow", srp.segments()[1].val());
    /// # }
    pub fn segments<'a>(&'a self) -> Vec<&PercentDecoded> {
        self.segments
            .iter()
            .enumerate()
            .filter_map(|(i, v)| {
                if i == 0 || i > self.offset {
                    Some(v)
                } else {
                    None
                }
            })
            .collect::<Vec<&PercentDecoded>>()
    }

    /// Increases the current offset value.
    ///
    /// * add: Indicates how much the offset should be increased by
    pub fn increase_offset(&mut self, add: usize) {
        self.offset += add;
    }

    /// Sets the offset.
    ///
    /// * offset: Sets the offset to the supplied value
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }
}
