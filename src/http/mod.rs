//! Helpers for HTTP related data

pub mod request_path;
pub mod query_string;

use std::borrow::Cow;
use url::percent_encoding::percent_decode;

/// Represents data that has been successfully percent decoded and is valid utf8
pub struct PercentDecoded<'a> {
    val: Cow<'a, str>,
}

impl<'a> PercentDecoded<'a> {
    /// Attempt to decode data that has been provided in a perecent encoded format and ensure that
    /// the result is valid utf8.
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

/// Represents data that has been successfully decoded from a form-urlencoded source and is
/// valid utf8
#[derive(PartialEq, Eq, Hash)]
pub struct FormUrlDecoded<'a> {
    val: Cow<'a, str>,
}

impl<'a> FormUrlDecoded<'a> {
    /// Attempt to decode data that has been provided in www-form-urlencoded format and ensure that
    /// the result is valid utf8.
    ///
    /// On success encapulate resultant data for use by components that expect this transformation
    /// has already occured.
    pub fn new(raw: &'a str) -> Option<Self> {
        match percent_decode(raw.as_bytes()).decode_utf8() {
            Ok(mut val) => {
                if val.contains('+') {
                    val = Cow::Owned(val.to_mut().replace("+", " "));
                }
                Some(FormUrlDecoded { val })
            }
            Err(_) => None,
        }
    }

    /// Provide the decoded data this type encapsulates
    pub fn val(&self) -> &str {
        &self.val
    }
}
