//! Defines helper functions for processing the request path

use std::sync::Arc;

use http::PercentDecoded;

const EXCLUDED_SEGMENTS: [&str; 1] = [""];

/// Holder for `Request` URI path segments that have been split into individual segments.
///
/// Used internally by the `Router` when traversing its internal `Tree`.
#[derive(Clone, PartialEq)]
pub struct RequestPathSegments {
    offset: usize,
    segments: Arc<Vec<PercentDecoded>>,
}

impl RequestPathSegments {
    /// Creates a new RequestPathSegments instance by splitting a `Request` URI path.
    ///
    /// Empty segments are skipped when generating the `RequestPathSegments` value, and a leading
    /// `/` segment is added to represent the root (and the beginning of traversal). So, a request
    /// path of `/some/path/to//my/handler` will be split into segments:
    ///
    /// ```plain
    /// ["/", "some", "path", "to", "my", "handler"]
    /// ```
    pub(crate) fn new<'r>(path: &'r str) -> Self {
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
    pub(crate) fn segments<'a>(&'a self) -> Vec<&PercentDecoded> {
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
    pub(crate) fn increase_offset(&mut self, add: usize) {
        self.offset += add;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_path_segments_tests() {
        // Validate the claim made in the doc comment above.
        let rps = RequestPathSegments::new("/some/path/to//my/handler");

        assert_eq!(
            rps.segments.iter().map(|s| s.as_ref()).collect::<Vec<_>>(),
            vec!["/", "some", "path", "to", "my", "handler"]
        );
    }
}
