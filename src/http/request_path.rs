//! Defines functionality for operating on `Request` path values

use std::str::FromStr;
use std::error::Error;
use std::fmt;
use std::str::ParseBoolError;
use std::num::{ParseIntError, ParseFloatError};

use state::State;
use router::tree::SegmentMapping;

/// Derived through the macro of the same name supplied by `gotham-derive` for application defined
/// Structs that will pass `Request` path data to custom `Middleware` and `Handler` implementations.
pub trait RequestPathExtractor {
    /// Populates the struct with data from the `Request` path and adds it to `State`
    fn extract(state: &mut State, segment_mapping: SegmentMapping) -> Result<(), String>;
}

/// A `RequestPathExtractor` that does not extract/store any data from the `Request` path.
///
/// Useful in purely static routes and within documentation.
pub struct NoopRequestPathExtractor;
impl RequestPathExtractor for NoopRequestPathExtractor {
    fn extract(_state: &mut State, _segment_mapping: SegmentMapping) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Debug)]
/// Represents an error in coverting a segment(s) from a `Request` path into a type safe
/// value.
///
/// Deliberately kept generic as implementations of FromRequestPath cannot be known in advance.
pub struct FromRequestPathError {
    description: String,
}

impl fmt::Display for FromRequestPathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error decoding request path value: {}", self.description)
    }
}

impl Error for FromRequestPathError {
    fn description(&self) -> &str {
        &self.description
    }
}

/// Converts string data received as part of a `Request` path to type safe values for usage by
/// `Middleware` and `Handlers`.
pub trait FromRequestPath {
    /// Converts a `1..n` `Request` path segments into type safe values.
    ///
    /// # Panic
    /// If the input data is not of the expected format or size a panic will occur.
    ///
    /// e.g. Multiple segments due to usage of a Glob are provided for a value that should
    /// only be generated from a single segment, such as a `u8`.
    fn from_request_path(&Vec<String>) -> Result<Self, FromRequestPathError> where Self: Sized;
}

impl<T> FromRequestPath for Option<T>
    where T: FromRequestPath
{
    fn from_request_path(segments: &Vec<String>) -> Result<Self, FromRequestPathError> {
        if segments.len() == 0 {
            Ok(None)
        } else {
            match T::from_request_path(segments) {
                Ok(v) => Ok(Some(v)),
                Err(v) => Err(v),
            }
        }
    }
}

impl From<ParseIntError> for FromRequestPathError {
    fn from(err: ParseIntError) -> FromRequestPathError {
        FromRequestPathError { description: err.description().to_string() }
    }
}

impl From<ParseFloatError> for FromRequestPathError {
    fn from(err: ParseFloatError) -> FromRequestPathError {
        FromRequestPathError { description: err.description().to_string() }
    }
}

impl From<ParseBoolError> for FromRequestPathError {
    fn from(err: ParseBoolError) -> FromRequestPathError {
        FromRequestPathError { description: err.description().to_string() }
    }
}

macro_rules! frp {
    ($($t:ident),*) => { $(
        impl FromRequestPath for $t {
            fn from_request_path(segments: &Vec<String>) -> Result<Self, FromRequestPathError> {
                if segments.len() == 1 {
                    Ok($t::from_str(segments[0].as_str())?)
                } else {
                    Err(FromRequestPathError {
                        description: String::from("Invalid number of segments")
                    })
                }
            }
        }
    )+ }
}

frp!(bool,
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
     u64);

impl FromRequestPath for String {
    fn from_request_path(segments: &Vec<String>) -> Result<Self, FromRequestPathError> {
        if segments.len() == 1 {
            Ok(segments[0].clone())
        } else {
            Err(FromRequestPathError { description: String::from("Invalid number of segments") })
        }
    }
}
