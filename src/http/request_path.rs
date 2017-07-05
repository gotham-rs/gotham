//! Defines functionality for operating on `Request` path values

use std::sync::Arc;
use std::str::FromStr;
use std::error::Error;
use std::fmt;
use std::string::ParseError;
use std::str::ParseBoolError;
use std::num::{ParseIntError, ParseFloatError};

use http::PercentDecoded;
use state::{State, StateData};
use router::tree::SegmentMapping;

const EXCLUDED_SEGMENTS: [&str; 1] = [""];

/// Holder for `Request` uri path segments that have been split into individual segments that are
/// suitable for use with `Tree` traversal.
#[derive(Clone, PartialEq)]
pub struct RequestPathSegments {
    offset: usize,
    segments: Arc<Vec<PercentDecoded>>,
}

impl StateData for RequestPathSegments {}

impl RequestPathSegments {
    /// Creates a new RequestPathSegments instance.
    ///
    /// * path: A `Request` uri path that will be split into indivdual segments with
    ///         a leading "/" to represent the root. Empty segments are removed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate gotham;
    /// #
    /// # use gotham::http::request_path::RequestPathSegments;
    /// #
    /// # pub fn main() {
    ///     let srp = RequestPathSegments::new("/%61ctiv%61te//batsignal");
    ///     assert_eq!("/", srp.segments()[0].val());
    ///     assert_eq!("activate", srp.segments()[1].val());
    ///     assert_eq!("batsignal", srp.segments()[2].val());
    /// # }
    /// ```
    pub fn new<'r>(path: &'r str) -> Self {
        let mut segments = vec!["/"];
        segments.extend(path.split('/')
                            .filter(|s| !EXCLUDED_SEGMENTS.contains(s))
                            .collect::<Vec<&'r str>>());

        let segments = Arc::new(segments
                                    .iter()
                                    .filter_map(|s| PercentDecoded::new(s))
                                    .collect::<Vec<PercentDecoded>>());

        RequestPathSegments {
            offset: 0,
            segments,
        }
    }

    /// Provide segments that still need to be processed via a `Router`.
    ///
    /// n.b. When offset from something other than 0, that is for a delegated `Router`, the
    /// `Tree` structure will still (validly) believe it starts from a root segment of "/" as
    /// there is deliberately no knowledge of any other `Router` having been involved in
    /// the `Request`. To facilitiate this we always include "/" and filter anything that has
    /// previously been processed.
    pub fn segments<'a>(&'a self) -> Vec<&PercentDecoded> {
        self.segments
            .iter()
            .enumerate()
            .filter_map(|(i, v)| if i == 0 || i > self.offset {
                            Some(v)
                        } else {
                            None
                        })
            .collect::<Vec<&PercentDecoded>>()
    }

    /// Increases the offset for the original Request path that should be considered the
    /// first node for the next delegated router.
    ///
    /// * add: Indicates how many segments have been consumed by the current router, *including* the
    /// root node, "/". This will be added to any exisiting offset amount.
    pub fn increase_offset(&mut self, add: usize) {
        self.offset += add;
    }
}

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
    fn from_request_path(&[&PercentDecoded]) -> Result<Self, FromRequestPathError> where Self: Sized;
}

impl<T> FromRequestPath for Option<T>
    where T: FromRequestPath
{
    fn from_request_path(segments: &[&PercentDecoded]) -> Result<Self, FromRequestPathError> {
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

impl From<ParseError> for FromRequestPathError {
    fn from(err: ParseError) -> FromRequestPathError {
        FromRequestPathError { description: err.description().to_string() }
    }
}

macro_rules! fstr {
    ($($t:ident),*) => { $(
        impl FromRequestPath for $t {
            fn from_request_path(segments: &[&PercentDecoded]) -> Result<Self, FromRequestPathError> {
                if segments.len() == 1 {
                    Ok($t::from_str(segments[0].val())?)
                } else {
                    Err(FromRequestPathError {
                        description: String::from("Invalid number of segments")
                    })
                }
            }
        }
    )+ }
}

fstr!(String,
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
      u64);
