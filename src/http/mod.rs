//! Helpers for HTTP related data

pub mod request_path;

use std::borrow::Cow;
use url::percent_encoding::percent_decode;

const EXCLUDED_SEGMENTS: [&str; 3] = ["", ".", ".."];

/// Spilt a `Request` path into indivdual segments with leading "/" to represent the root.
///
/// Removes any reference to `.` or `..` if supplied.
///
/// # Example
///
/// ```rust
/// # extern crate gotham;
/// #
/// # use gotham::http::split_request_path;
/// #
/// # pub fn main() {
///     let srp = split_request_path("/%61ctiv%61te/../batsignal").unwrap();
///     assert_eq!("/", srp[0].val());
///     assert_eq!("activate", srp[1].val());
///     assert_eq!("batsignal", srp[2].val());
/// # }
/// ```
pub fn split_request_path<'r>(path: &'r str) -> Option<Vec<PercentDecoded>> {
    let mut segments = vec!["/"];
    segments.extend(path.split('/')
                        .filter(|s| !EXCLUDED_SEGMENTS.contains(s))
                        .collect::<Vec<&'r str>>());
    let decoded_segments =
        segments.iter().filter_map(|s| PercentDecoded::new(s)).collect::<Vec<PercentDecoded>>();

    // Ensure that no segment failed to be encoded
    if decoded_segments.len() == segments.len() {
        Some(decoded_segments)
    } else {
        None
    }
}

/// Transport data that has been successfully percent decoded and is valid utf8
pub struct PercentDecoded<'a> {
    val: Cow<'a, str>,
}

impl<'a> PercentDecoded<'a> {
    /// Attempt to percent and utf8 decode the supplied data.
    ///
    /// On success encapulate resultant data for use by components that expect this transformation
    /// has already occured.
    pub fn new(raw: &'a str) -> Option<Self> {
        match percent_decode(raw.as_bytes()).decode_utf8() {
            Ok(val) => Some(PercentDecoded { val }),
            Err(_) => None,
        }
    }

    /// Provide the decoded data this type encapsulates
    pub fn val(&self) -> &str {
        &self.val
    }
}
