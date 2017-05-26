//! Helpers for HTTP related data

pub mod request_path;

use std::borrow::Cow;
use url::percent_encoding::percent_decode;

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
