//! Makes a Diesel connection available to Middleware and Handlers that are involved in
//! processing a Request.
//!
//! Utilises r2d2 pooling to ensure efficent database usage and prevent resource exhaustion.

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

mod state_data;
mod job;
mod middleware;

pub use job::run_with_diesel;
pub use state_data::Diesel;
pub use middleware::DieselMiddleware;
