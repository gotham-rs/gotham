//! Defines `SegmentType` for `Tree`.
use std::collections::HashMap;

use helpers::http::PercentDecoded;
use router::tree::regex::ConstrainedSegmentRegex;

/// Mapping of segment names into the collection of values for that segment.
pub type SegmentMapping<'r> = HashMap<&'r str, Vec<&'r PercentDecoded>>;

/// Indicates the type of segment which is being represented by this Node.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum SegmentType {
    /// Is matched exactly (string equality) to the segment for incoming request paths.
    ///
    /// Unlike all other `SegmentTypes`, values determined to be associated with this segment
    /// within a `Request` path are **not** stored within `State`.
    Static,

    /// Uses the supplied regex to determine match against incoming request paths.
    Constrained {
        /// Regex used to match against a single segment of a request path.
        regex: ConstrainedSegmentRegex,
    },

    /// Matches any corresponding segment for incoming request paths.
    Dynamic,

    /// Matches multiple path segments until the end of the request path or until a child
    /// segment of the above defined types is found.
    Glob,
}
