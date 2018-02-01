//! Helpers for HTTP Request handling and Response generation

pub mod request;
pub mod response;
pub mod header;

use std;
use url::percent_encoding::percent_decode;

/// Represents data that has been successfully percent decoded and is valid utf8
#[derive(Clone, PartialEq, Debug)]
pub struct PercentDecoded {
    val: String,
}

impl PercentDecoded {
    /// Attempt to decode data that has been provided in a perecent encoded format and ensure that
    /// the result is valid utf8.
    ///
    /// On success encapulate resultant data for use by components that expect this transformation
    /// has already occured.
    pub fn new(raw: &str) -> Option<Self> {
        match percent_decode(raw.as_bytes()).decode_utf8() {
            Ok(pd) => {
                trace!(" percent_decode: {}, src: {}", pd, raw);
                Some(PercentDecoded {
                    val: pd.into_owned(),
                })
            }
            Err(_) => {
                trace!(" percent_decode: error, src: {}", raw);
                None
            }
        }
    }

    /// Provide the decoded data this type encapsulates
    pub fn val(&self) -> &str {
        &self.val
    }
}

impl AsRef<str> for PercentDecoded {
    fn as_ref(&self) -> &str {
        &self.val
    }
}

/// Decode form-urlencoded strings
pub fn form_url_decode(raw: &str) -> Result<String, std::str::Utf8Error> {
    match percent_decode(raw.replace("+", " ").as_bytes()).decode_utf8() {
        Ok(pd) => {
            trace!(" form_url_decode: {}, src: {}", pd, raw);
            Ok(pd.into_owned())
        }
        Err(e) => {
            trace!(" form_url_decode: error, src: {}", raw);
            Err(e)
        }
    }
}

/// Represents data that has been successfully decoded from a form-urlencoded source and is
/// valid utf8
#[derive(PartialEq, Eq, Hash, Debug)]
pub struct FormUrlDecoded {
    val: String,
}

impl FormUrlDecoded {
    /// Attempt to decode data that has been provided in www-form-urlencoded format and ensure that
    /// the result is valid utf8.
    ///
    /// On success encapulate resultant data for use by components that expect this transformation
    /// has already occured.
    pub fn new(raw: &str) -> Option<Self> {
        match form_url_decode(raw) {
            Ok(val) => Some(FormUrlDecoded { val }),
            Err(_) => None,
        }
    }

    /// Provide the decoded data this type encapsulates
    pub fn val(&self) -> &str {
        &self.val
    }
}

impl AsRef<str> for FormUrlDecoded {
    fn as_ref(&self) -> &str {
        &self.val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_valid_percent_decode() {
        let pd = PercentDecoded::new("%41+%42%2B%63%20%64").unwrap();
        assert_eq!("A+B+c d", pd.val());
    }

    #[test]
    fn ensure_valid_www_form_url_encoded_value() {
        let f = FormUrlDecoded::new("%41+%42%2B%63%20%64").unwrap();
        assert_eq!("A B+c d", f.val());
    }
}
