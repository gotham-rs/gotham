//! Defines data structure for storage in Gotham State that provides access to the underlying r2d2
//! pool so a connection can be established if required by Middleware or Handlers.

use diesel::Connection;
use r2d2::{GetTimeout, Pool, PooledConnection};
use r2d2_diesel::ConnectionManager;

use gotham;
use gotham::state::{State, FromState};

/// Convenience function for usage within 3rd party Middleware and Handlers to obtain a
/// Diesel connection.
///
// Really just addresses the need to include all the relevant types in 3rd
// party apps but it made working with this middleware a lot nicer.
pub fn conn<T>(s: &State) -> Result<PooledConnection<ConnectionManager<T>>, GetTimeout>
where
    T: Connection + 'static,
{
    Diesel::borrow_from(s).conn()
}

/// Provides access to a Diesel connection within an r2d2 pool via Gotham State
/// for Middleware and Handlers.
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
    pub fn conn(&self) -> Result<PooledConnection<ConnectionManager<T>>, GetTimeout> {
        self.pool.get()
    }
}
