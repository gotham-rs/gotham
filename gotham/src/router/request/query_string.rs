//! Extracts query string parameters into type safe structs

use std;

use std::error::Error;
use std::str::FromStr;
use std::string::ParseError;
use std::str::ParseBoolError;
use std::num::{ParseFloatError, ParseIntError};

use hyper::Response;

use state::State;
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
pub trait QueryStringExtractor: StaticResponseExtender {
    /// Populates the struct with data from the `Request` query string and adds it to `State`
    fn extract(state: &mut State) -> Result<(), String>;
}

/// A `QueryStringExtractor` that does not extract/store any data.
///
/// Useful in purely static routes and within documentation.
#[derive(Debug)]
pub struct NoopQueryStringExtractor;
impl QueryStringExtractor for NoopQueryStringExtractor {
    fn extract(_state: &mut State) -> Result<(), String> {
        Ok(())
    }
}

impl StaticResponseExtender for NoopQueryStringExtractor {
    fn extend(_state: &mut State, _res: &mut Response) {}
}

#[derive(Debug)]
/// Represents an error in coverting a key=value pair from a `Request` query string into a
/// type safe value.
pub struct FromQueryStringError {
    description: String,
}

impl std::fmt::Display for FromQueryStringError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error decoding query string: {}", self.description)
    }
}

impl Error for FromQueryStringError {
    fn description(&self) -> &str {
        &self.description
    }
}

/// Converts string data received as part of a `Request` query string to type safe values for
/// usage by `Middleware` and `Handlers`.
pub trait FromQueryString {
    /// Converts a key=value pair from `Request` query string into a type safe value.
    ///
    /// # Panic
    /// If the input data is not of the expected format or size a panic will occur.
    fn from_query_string(&str, &[FormUrlDecoded]) -> Result<Self, FromQueryStringError>
    where
        Self: Sized;
}

impl<T> FromQueryString for Option<T>
where
    T: FromQueryString,
{
    fn from_query_string(
        key: &str,
        values: &[FormUrlDecoded],
    ) -> Result<Self, FromQueryStringError> {
        if values.len() == 0 {
            Ok(None)
        } else {
            match T::from_query_string(key, values) {
                Ok(v) => Ok(Some(v)),
                Err(v) => Err(v),
            }
        }
    }
}

impl<T> FromQueryString for Vec<T>
where
    T: FromQueryString,
{
    fn from_query_string(
        key: &str,
        values: &[FormUrlDecoded],
    ) -> Result<Self, FromQueryStringError> {
        values
            .windows(1)
            .map(|value| T::from_query_string(key, value))
            .collect()
    }
}

impl From<ParseIntError> for FromQueryStringError {
    fn from(err: ParseIntError) -> FromQueryStringError {
        FromQueryStringError {
            description: err.description().to_string(),
        }
    }
}

impl From<ParseFloatError> for FromQueryStringError {
    fn from(err: ParseFloatError) -> FromQueryStringError {
        FromQueryStringError {
            description: err.description().to_string(),
        }
    }
}

impl From<ParseBoolError> for FromQueryStringError {
    fn from(err: ParseBoolError) -> FromQueryStringError {
        FromQueryStringError {
            description: err.description().to_string(),
        }
    }
}

impl From<ParseError> for FromQueryStringError {
    fn from(err: ParseError) -> FromQueryStringError {
        FromQueryStringError {
            description: err.description().to_string(),
        }
    }
}

macro_rules! fstr {
    ($($t:ident),*) => { $(
        impl FromQueryString for $t {
            fn from_query_string(_key: &str, values: &[FormUrlDecoded])
                -> Result<Self, FromQueryStringError> {
                if values.len() == 1 {
                    Ok($t::from_str(values[0].val())?)
                } else {
                    Err(FromQueryStringError {
                            description: String::from("Invalid number of values")
                    })
                }
            }
        }
    )+}
}

fstr!(
    String,
    bool,
    f32,
    f64,
    isize,
    i8,
    i16,
    i32,
    i64,
    usize,
    u8,
    u16,
    u32,
    u64
);
