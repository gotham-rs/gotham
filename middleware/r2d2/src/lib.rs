//! Makes a R2D2 connection available to Middleware and Handlers that are involved in
//! processing a Request.
//!
//! Utilises r2d2 pooling to ensure efficent database usage and prevent resource exhaustion.

#![warn(missing_docs, deprecated)]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
#[macro_use]
extern crate log;
extern crate r2d2;
#[cfg(test)]
extern crate r2d2_redis;
#[cfg(test)]
extern crate r2d2_sqlite;
#[cfg(test)]
extern crate redis;

pub mod state_data;

use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process;

use futures::{future, Future};

use gotham::middleware::{Middleware, NewMiddleware};
use gotham::state::{request_id, State};
use gotham::handler::HandlerFuture;

use r2d2::Pool;
use r2d2::ManageConnection;
use state_data::StateConnection;

/// A Gotham compatible Middleware that manages a pool of r2d2 connections via r2d2 and hands
/// out connections to other Middleware and Handlers that require them via the Gotham `State`
/// mechanism.
/// Theoretically it should support all r2d2 adaptors listed on [https://github.com/sfackler/r2d2](https://github.com/sfackler/r2d2), but support may vary depending on the adaptors implementation.
pub struct R2D2Middleware<T>
where
    T: ManageConnection + 'static,
{
    pool: AssertUnwindSafe<Pool<T>>,
}

/// Instance created by R2D2Middleware for each request that implements
/// the actual logic of the middleware.
pub struct R2D2MiddlewareImpl<T>
where
    T: ManageConnection + 'static,
{
    pool: r2d2::Pool<T>,
}

impl<T> R2D2Middleware<T>
where
    T: ManageConnection,
{
    /// Sets up a new instance of the middleware and establishes a connection to the database.
    ///
    /// * The connection manager to connect to, including authentication components.
    ///
    /// # Panics
    /// If the connection manager cannot be connected to at application start.
    ///
    /// n.b. connection will be re-established if the database goes away and returns mid execution
    /// without panic.
    pub fn new(manager: T) -> Self {
        let pool = Pool::<T>::new(manager).expect("Failed to create connection pool.");

        R2D2Middleware::with_pool(pool)
    }

    /// Sets up a new instance of the middleware and establishes a connection to the database.
    ///
    /// * The connection pool (with custom configuration)
    ///
    /// n.b. connection will be re-established if the database goes away and returns mid execution
    /// without panic.
    pub fn with_pool(pool: Pool<T>) -> Self {
        R2D2Middleware {
            pool: AssertUnwindSafe(pool),
        }
    }
}

impl<T> NewMiddleware for R2D2Middleware<T>
where
    T: ManageConnection + 'static,
{
    type Instance = R2D2MiddlewareImpl<T>;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        match catch_unwind(|| self.pool.clone()) {
            Ok(pool) => Ok(R2D2MiddlewareImpl { pool }),
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

impl<T> Clone for R2D2Middleware<T>
where
    T: ManageConnection + 'static,
{
    fn clone(&self) -> Self {
        match catch_unwind(|| self.pool.clone()) {
            Ok(pool) => R2D2Middleware {
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

impl<T> Middleware for R2D2MiddlewareImpl<T>
where
    T: ManageConnection + 'static,
{
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        trace!("[{}] pre chain", request_id(&state));
        state.put(StateConnection::<T>::new(self.pool));

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
    use r2d2_sqlite::SqliteConnectionManager;
    use r2d2_redis::RedisConnectionManager;
    use redis;

    #[test]
    fn sqlite_new_with_default_config() {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::new(manager).unwrap();
        let _middleware = R2D2Middleware::with_pool(pool);
    }

    #[test]
    fn sqlite_new_with_custom_pool_config() {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::<SqliteConnectionManager>::builder()
            .min_idle(Some(1))
            .build(manager)
            .unwrap();
        let _middleware = R2D2Middleware::with_pool(pool);
    }

    #[test]
    fn redis_new_with_default_config() {
        let manager = RedisConnectionManager::new("redis://localhost").unwrap();
        let pool = Pool::new(manager).unwrap();
        let _middleware = R2D2Middleware::with_pool(pool);
    }

    #[test]
    fn redis_with_custom_pool_config() {
        let manager = RedisConnectionManager::new("redis://localhost").unwrap();
        let pool = Pool::<RedisConnectionManager>::builder()
            .min_idle(Some(1))
            .build(manager)
            .unwrap();
        let _middleware = R2D2Middleware::with_pool(pool);
    }

}
