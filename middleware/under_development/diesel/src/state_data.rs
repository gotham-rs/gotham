//! Defines data structure for storage in Gotham State that provides access to the underlying r2d2
//! pool so a connection can be established if required by Middleware or Handlers.

use diesel::Connection;
use r2d2::{Error, Pool, PooledConnection};
use r2d2_diesel::ConnectionManager;

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

impl<T> Clone for Diesel<T>
where
    T: Connection + 'static,
{
    fn clone(&self) -> Diesel<T> {
        Diesel::new(self.pool.clone())
    }
}
