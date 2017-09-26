//! Gotham - A flexible web framework that does not sacrifice safety, security or speed.
//!
//! You can find out more about Gotham, including where to get help,  at https://gotham.rs.
//!
//! We look forward to welcoming you into the Gotham community!
#![doc(html_root_url = "https://docs.rs/gotham/0.1.2")] // Update when changed in Cargo.toml
#![warn(missing_docs, deprecated)]

// Stricter requirements once we get to pull request stage, all warnings must be resolved.
#![cfg_attr(feature = "ci", deny(warnings))]

#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

#[macro_use]
extern crate hyper;
extern crate futures;
extern crate tokio_core;
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
extern crate regex;
#[cfg(windows)]
extern crate crossbeam;

#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub mod handler;
pub mod middleware;
pub mod http;
pub mod router;
pub mod state;
pub mod test;

#[cfg(not(windows))]
mod clone_sockets;
#[cfg(not(windows))]
pub use clone_sockets::start_with_num_threads;

#[cfg(windows)]
mod socket_queue;
#[cfg(windows)]
pub use socket_queue::start_with_num_threads;

use std::net::{SocketAddr, ToSocketAddrs, TcpListener};
use handler::NewHandler;

/// Starts a Gotham application, with the default number of threads (equal to the number of CPUs).
///
/// ## Windows
///
/// An additional thread is used on Windows to accept connections.
pub fn start<NH, A>(addr: A, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let threads = num_cpus::get();
    start_with_num_threads(addr, threads, new_handler)
}

fn tcp_listener<A>(addr: A) -> (TcpListener, SocketAddr)
where
    A: ToSocketAddrs,
{
    let addr = match addr.to_socket_addrs().map(|ref mut i| i.next()) {
        Ok(Some(a)) => a,
        Ok(_) => panic!("unable to resolve listener address"),
        Err(_) => panic!("unable to parse listener address"),
    };

    let listener = TcpListener::bind(addr).expect("unable to open TCP listener");

    (listener, addr)
}
