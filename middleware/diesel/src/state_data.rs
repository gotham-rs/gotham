//! Defines data structure for storage in Gotham State that provides access to the underlying r2d2
//! pool so a connection can be established if required by Middleware or Handlers.

use diesel::Connection;
use gotham::state::{FromState, State};
use r2d2::{Error, Pool, PooledConnection};
use r2d2_diesel::ConnectionManager;

/// Convenience function for usage within 3rd party Middleware and Handlers to obtain a
/// Diesel connection.
///
/// # Panics
/// If a connection can not be provided.
pub fn connection<T>(s: &State) -> PooledConnection<ConnectionManager<T>>
where
    T: Connection + 'static,
{
    Diesel::borrow_from(s)
        .conn()
        .expect("Did not obtain valid Diesel connection from R2D2 pool")
}

/// Convenience function for Middleware and Handlers to obtain a Diesel connection.
pub fn try_connection<T>(s: &State) -> Result<PooledConnection<ConnectionManager<T>>, Error>
where
    T: Connection + 'static,
{
    Diesel::borrow_from(s).conn()
}

/// Provides access to a Diesel connection within an r2d2 pool via Gotham State
#[derive(StateData)]
pub struct Diesel<T>
where
    T: Connection + 'static,
{
    pool: Pool<ConnectionManager<T>>,
}

impl<T> Diesel<T>
where
    T: Connection + 'static,
{
    pub(crate) fn new(pool: Pool<ConnectionManager<T>>) -> Self {
        Diesel { pool }
    }

    /// Provides access to a Diesel connection from our r2d2 backed connection pool.
    pub fn conn(&self) -> Result<PooledConnection<ConnectionManager<T>>, Error> {
        self.pool.get()
    }
}
