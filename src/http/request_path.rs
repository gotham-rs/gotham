//! Defines functionality for operating on `Request` path values

use std::str::FromStr;
use std::any::Any;

use state::State;
use router::tree::SegmentMapping;

/// Derived through the macro of the same name supplied by `gotham-derive` for application defined
/// Structs that will pass `Request` path data to custom `Middleware` and `Handler` implementations.
pub trait RequestPathExtractor {
    /// Populates the struct with data from the `Request` path and adds it to `State`
    fn extract(state: &mut State, segment_mapping: SegmentMapping) -> Result<(), Box<Any + Send>>;
}

/// A `RequestPathExtractor` that does not extract/store any data from the `Request` path.
///
/// Useful in purely static routes and within documentation.
pub struct NoopRequestPathExtractor;
impl RequestPathExtractor for NoopRequestPathExtractor {
    fn extract(_state: &mut State,
               _segment_mapping: SegmentMapping)
               -> Result<(), Box<Any + Send>> {
        Ok(())
    }
}

/// Converts string data received as part of a `Request` path to type safe values for usage by
/// `Middleware` and `Handlers`.
pub trait FromRequestPath {
    /// The associated error which can be returned from parsing.
    ///
    /// In many cases this is passed from the corresponding implementation of FromStr for Self.
    type Err;

    /// Converts a `1..n` `Request` path segments into type safe values.
    ///
    /// # Panic
    /// If the input data is not of the expected format or size a panic will occur.
    ///
    /// e.g. Multiple segments due to usage of a Glob are provided for a value that should
    /// only be generated from a single segment, such as a `u8`.
    fn from_request_path(&Vec<String>) -> Result<Self, Self::Err> where Self: Sized;
}

impl<T> FromRequestPath for Option<T>
    where T: FromRequestPath
{
    type Err = T::Err;

    fn from_request_path(segments: &Vec<String>) -> Result<Self, Self::Err> {
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

macro_rules! frp {
    ($($t:ident),*) => { $(
        impl FromRequestPath for $t {
            type Err = <$t as FromStr>::Err;

            fn from_request_path(segments: &Vec<String>) -> Result<Self, Self::Err> {
                if segments.len() == 1 {
                    $t::from_str(segments[0].as_str())
                } else {
                    panic!(format!("Invalid data for request path segment translation"));
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
    type Err = <String as FromStr>::Err;

    fn from_request_path(segments: &Vec<String>) -> Result<Self, Self::Err> {
        if segments.len() == 1 {
            Ok(segments[0].clone())
        } else {
            panic!("Invalid data for request path segment to String conversion");
        }
    }
}
