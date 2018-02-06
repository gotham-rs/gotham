//! Extracts request data into type-safe structs using Serde.
//!
//! The request data is extracted by the `Route` implementation when dispatching the request. An
//! application-provided data structure is used to deserialize the data and store it within the
//! request `State` before the request is dispatched to the `Handler`.

mod query_string;
mod path;
pub(crate) mod internal;

pub use self::query_string::*;
pub use self::path::*;
