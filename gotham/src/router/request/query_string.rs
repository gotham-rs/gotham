//! Extracts query string parameters into type safe structs

use std;

use std::error::Error;
use std::str::FromStr;
use std::string::ParseError;
use std::str::ParseBoolError;
use std::num::{ParseFloatError, ParseIntError};

use serde::{Deserialize, Deserializer};
use hyper::Response;

use state::{State, StateData};
use http::FormUrlDecoded;
use router::response::extender::StaticResponseExtender;

/// Extracts the `Request` query string into `State`. On failure is capable of extending `Response`
/// to indicate why the extraction process failed.
///
/// This functionality can be simply derived for application structs via `QueryStringExtractor`,
/// which will attempt to populate the associated struct. Combine with the derive
/// `StaticResponseExtender` to have invalid query string data result in "400 Bad Request".
///
/// Custom responses can be created by using the `QueryStringExtractor` derive and then
/// implementing `StaticResponseExtender` independently.
pub trait QueryStringExtractor
    : for<'de> Deserialize<'de> + StaticResponseExtender + StateData {
}

impl<T> QueryStringExtractor for T
where
    for<'de> T: Deserialize<'de> + StaticResponseExtender + StateData,
{
}

/// A `QueryStringExtractor` that does not extract/store any data.
///
/// Useful in purely static routes and within documentation.
#[derive(Debug)]
pub struct NoopQueryStringExtractor;

// This doesn't get derived correctly if we just `#[derive(Deserialize)]` above, because the
// Deserializer expects to _ignore_ a value, not just do nothing. By filling in the impl ourselves,
// we can explicitly do nothing.
impl<'de> Deserialize<'de> for NoopQueryStringExtractor {
    fn deserialize<D>(_de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(NoopQueryStringExtractor)
    }
}

impl StateData for NoopQueryStringExtractor {}

impl StaticResponseExtender for NoopQueryStringExtractor {
    fn extend(_state: &mut State, _res: &mut Response) {}
}
