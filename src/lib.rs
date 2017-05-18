//! A fast and safe web application framework
//!
//! This crate builds on the [hyper][], [tokio][], [futures][], and [mio][] libraries to provide an
//! ergonomic API for routing requests and structuring a web application without sacrificing type
//! safety.
//!
//! [hyper]: https://github.com/hyperium/hyper
//! [tokio]: https://github.com/tokio-rs/tokio
//! [futures]: https://github.com/alexcrichton/futures-rs
//! [mio]: https://github.com/carllerche/mio

#![warn(missing_docs)]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

extern crate hyper;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate mio;
extern crate borrow_bag;

pub mod dispatch;
pub mod handler;
pub mod middleware;
pub mod router;
pub mod state;
pub mod test;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
