//! Gotham - A flexible web framework that does not sacrifice safety, security or speed.
//!
//! You can find out more about Gotham, including where to get help,  at https://gotham.rs.
//!
//! We look forward to welcoming you into the Gotham community!
#![doc(html_root_url = "https://docs.rs/gotham/0.1.1")] // Update when changed in Cargo.toml
#![warn(missing_docs, deprecated)]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

#[macro_use]
extern crate hyper;
extern crate futures;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate tokio_io;
extern crate mio;
extern crate borrow_bag;
extern crate url;
extern crate uuid;
#[macro_use]
extern crate log;
extern crate chrono;
extern crate mime;
extern crate serde;
extern crate rand;
extern crate base64;
extern crate rmp_serde;
extern crate linked_hash_map;
extern crate num_cpus;

#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub mod handler;
pub mod middleware;
pub mod http;
pub mod router;
pub mod state;
pub mod test;

use std::net::SocketAddr;
use hyper::server::Http;
use handler::{NewHandler, NewHandlerService};

pub fn start<NH>(addr: SocketAddr, new_handler: NH)
where
    NH: NewHandler + 'static,
{
    let threads = num_cpus::get();
    start_with_num_threads(addr, threads, new_handler)
}

pub fn start_with_num_threads<NH>(addr: SocketAddr, threads: usize, new_handler: NH)
where
    NH: NewHandler + 'static,
{
    let server = Http::new()
        .bind(&addr, NewHandlerService::new(new_handler))
        .unwrap();

    info!(
        target: "gotham::start",
        " Gotham listening on http://{} with {} threads",
        server.local_addr().unwrap(),
        threads,
    );

    server.run().unwrap();
}
