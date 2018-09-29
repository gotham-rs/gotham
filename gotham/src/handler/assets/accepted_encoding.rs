//! Defines `AcceptedEncoding` for parsing 'Accept-Encoding' header
//! values in requests, used to determine whether compressed versions
//! of static assets are supported by the client.

use hyper::header::{HeaderMap, ACCEPT_ENCODING};

use std::result;
use std::str::FromStr;

/// An error returned from the `FromStr` implementation
/// for `AcceptedEncoding`
#[derive(Debug)]
pub enum ParseEncodingError {
    InvalidEncoding,
}

/// A value for a single accepted encoding,
/// with an encoding name and quality value.
#[derive(PartialEq, Debug)]
pub struct AcceptedEncoding {
    pub encoding: String,
    pub quality: f32,
}

// Parses a single "accept-encoding" value, with optional quality value
// e.g. "gzip" or  "gzip;q=0.8"
// quality defaults to 1 if not supplied
impl FromStr for AcceptedEncoding {
    type Err = ParseEncodingError;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let mut iter = s.split(";");
        iter.next()
            .map(str::trim)
            .and_then(|encoding_str| {
                let encoding = encoding_str.to_string();
                let quality = iter
                    .next()
                    .and_then(|qval| qval.replace("q=", "").trim().parse::<f32>().ok())
                    .unwrap_or(1f32);
                Some(AcceptedEncoding { encoding, quality })
            }).ok_or(ParseEncodingError::InvalidEncoding)
    }
}

/// Returns an Iterator of encodings accepted by the client sorted by quality,
/// with the preferred encoding first.
/// Multiple encodings can be in single "Accept-Encoding" header value,
/// e.g.
/// Accept-Encoding: deflate, gzip;q=1.0, *;q=0.5
///
/// or in multiple headers,
/// e.g.
/// Accept-Encoding: deflate
/// Accept-Encoding: gzip;q=1.0
/// Accept-Encoding: *;q=0.5
pub fn accepted_encodings(headers: &HeaderMap) -> Vec<AcceptedEncoding> {
    let mut accepted_encodings: Vec<AcceptedEncoding> = headers
        .get_all(ACCEPT_ENCODING)
        .iter()
        .filter_map(|val| val.to_str().ok())
        .flat_map(|val| val.split(","))
        .filter_map(|val| val.parse::<AcceptedEncoding>().ok())
        .collect();

    accepted_encodings.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap());
    accepted_encodings
}

#[cfg(test)]
mod tests {
    use super::{accepted_encodings, AcceptedEncoding};
    use hyper::header::{HeaderMap, ACCEPT_ENCODING};

    #[test]
    fn accepted_encoding_single() {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_ENCODING, "gzip".parse().unwrap());
        let expected = vec![AcceptedEncoding {
            encoding: "gzip".to_string(),
            quality: 1f32,
        }];

        assert_eq!(accepted_encodings(&headers), expected);
    }

    #[test]
    fn accepted_encoding_single_with_quality() {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_ENCODING, "gzip;q=0.8".parse().unwrap());
        let expected = vec![AcceptedEncoding {
            encoding: "gzip".to_string(),
            quality: 0.8f32,
        }];

        assert_eq!(accepted_encodings(&headers), expected);
    }

    #[test]
    fn accepted_encoding_multiple_headers() {
        let mut headers = HeaderMap::new();
        headers.append(ACCEPT_ENCODING, "br;q=0.8".parse().unwrap());
        headers.append(ACCEPT_ENCODING, "gzip".parse().unwrap());
        headers.append(ACCEPT_ENCODING, "*;q=0.5".parse().unwrap());
        let expected = vec![
            AcceptedEncoding {
                encoding: "gzip".to_string(),
                quality: 1.0f32,
            },
            AcceptedEncoding {
                encoding: "br".to_string(),
                quality: 0.8f32,
            },
            AcceptedEncoding {
                encoding: "*".to_string(),
                quality: 0.5f32,
            },
        ];

        assert_eq!(accepted_encodings(&headers), expected);
    }

    #[test]
    fn accepted_encoding_multiple_values() {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_ENCODING, "*;q=0.5, gzip;q=0.9, br".parse().unwrap());
        let expected = vec![
            AcceptedEncoding {
                encoding: "br".to_string(),
                quality: 1.0f32,
            },
            AcceptedEncoding {
                encoding: "gzip".to_string(),
                quality: 0.9f32,
            },
            AcceptedEncoding {
                encoding: "*".to_string(),
                quality: 0.5f32,
            },
        ];

        assert_eq!(accepted_encodings(&headers), expected);
    }
}
