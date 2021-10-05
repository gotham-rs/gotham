//! A collection of useful traits that should always be imported.

pub use crate::handler::{IntoHandlerFuture, IntoResponse, MapHandlerError, MapHandlerErrorFuture};
pub use crate::router::builder::{DefineSingleRoute, DrawRoutes};
pub use crate::state::FromState;
