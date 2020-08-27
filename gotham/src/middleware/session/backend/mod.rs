pub(super) mod memory;

use std::panic::RefUnwindSafe;
use std::pin::Pin;

use futures::prelude::*;

use crate::middleware::session::{SessionError, SessionIdentifier};

/// A type which is used to spawn new `Backend` values.
pub trait NewBackend: Sync + Clone + RefUnwindSafe {
    /// The type of `Backend` created by the `NewBackend`.
    type Instance: Backend + Send + 'static;

    /// Create and return a new `Backend` value.
    fn new_backend(&self) -> anyhow::Result<Self::Instance>;
}

/// Type alias for the trait objects returned by `Backend`.
pub type SessionFuture = dyn Future<Output = Result<Option<Vec<u8>>, SessionError>> + Send;

/// A `Backend` receives session data and stores it, and recalls the session data subsequently.
///
/// All session data is serialized into a `Vec<u8>` which is treated as opaque by the backend. The
/// serialization format is subject to change and must not be relied upon by the `Backend`.
pub trait Backend: Send {
    /// Persists a session, either creating a new session or updating an existing session.
    fn persist_session(
        &self,
        identifier: SessionIdentifier,
        content: &[u8],
    ) -> Result<(), SessionError>;

    /// Retrieves a session from the underlying storage.
    ///
    /// The returned future will resolve to an `Option<Vec<u8>>` on success, where a value of
    /// `None` indicates that the session is not available for use and a new session should be
    /// established.
    fn read_session(&self, identifier: SessionIdentifier) -> Pin<Box<SessionFuture>>;

    /// Drops a session from the underlying storage.
    fn drop_session(&self, identifier: SessionIdentifier) -> Result<(), SessionError>;
}
