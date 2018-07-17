//! The workers middleware creates a single thread pool for a Gotham app to run tasks in the
//! "background", without blocking the main event loop. The thread pool can process blocking I/O
//! and long-running computation without degrading the performance of other requests.
//!
//! Libraries targeting Gotham web framework can expose an API which takes a `State` instance,
//! creates a job to run in the background, and returns a `Future` to wait on its completion.
//! Generally, Gotham apps need only ensure that the `WorkersMiddleware` is correctly configured
//! and available to libraries that need it.
//!
//! # Caveats
//!
//! This is not a replacement for asynchronous logic, but rather a workaround which allows
//! synchronous APIs to be used without impacting the event loop. Before using this crate, consider
//! whether an asynchronous API is available, as it may have superior performance or scaling
//! characteristics.
//!
//! # Examples
//!
//! Typical usage of `WorkersMiddleware`:
//!
//! ```rust
//! # extern crate gotham;
//! # extern crate hyper;
//! # extern crate futures;
//! # extern crate mime;
//! #
//! # use futures::Future;
//! # use hyper::StatusCode;
//! # use gotham::handler::{HandlerFuture, HandlerError, IntoHandlerError};
//! # use gotham::helpers::http::response::create_response;
//! # use gotham::state::State;
//! # use gotham::router::Router;
//! # use gotham::router::builder::*;
//! # use gotham::pipeline::*;
//! # use gotham::pipeline::single::*;
//! # use gotham::middleware::workers::WorkersMiddleware;
//! # use gotham::test::TestServer;
//! #
//! # mod some_library {
//! #   use super::*;
//! #
//! #   pub enum Impossible {}
//! #
//! #   impl IntoHandlerError for Impossible {
//! #       fn into_handler_error(self) -> HandlerError {
//! #           unreachable!()
//! #       }
//! #   }
//! #
//! #   pub fn run_with_worker(
//! #       state: State,
//! #   ) -> Box<Future<Item = (State, u64), Error = (State, Impossible)> + Send> {
//! #       gotham::middleware::workers::run_with_worker(state, |_state: &mut State| || Ok(1))
//! #   }
//! # }
//! #
//! pub fn router() -> Router {
//!     // In this example, the app has a single pipeline containing only
//!     // the `WorkersMiddleware`.
//!     let (chain, pipelines) = single_pipeline(
//!         new_pipeline()
//!             .add(WorkersMiddleware::new(20))
//!             .build()
//!     );
//!
//!     // Build the router with the pipeline we just created.
//!     build_router(chain, pipelines, |route| {
//!         route.get("/").to(my_handler);
//!     })
//! }
//!
//! fn my_handler(state: State) -> Box<HandlerFuture> {
//!     // Using a library which requires the workers middleware. In this example,
//!     // `some_library::run_with_worker` is yielding a `u64` value.
//!     let f = some_library::run_with_worker(state)
//!
//!         // Map from the library's error type into Gotham's `HandlerError`.
//!         .map_err(|(state, err)| (state, err.into_handler_error()))
//!
//!         // Build the response after the computation completes successfully.
//!         .map(|(state, num)| {
//!             let body = format!("{}", num).into_bytes();
//!
//!             let response = create_response(
//!                 &state,
//!                 StatusCode::Ok,
//!                 Some((body, mime::TEXT_PLAIN))
//!             );
//!
//!             (state, response)
//!         });
//!
//!     Box::new(f)
//! }
//! #
//! # pub fn main() {
//! #   let test_server = TestServer::new(router()).unwrap();
//! #   let response = test_server
//! #       .client()
//! #       .get("https://example.com/")
//! #       .perform()
//! #       .unwrap();
//! #   assert_eq!(response.status(), StatusCode::Ok);
//! #   let body = response.read_utf8_body().unwrap();
//! #   assert_eq!(&body, "1");
//! # }
//! ```

mod job;
mod middleware;
mod pool;

pub use self::job::{run_with_worker, Job, PreparedJob};
pub use self::middleware::WorkersMiddleware;
