use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process;

use futures::{future, Future};

use gotham::handler::HandlerFuture;
use gotham::middleware::{Middleware, NewMiddleware};
use gotham::state::{request_id, State};

use diesel::Connection;
use r2d2::{self, Pool};
use r2d2_diesel::ConnectionManager;

use state_data::Diesel;

/// A Gotham compatible Middleware that manages a pool of Diesel connections via r2d2 and hands
/// out connections to other Middleware and Handlers that require them via the Gotham `State`
/// mechanism.
pub struct DieselMiddleware<T>
where
    T: Connection + 'static,
{
    pool: AssertUnwindSafe<r2d2::Pool<ConnectionManager<T>>>,
}

/// Instance created by DieselMiddleware for each request that implements
/// the actual logic of the middleware.
pub struct DieselMiddlewareImpl<T>
where
    T: Connection + 'static,
{
    pool: r2d2::Pool<ConnectionManager<T>>,
}

impl<T> DieselMiddleware<T>
where
    T: Connection,
{
    /// Sets up a new instance of the middleware and establishes a connection to the database.
    ///
    /// * The database to connect to, including authentication components.
    ///
    /// # Panics
    /// If the database identified in `database_url` cannot be connected to at application start.
    ///
    /// n.b. connection will be re-established if the database goes away and returns mid execution
    /// without panic.
    pub fn new(database_url: &str) -> Self {
        let manager = ConnectionManager::<T>::new(database_url);

        let pool = Pool::<ConnectionManager<T>>::new(manager).expect("Failed to create pool.");

        DieselMiddleware::with_pool(pool)
    }

    /// Sets up a new instance of the middleware and establishes a connection to the database.
    ///
    /// * The connection pool (with custom configuration)
    ///
    /// n.b. connection will be re-established if the database goes away and returns mid execution
    /// without panic.
    pub fn with_pool(pool: Pool<ConnectionManager<T>>) -> Self {
        DieselMiddleware {
            pool: AssertUnwindSafe(pool),
        }
    }
}

impl<T> NewMiddleware for DieselMiddleware<T>
where
    T: Connection + 'static,
{
    type Instance = DieselMiddlewareImpl<T>;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        match catch_unwind(|| self.pool.clone()) {
            Ok(pool) => Ok(DieselMiddlewareImpl { pool }),
            Err(_) => {
                error!(
                    "PANIC: r2d2::Pool::clone caused a panic, unable to rescue with a HTTP error"
                );
                eprintln!(
                    "PANIC: r2d2::Pool::clone caused a panic, unable to rescue with a HTTP error"
                );
                process::abort()
            }
        }
    }
}

impl<T> Clone for DieselMiddleware<T>
where
    T: Connection + 'static,
{
    fn clone(&self) -> Self {
        match catch_unwind(|| self.pool.clone()) {
            Ok(pool) => DieselMiddleware {
                pool: AssertUnwindSafe(pool),
            },
            Err(_) => {
                error!("PANIC: r2d2::Pool::clone caused a panic");
                eprintln!("PANIC: r2d2::Pool::clone caused a panic");
                process::abort()
            }
        }
    }
}

impl<T> Middleware for DieselMiddlewareImpl<T>
where
    T: Connection + 'static,
{
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        trace!("[{}] pre chain", request_id(&state));
        state.put(Diesel::<T>::new(self.pool));

        let f = chain(state).and_then(move |(state, response)| {
            {
                trace!("[{}] post chain", request_id(&state));
            }
            future::ok((state, response))
        });
        Box::new(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use diesel::sqlite::SqliteConnection;
    use r2d2_diesel::ConnectionManager;

    static DATABASE_URL: &'static str = ":memory:";

    #[test]
    fn new_with_default_config() {
        let manager = ConnectionManager::new(DATABASE_URL);
        let pool = Pool::<ConnectionManager<SqliteConnection>>::new(manager).unwrap();
        let _middleware = DieselMiddleware::with_pool(pool);
    }

    #[test]
    fn new_with_custom_pool_config() {
        let manager = ConnectionManager::new(DATABASE_URL);
        let pool = Pool::<ConnectionManager<SqliteConnection>>::builder()
            .min_idle(Some(1))
            .build(manager)
            .unwrap();
        let _middleware = DieselMiddleware::with_pool(pool);
    }
}
