//! Provides an interface for running Diesel queries in a Gotham application.
//!
//! The gotham diesel middleware uses [tokio::task::spawn_blocking], which allows
//! blocking operations to run without blocking the tokio reactor. Although not true async,
//! this allows multiple concurrent database requests to be handled, with a default of 100
//! concurrent blocking operations.
//!
//! Usage example:
//!
//! ```rust
//! # use diesel::{RunQueryDsl, SqliteConnection};
//! # use diesel::sql_types::Int8;
//! # use futures_util::FutureExt;
//! # use gotham::router::Router;
//! # use gotham::router::builder::*;
//! # use gotham::pipeline::*;
//! # use gotham::state::{FromState, State};
//! # use gotham::helpers::http::response::create_response;
//! # use gotham::handler::HandlerFuture;
//! # use gotham_middleware_diesel::{self, DieselMiddleware};
//! # use gotham::hyper::StatusCode;
//! # use gotham::test::TestServer;
//! # use gotham::mime::TEXT_PLAIN;
//! # use std::pin::Pin;
//!
//! pub type Repo = gotham_middleware_diesel::Repo<SqliteConnection>;
//!
//! fn router() -> Router {
//!     // Create a Repo - using an in memory Sqlite DB
//!     let repo = Repo::new(":memory:");
//!     // Add the diesel middleware to a new pipeline
//!     let (chain, pipeline) =
//!         single_pipeline(new_pipeline().add(DieselMiddleware::new(repo)).build());
//!
//!     // Build the router
//!     build_router(chain, pipeline, |route| {
//!         route.get("/").to(handler);
//!     })
//! }
//!
//! fn handler(state: State) -> Pin<Box<HandlerFuture>> {
//!     let repo = Repo::borrow_from(&state).clone();
//!     // As an example, we perform the query:
//!     // `SELECT 1`
//!     async move {
//!         let result = repo
//!             .run(move |mut conn| {
//!                 diesel::select(diesel::dsl::sql::<Int8>("1"))
//!                     .load::<i64>(&mut conn)
//!                     .map(|v| v.into_iter().next().expect("no results"))
//!             })
//!             .await;
//!         match result {
//!             Ok(n) => {
//!                 let body = format!("result: {}", n);
//!                 let res = create_response(&state, StatusCode::OK, TEXT_PLAIN, body);
//!                 Ok((state, res))
//!             }
//!             Err(e) => Err((state, e.into())),
//!         }
//!     }
//!     .boxed()
//! }
//!
//! # fn main() {
//! #    let test_server = TestServer::new(router()).unwrap();
//! #    let response = test_server
//! #        .client()
//! #        .get("https://example.com/")
//! #        .perform()
//! #        .unwrap();
//! #    assert_eq!(response.status(), StatusCode::OK);
//! #    let body = response.read_utf8_body().unwrap();
//! #    assert_eq!(&body, "result: 1");
//! # }
//! ```
#![warn(rust_2018_idioms, unreachable_pub)]
#![forbid(elided_lifetimes_in_paths, unsafe_code)]
#![doc(test(no_crate_inject, attr(allow(unused_variables), deny(warnings))))]

use diesel::r2d2::R2D2Connection;
use futures_util::future::{self, FutureExt, TryFutureExt};
use log::{error, trace};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::process;

use gotham::handler::HandlerFuture;
use gotham::middleware::Middleware;
use gotham::prelude::*;
use gotham::state::{request_id, State};

mod repo;
pub use repo::Repo;

/// A Gotham compatible Middleware that manages a pool of Diesel connections via a `Repo` and hands
/// out connections to other Middleware and Handlers that require them via the Gotham `State`
/// mechanism.
#[derive(NewMiddleware)]
pub struct DieselMiddleware<T>
where
    T: R2D2Connection + 'static,
{
    repo: AssertUnwindSafe<Repo<T>>,
}

impl<T> DieselMiddleware<T>
where
    T: R2D2Connection,
{
    pub fn new(repo: Repo<T>) -> Self {
        DieselMiddleware {
            repo: AssertUnwindSafe(repo),
        }
    }
}

impl<T> Clone for DieselMiddleware<T>
where
    T: R2D2Connection + 'static,
{
    fn clone(&self) -> Self {
        match catch_unwind(|| self.repo.clone()) {
            Ok(repo) => DieselMiddleware {
                repo: AssertUnwindSafe(repo),
            },
            Err(_) => {
                error!("PANIC: r2d2::Pool::clone caused a panic");
                process::abort()
            }
        }
    }
}

impl<T> Middleware for DieselMiddleware<T>
where
    T: R2D2Connection + 'static,
{
    fn call<Chain>(self, mut state: State, chain: Chain) -> Pin<Box<HandlerFuture>>
    where
        Chain: FnOnce(State) -> Pin<Box<HandlerFuture>> + 'static,
        Self: Sized,
    {
        trace!("[{}] pre chain", request_id(&state));
        state.put(self.repo.clone());

        let f = chain(state).and_then(move |(state, response)| {
            {
                trace!("[{}] post chain", request_id(&state));
            }
            future::ok((state, response))
        });
        f.boxed()
    }
}
