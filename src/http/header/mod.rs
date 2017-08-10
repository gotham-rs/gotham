//! Defines HTTP headers which are set by Gotham but not provided by Hyper.

mod x_request_id;
mod x_frame_options;
mod x_xss_protection;
mod x_content_type_options;
mod x_runtime_microseconds;

pub use http::header::x_request_id::XRequestId;
pub use http::header::x_frame_options::XFrameOptions;
pub use http::header::x_xss_protection::XXssProtection;
pub use http::header::x_content_type_options::XContentTypeOptions;
pub use http::header::x_runtime_microseconds::XRuntimeMicroseconds;

use std::str;
use hyper;
use hyper::header::Raw;

/// Reads a single, space delimited, raw string into a Vec.
pub fn from_one_rws_delimited_raw_str<T: str::FromStr>(raw: &Raw) -> hyper::error::Result<Vec<T>> {
    if let Some(line) = raw.one() {
        if !line.is_empty() {
            return from_rws_delimited_raw_str(raw);
        }
    }

    Err(hyper::error::Error::Header)
}

/// Reads a space delimited, raw string into a Vec.
pub fn from_rws_delimited_raw_str<T: str::FromStr>(raw: &Raw) -> hyper::error::Result<Vec<T>> {
    let mut result = Vec::new();
    for line in raw {
        let line = try!(str::from_utf8(line.as_ref()));
        result.extend(
            line.split(' ')
                .filter_map(|header| match header.trim() {
                    "" => None,
                    header => Some(header),
                })
                .filter_map(|header| header.trim().parse().ok()),
        )
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_rws_from_one_header() {
        let r: Raw = Raw::from("X Y".as_bytes().to_vec());
        let values: Vec<String> = from_one_rws_delimited_raw_str(&r).unwrap();
        assert_eq!(values, ["X", "Y"]);
    }

    #[test]
    fn invalid_rws_from_one_header() {
        let r: Raw = Raw::from(vec!["Y Z".as_bytes().to_vec(), "X".as_bytes().to_vec()]);
        let values: hyper::error::Result<Vec<String>> = from_one_rws_delimited_raw_str(&r);
        assert!(values.is_err());
    }
}
