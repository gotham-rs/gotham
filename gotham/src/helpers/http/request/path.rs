//! Defines helper functions for processing the request path

use helpers::http::PercentDecoded;

const EXCLUDED_SEGMENTS: [&str; 1] = [""];

/// Holder for `Request` URI path segments that have been split into individual segments.
///
/// Used internally by the `Router` when traversing its internal `Tree`.
#[derive(Clone, Debug, PartialEq)]
pub struct RequestPathSegments {
    segments: Vec<PercentDecoded>,
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
    pub(crate) fn new(path: &str) -> Self {
        let segments = path
            .split('/')
            .filter(|s| !EXCLUDED_SEGMENTS.contains(s))
            .filter_map(PercentDecoded::new)
            .collect();

        RequestPathSegments { segments }
    }

    pub(crate) fn into_subsegments(&self, offset: usize) -> Self {
        RequestPathSegments {
            segments: self.segments.split_at(offset).1.to_vec(),
        }
    }

    /// Provide segments that still need to be processed.
    ///
    /// This will always include a "/" node to represent the root as well as all segments
    /// that remain as of the current offset.
    ///
    /// The offset starts at 0 meaning all segments of the initial Request path will be provided
    /// until the offset is updated.
    pub(crate) fn segments(&self) -> &Vec<PercentDecoded> {
        &self.segments
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
            vec!["some", "path", "to", "my", "handler"]
        );
    }
}
