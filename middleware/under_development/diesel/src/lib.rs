//! Provides an interface for running Diesel queries in a Gotham application.
//!
//! # Installing the middleware
//!
//! Correct usage of the `DieselMiddleware` requires the `WorkersMiddleware`. Ensure both
//! middleware are in the pipeline used by your routes.
//!
//! ```rust
//! # extern crate gotham;
//! # extern crate gotham_middleware_workers;
//! # extern crate gotham_middleware_diesel;
//! # extern crate diesel;
//! #
//! # use gotham::router::Router;
//! # use gotham::router::builder::*;
//! # use gotham::pipeline::single::*;
//! # use gotham::pipeline::*;
//! # use gotham_middleware_workers::*;
//! # use gotham_middleware_diesel::*;
//! # use diesel::SqliteConnection;
//! #
//! fn router() -> Router {
//!     let (chain, pipelines) = single_pipeline(
//!         new_pipeline()
//!             // The middleware from `gotham_middleware_workers`, to manage
//!             // the thread pool which `DieselMiddleware` requires.
//!             .add(WorkersMiddleware::new(20))
//!             // Initialize `DieselMiddleware` with a connection string / URL.
//!             .add(DieselMiddleware::<SqliteConnection>::new(":memory:"))
//!             .build()
//!     );
//!
//!     build_router(chain, pipelines, |route| {
//!         // Your routes here...
//! #       let _ = route;
//!     })
//! }
//! #
//! # fn main() { router(); }
//! ```
//!
//! # Running a query
//!
//! At the time of writing, Diesel only supports synchronous database queries. To avoid blocking
//! the event loop, this middleware provides a `run_with_diesel` function, which executes queries
//! via the thread pool provided by `WorkersMiddleware`.
//!
//! ```rust
//! # extern crate gotham;
//! # extern crate gotham_middleware_workers;
//! # extern crate gotham_middleware_diesel;
//! # extern crate diesel;
//! # extern crate futures;
//! # extern crate hyper;
//! # extern crate mime;
//! #
//! # use gotham::handler::*;
//! # use gotham::helpers::http::response::*;
//! # use gotham::router::Router;
//! # use gotham::router::builder::*;
//! # use gotham::pipeline::single::*;
//! # use gotham::pipeline::*;
//! # use gotham::state::*;
//! # use gotham::test::*;
//! # use gotham_middleware_workers::*;
//! # use gotham_middleware_diesel::*;
//! # use futures::*;
//! # use hyper::StatusCode;
//! # use diesel::{RunQueryDsl, SqliteConnection};
//! #
//! # fn router() -> Router {
//! #   let (chain, pipelines) = single_pipeline(
//! #       new_pipeline()
//! #           .add(WorkersMiddleware::new(20))
//! #           .add(DieselMiddleware::<SqliteConnection>::new(":memory:"))
//! #           .build()
//! #   );
//! #
//! #   build_router(chain, pipelines, |route| {
//! #       route.get("/").to(handler);
//! #   })
//! # }
//! #
//! fn handler(state: State) -> Box<HandlerFuture> {
//!     // Ownership of `state` is taken by `run_with_diesel`. Since we can't
//!     // pass it to other threads, it can't be captured by the closure. The
//!     // `state` value will be yielded by the future upon success or error.
//!     let f = run_with_diesel(state, |conn: &SqliteConnection| {
//!         // In this context, we are on a background thread. We can do
//!         // synchronous operations without blocking the event loop.
//!         //
//!         // As an example, we perform the query:
//!         // `SELECT 1`
//!         //
//!         // The result is loaded into a single `i64` which is yielded by the
//!         // future upon successful completion.
//!         diesel::select(diesel::dsl::sql("1"))
//!             .load::<i64>(conn)
//!             .map(|v| v.into_iter().next().expect("no results"))
//!     }).then(|r| {
//!         // Continuations to the future returned from `run_with_diesel` will
//!         // be run on the event loop. Note that we no longer have access to
//!         // the `conn` value, but we've now regained our `state` via `r`
//!         // (which is a `Result<(State, i64), (State, HandlerError)>`).
//!         //
//!         // We unwrap `r` to get `state` and the result from the query, and
//!         // then render it into the response body.
//!         let (state, n) = r.unwrap_or_else(|_| panic!("query failed"));
//!         let body = format!("result: {}", n);
//!
//!         // Use Gotham's `create_response` helper to populate the response.
//!         let response = create_response(
//!             &state,
//!             StatusCode::Ok,
//!             Some((body.into_bytes(), mime::TEXT_PLAIN)),
//!         );
//!
//!         // Complete the future with an immediately available value.
//!         Ok((state, response))
//!     });
//!
//!     Box::new(f)
//! }
//! #
//! # fn main() {
//! #   let test_server = TestServer::new(router()).unwrap();
//! #   let response = test_server
//! #       .client()
//! #       .get("https://example.com/")
//! #       .perform()
//! #       .unwrap();
//! #   assert_eq!(response.status(), StatusCode::Ok);
//! #   let body = response.read_utf8_body().unwrap();
//! #   assert_eq!(&body, "result: 1");
//! # }
//! ```

#![warn(missing_docs, deprecated)]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

extern crate diesel;
extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate gotham_middleware_workers;
#[macro_use]
extern crate log;
extern crate r2d2;
extern crate r2d2_diesel;

#[cfg(test)]
extern crate hyper;

#[cfg(test)]
extern crate mime;

mod job;
mod middleware;
mod state_data;

pub use job::run_with_diesel;
pub use middleware::DieselMiddleware;
pub use state_data::Diesel;
