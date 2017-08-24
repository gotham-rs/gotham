//! Makes a Diesel connection available to every Middleware and Handler that is involved in
//! processing a single Gotham request.
//!
//! Utilises r2d2 pooling to ensure efficent database usage and prevent resource exhaustion.

#![warn(missing_docs, deprecated)]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

#[macro_use]
extern crate log;
extern crate futures;
extern crate hyper;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate diesel;
extern crate r2d2;
extern crate r2d2_diesel;

pub mod state_data;

use std::io;

use futures::{Future, future};
use hyper::Request;

use gotham::middleware::{NewMiddleware, Middleware};
use gotham::state::{State, request_id};
use gotham::handler::HandlerFuture;

use diesel::Connection;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;

use state_data::Diesel;

/// A Gotham compatible Middleware that manages a pool of Diesel connections via r2d2 and hands
/// out connections to other Middleware and Handlers that require them via the Gotham `State`
/// mechanism.
pub struct DieselMiddleware<T>
where
    T: Connection + Send + 'static,
{
    pool: r2d2::Pool<ConnectionManager<T>>,
}

impl<T> DieselMiddleware<T>
where
    T: Connection + Send + 'static,
{
    /// Sets up a new instance of the middleware and establishes a connection to the database.
    ///
    /// * The database to connect to, including authentication components.
    /// * An optional config instance. Utilises `r2d2::Config::default()` if not provided.
    ///
    /// # Panics
    /// If the database identified in `database_url` cannot be connected to at application start.
    ///
    /// n.b. connection will be re-established if the database goes away and returns mid execution
    /// without panic.
    pub fn new(database_url: &str, c: Option<r2d2::Config<T, r2d2_diesel::Error>>) -> Self {
        let manager = ConnectionManager::<T>::new(database_url);
        let r2d2_config = match c {
            Some(c) => c,
            None => r2d2::Config::default(),
        };
        let pool = Pool::<ConnectionManager<T>>::new(r2d2_config, manager)
            .expect("Failed to create pool.");

        DieselMiddleware { pool }
    }
}

impl<T> NewMiddleware for DieselMiddleware<T>
where
    T: Connection + Send + 'static,
{
    type Instance = DieselMiddleware<T>;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        let pool = self.pool.clone();
        Ok(DieselMiddleware { pool })
    }
}

impl<T> Middleware for DieselMiddleware<T>
where
    T: Connection + Send + 'static,
{
    fn call<Chain>(self, mut state: State, request: Request, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State, Request) -> Box<HandlerFuture>,
    {
        trace!("[{}] pre chain", request_id(&state));
        state.put(Diesel::<T>::new(self.pool));

        chain(state, request)
            .and_then(move |(state, response)| {
                {
                    trace!("[{}] post chain", request_id(&state));
                }
                future::ok((state, response))
            })
            .boxed()
    }
}
