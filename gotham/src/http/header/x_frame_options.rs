//! Define the X-Frame-Options header.

use std::fmt;
use std::str::FromStr;
// TODO: Remove when this import isn't required in stable anymore.
#[allow(unused_imports)]
use std::ascii::AsciiExt;

use hyper;
use hyper::Uri;
use hyper::header::{Formatter, Header, Raw};

use http::header::from_one_rws_delimited_raw_str;

static NAME: &'static str = "X-Frame-Options";

/// The X-Frame-Options header as defined as part of [RFC
/// 7034](https://tools.ietf.org/html/rfc7034).
///
/// The X-Frame-Options HTTP header field, which declares a policy, communicated from the
/// server to the client browser, regarding whether the browser may display the transmitted
/// content in frames that are part of other web pages.
///
/// # ABNF
/// ```plain
/// X-Frame-Options: "DENY"
///                  / "SAMEORIGIN"
///                  / ( "ALLOW-FROM" RWS SERIALIZED-ORIGIN )
///
/// RWS: 1*( SP / HTAB )
///      ; required whitespace
///
/// SERIALIZED-ORIGIN   = scheme "://" host [ ":" port ]
///                       ; <scheme>, <host>, <port> from RFC 3986
/// ```
///
/// # Example values
/// * `ALLOW-FROM https://example.com`
///
/// # Example
/// ```
/// # extern crate hyper;
/// # extern crate gotham;
///
/// use hyper::header::Headers;
/// use gotham::http::header::XFrameOptions;
///
/// # fn main () {
/// let mut headers = Headers::new();
/// headers.set(XFrameOptions::Deny);
/// # }
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum XFrameOptions {
    /// A browser receiving content with this header field MUST NOT display this content in any
    /// frame.
    Deny,

    /// A browser receiving content with this header field MUST NOT display this content in any
    /// frame from a page of different origin than the content itself.
    SameOrigin,

    /// A browser receiving content with this header MUST NOT display this content in a frame from
    /// any page with a top-level browsing context of different origin than the specified origin.
    AllowFrom(String),
}

impl Header for XFrameOptions {
    fn header_name() -> &'static str {
        NAME
    }

    fn parse_header(raw: &Raw) -> hyper::error::Result<XFrameOptions> {
        let mut values: Vec<String> = from_one_rws_delimited_raw_str(raw)?;
        let origin = if values.len() == 2 {
            let uri = values.remove(1);
            match Uri::from_str(&uri) {
                Ok(_) => Some(uri),
                Err(_) => None,
            }
        } else {
            None
        };

        match values.first() {
            Some(fo) => match fo.to_ascii_uppercase().as_str() {
                "DENY" => Ok(XFrameOptions::Deny),
                "SAMEORIGIN" => Ok(XFrameOptions::SameOrigin),
                "ALLOW-FROM" if origin.is_some() => Ok(XFrameOptions::AllowFrom(origin.unwrap())),
                _ => Err(hyper::error::Error::Header),
            },
            None => Err(hyper::error::Error::Header),
        }
    }

    fn fmt_header(&self, f: &mut Formatter) -> fmt::Result {
        f.fmt_line(self)
    }
}

impl fmt::Display for XFrameOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::XFrameOptions::*;
        match *self {
            Deny => f.write_str("DENY"),
            SameOrigin => f.write_str("SAMEORIGIN"),
            AllowFrom(ref url) => f.write_fmt(format_args!("ALLOW-FROM {}", url)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_display_formatting() {
        assert_eq!(format!("{}", XFrameOptions::Deny), "DENY");
        assert_eq!(format!("{}", XFrameOptions::SameOrigin), "SAMEORIGIN");
        assert_eq!(
            format!(
                "{}",
                XFrameOptions::AllowFrom(String::from("https://example.com"))
            ),
            "ALLOW-FROM https://example.com"
        );
    }

    #[test]
    fn parse_deny() {
        let a: XFrameOptions = Header::parse_header(&"DENY".into()).unwrap();
        let b = XFrameOptions::Deny;
        assert_eq!(a, b);
    }

    #[test]
    fn parse_same_origin() {
        let a: XFrameOptions = Header::parse_header(&"sameorigin".into()).unwrap();
        let b = XFrameOptions::SameOrigin;
        assert_eq!(a, b);
    }

    #[test]
    fn parse_allow_from() {
        let a: XFrameOptions =
            Header::parse_header(&"allow-FROM https://example.com".into()).unwrap();
        let b = XFrameOptions::AllowFrom(String::from("https://example.com"));
        assert_eq!(a, b);

        let e: hyper::error::Result<XFrameOptions> =
            Header::parse_header(&"allow-FROM https://".into());
        assert!(e.is_err());

        let e: hyper::error::Result<XFrameOptions> = Header::parse_header(&"allow-FROM".into());
        assert!(e.is_err());
    }

    #[test]
    fn parse_fails() {
        let e: hyper::error::Result<XFrameOptions> = Header::parse_header(&"foobar".into());
        assert!(e.is_err());
    }

}
