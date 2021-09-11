//! Defines `Router` functionality which acts on the `Response`

mod extender;
mod finalizer;

pub use extender::*;
pub use finalizer::*;

#[cfg(feature = "derive")]
pub use gotham_derive::StaticResponseExtender;
