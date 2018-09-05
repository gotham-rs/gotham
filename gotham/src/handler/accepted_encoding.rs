use hyper::header::{HeaderMap, ACCEPT_ENCODING};
use std::result;
use std::str::FromStr;

#[derive(Debug, Fail)]
pub enum ParseEncodingError {
    #[fail(display = "Invalid encoding")]
    InvalidEncoding,
}

#[derive(PartialEq, Debug)]
pub struct AcceptedEncoding {
    pub encoding: String,
    pub quality: f32,
}

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
            })
            .ok_or(ParseEncodingError::InvalidEncoding)
    }
}

/// Returns an Iterator of encodings accepted by the client sorted by quality,
/// with the preferred encoding first.
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
