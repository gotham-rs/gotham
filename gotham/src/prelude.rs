//! A collection of useful traits and macros that should always be imported.

#[cfg(feature = "derive")]
pub use gotham_derive::*;

pub use crate::handler::{IntoHandlerFuture, IntoResponse, MapHandlerError, MapHandlerErrorFuture};
pub use crate::router::builder::{DefineSingleRoute, DrawRoutes};
pub use crate::state::FromState;
