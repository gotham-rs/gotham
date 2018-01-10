//! Define the X-XSS-Protection header.

use std::fmt;

use hyper;
use hyper::header::{parsing, Formatter, Header, Raw};

static NAME: &'static str = "X-XSS-Protection";

/// The HTTP X-XSS-Protection response header is a feature of Internet Explorer, Chrome and Safari
/// that stops pages from loading when they detect reflected cross-site scripting (XSS) attacks.
///
/// Not as important for modern browsers but useful to set nonetheless.
///
/// No formal specification/RFC exists for this header.
///
/// # Example values
/// * `1; mode=block`
///
/// # Example
/// ```
/// # extern crate hyper;
/// # extern crate gotham;
///
/// use hyper::header::Headers;
/// use gotham::http::header::XXssProtection;
///
/// # fn main () {
/// let mut headers = Headers::new();
/// headers.set(XXssProtection::EnableBlock);
/// # }
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum XXssProtection {
    /// Disables XSS filtering.
    Disable,

    /// Enables XSS filtering.
    ///
    /// If a cross-site scripting attack is detected, the browser will sanitize the page.
    Enable,

    /// Enables XSS filtering.
    ///
    /// Rather than sanitizing, the browser will prevent rendering of the page if an attack
    /// is detected.
    ///
    /// Adds `mode=block` to the header value.
    EnableBlock,
}

impl Header for XXssProtection {
    fn header_name() -> &'static str {
        NAME
    }

    fn parse_header(raw: &Raw) -> hyper::error::Result<XXssProtection> {
        let value: String = parsing::from_one_raw_str(raw)?;
        match value.as_str() {
            "0" => Ok(XXssProtection::Disable),
            "1" => Ok(XXssProtection::Enable),
            "1; mode=block" => Ok(XXssProtection::EnableBlock),
            _ => Err(hyper::error::Error::Header),
        }
    }

    fn fmt_header(&self, f: &mut Formatter) -> fmt::Result {
        f.fmt_line(self)
    }
}

impl fmt::Display for XXssProtection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::XXssProtection::*;
        match *self {
            Disable => f.write_str("0"),
            Enable => f.write_str("1"),
            EnableBlock => f.write_str("1; mode=block"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_display_formatting() {
        assert_eq!(format!("{}", XXssProtection::Disable), "0");
        assert_eq!(format!("{}", XXssProtection::Enable), "1");
        assert_eq!(format!("{}", XXssProtection::EnableBlock), "1; mode=block");
    }

    #[test]
    fn parse_disable() {
        let a: XXssProtection = Header::parse_header(&"0".into()).unwrap();
        let b = XXssProtection::Disable;
        assert_eq!(a, b);
    }

    #[test]
    fn parse_enable() {
        let a: XXssProtection = Header::parse_header(&"1".into()).unwrap();
        let b = XXssProtection::Enable;
        assert_eq!(a, b);

        let a: XXssProtection = Header::parse_header(&"1; mode=block".into()).unwrap();
        let b = XXssProtection::EnableBlock;
        assert_eq!(a, b);
    }

    #[test]
    fn parse_fails() {
        let e: hyper::error::Result<XXssProtection> = Header::parse_header(&"foobar".into());
        assert!(e.is_err());
    }
}
