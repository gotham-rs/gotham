//! Extracts request data into type-safe structs using Serde.
//!
//! Extractors are added to route definitions when defining a `Router`. The `PathExtractor` and
//! `QueryStringExtractor` traits provide usage examples.
//!
//! The request data is extracted by the `Route` implementation when dispatching the request. The
//! application-provided data structure which implements the extractor trait is used to deserialize
//! the data and store it within the request `State` before the request is dispatched to the
//! `Handler`.

pub(crate) mod internal;
mod path;
mod query_string;

pub use self::path::*;
pub use self::query_string::*;
