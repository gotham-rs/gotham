//! Define the X-Content-Type-Options header.

use std::fmt;
// TODO: Remove when this import isn't required in stable anymore.
#[allow(unused_imports)]
use std::ascii::AsciiExt;

use hyper;
use hyper::header::{parsing, Formatter, Header, Raw};

static NAME: &'static str = "X-Content-Type-Options";

/// The `X-Content-Type-Options` response header can be used to require checking of a response’s
/// `Content-Type` header against the type of a request.
///
/// # ABNF
///
/// X-Content-Type-Options: "nosniff"
///                         ; case-insensitive
///
/// # Examples values
/// * `nosniff`
///
/// # Example
/// ```
/// # extern crate hyper;
/// # extern crate gotham;
/// #
/// # fn main () {
/// use hyper::header::Headers;
/// use gotham::http::header::XContentTypeOptions;
///
/// let mut headers = Headers::new();
/// headers.set(XContentTypeOptions::NoSniff);
/// # }
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum XContentTypeOptions {
    /// Require checking of a response's `Content-Type` header.
    NoSniff,
}

impl Header for XContentTypeOptions {
    fn header_name() -> &'static str {
        NAME
    }

    fn parse_header(raw: &Raw) -> hyper::error::Result<XContentTypeOptions> {
        let value: String = parsing::from_one_raw_str(raw)?;
        match value.to_ascii_lowercase().as_str() {
            "nosniff" => Ok(XContentTypeOptions::NoSniff),
            _ => Err(hyper::error::Error::Header),
        }
    }

    fn fmt_header(&self, f: &mut Formatter) -> fmt::Result {
        f.fmt_line(self)
    }
}

impl fmt::Display for XContentTypeOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::XContentTypeOptions::*;
        match *self {
            NoSniff => f.write_str("nosniff"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_display_formatting() {
        assert_eq!(format!("{}", XContentTypeOptions::NoSniff), "nosniff");
    }

    #[test]
    fn parse_nosniff() {
        let a: XContentTypeOptions = Header::parse_header(&"nosniff".into()).unwrap();
        let b = XContentTypeOptions::NoSniff;
        assert_eq!(a, b);
    }

    #[test]
    fn parse_fails() {
        let e: hyper::error::Result<XContentTypeOptions> = Header::parse_header(&"foobar".into());
        assert!(e.is_err());
    }
}
