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

extern crate hyper;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate mio;

pub mod handler;
pub mod router;
pub mod test;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
