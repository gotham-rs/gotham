//! Defines the wrapping type for a segment-matching regex.

use regex::Regex;

use std::cmp::Ordering;
use std::panic::AssertUnwindSafe;

/// A unwind-safe wrapper for Regex that implements PartialEq, Eq, PartialOrd, and Ord.  These
/// traits are implemented in a potentially error-prone way by comparing the underlying &str
/// representations of the regular expression.
///
/// If the `ConstrainedSegmentRegex::is_match` traps a panic from `Regex::is_match`,
/// `std::process::abort()` will be called and the program will terminate.
pub struct ConstrainedSegmentRegex {
    regex: AssertUnwindSafe<Regex>,
}

impl ConstrainedSegmentRegex {
    /// Creates a new ConstrainedSegmentRegex from a provided string.
    ///
    /// It wraps the string in begin and end of line anchors to prevent it from matching more than
    /// intended.
    pub fn new(regex: &str) -> Self {
        ConstrainedSegmentRegex {
            regex: AssertUnwindSafe(Regex::new(&format!("^{}$", regex)).unwrap()),
        }
    }

    /// Returns the pattern backing this regex as a `&str`.
    #[inline]
    pub(crate) fn as_str(&self) -> &str {
        self.regex.as_str()
    }

    /// Wraps `regex::Regex::is_match` to return true if and only if the regex matches the string
    /// given.
    #[inline]
    pub(crate) fn is_match(&self, s: &str) -> bool {
        self.regex.is_match(s)
    }
}

impl PartialEq for ConstrainedSegmentRegex {
    fn eq(&self, other: &Self) -> bool {
        self.regex.as_str() == other.regex.as_str()
    }
}

impl Eq for ConstrainedSegmentRegex {}

impl PartialOrd for ConstrainedSegmentRegex {
    fn partial_cmp(&self, other: &ConstrainedSegmentRegex) -> Option<Ordering> {
        Some(self.as_str().cmp(other.as_str()))
    }
}

impl Ord for ConstrainedSegmentRegex {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Clone for ConstrainedSegmentRegex {
    fn clone(&self) -> ConstrainedSegmentRegex {
        ConstrainedSegmentRegex {
            regex: AssertUnwindSafe(self.regex.0.clone()),
        }
    }
}
