//! Defines data structure for storage in Gotham State that provides access to the underlying r2d2
//! pool so a connection can be established if required by Middleware or Handlers.

use r2d2::{Error, ManageConnection, Pool, PooledConnection};

use gotham::state::{FromState, State};

/// Convenience function for usage within 3rd party Middleware and Handlers to obtain a
/// pooled connection.
///
/// # Panics
/// If a connection can not be provided.
pub fn connection<T>(s: &State) -> PooledConnection<T>
where
    T: ManageConnection + 'static,
{
    StateConnection::borrow_from(s)
        .conn()
        .expect("Did not obtain valid connection from R2D2 pool")
}

/// Convenience function for usage within 3rd party Middleware and Handlers to obtain a
/// pooled connection.
pub fn try_connection<T>(s: &State) -> Result<PooledConnection<T>, Error>
where
    T: ManageConnection + 'static,
{
    StateConnection::borrow_from(s).conn()
}

/// Provides access to a r2d2 connection within an r2d2 pool via Gotham State
#[derive(StateData)]
pub struct StateConnection<T>
where
    T: ManageConnection + 'static,
{
    pool: Pool<T>,
}

impl<T> StateConnection<T>
where
    T: ManageConnection + 'static,
{
    pub(crate) fn new(pool: Pool<T>) -> Self {
        StateConnection { pool }
    }

    /// Provides access to a r2d2 connection from our r2d2 backed connection pool.
    pub fn conn(&self) -> Result<PooledConnection<T>, Error> {
        self.pool.get()
    }
}
