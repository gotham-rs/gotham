//! Define the X-XSS-Protection header.

use std::fmt;

use hyper;
use hyper::header::{Header, Raw, Formatter, parsing};

static NAME: &'static str = "X-XSS-Protection";

/// The HTTP X-XSS-Protection response header is a feature of Internet Explorer, Chrome and Safari
/// that stops pages from loading when they detect reflected cross-site scripting (XSS) attacks.
///
/// Not as important for modern browsers but useful to set none the less.
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
/// use gotham::http::header::XXxsProtection;
///
/// # fn main () {
/// let mut headers = Headers::new();
/// headers.set(XXxsProtection::EnableBlock);
/// # }
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum XXxsProtection {
    /// Disables XSS filtering.
    Disable,

    /// Enables XSS filtering.
    ///
    /// If a cross-site scripting attack is detected, the browser will sanitize the page.
    Enable,

    /// Enables XSS filtering.
    ///
    /// If a cross-site scripting attack is detected, the browser will sanitize the page.
    ///
    /// `mode=block`: Rather than sanitizing the page, the browser will prevent rendering of the
    ///               page if an attack is detected.
    EnableBlock,
}

impl Header for XXxsProtection {
    fn header_name() -> &'static str {
        NAME
    }

    fn parse_header(raw: &Raw) -> hyper::error::Result<XXxsProtection> {
        let value: String = parsing::from_one_raw_str(raw)?;
        match value.as_str() {
            "0" => return Ok(XXxsProtection::Disable),
            "1" => return Ok(XXxsProtection::Enable),
            "1; mode=block" => return Ok(XXxsProtection::EnableBlock),
            _ => (),
        };

        Err(hyper::error::Error::Header)
    }

    fn fmt_header(&self, f: &mut Formatter) -> fmt::Result {
        f.fmt_line(self)
    }
}

impl fmt::Display for XXxsProtection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::XXxsProtection::*;
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
        assert_eq!(format!("{}", XXxsProtection::Disable), "0");
        assert_eq!(format!("{}", XXxsProtection::Enable), "1");
        assert_eq!(format!("{}", XXxsProtection::EnableBlock), "1; mode=block");
    }

    #[test]
    fn parse_disable() {
        let a: XXxsProtection = Header::parse_header(&"0".into()).unwrap();
        let b = XXxsProtection::Disable;
        assert_eq!(a, b);
    }

    #[test]
    fn parse_enable() {
        let a: XXxsProtection = Header::parse_header(&"1".into()).unwrap();
        let b = XXxsProtection::Enable;
        assert_eq!(a, b);

        let a: XXxsProtection = Header::parse_header(&"1; mode=block".into()).unwrap();
        let b = XXxsProtection::EnableBlock;
        assert_eq!(a, b);
    }

    #[test]
    fn parse_fails() {
        let e: hyper::error::Result<XXxsProtection> = Header::parse_header(&"foobar".into());
        assert!(e.is_err());
    }
}
