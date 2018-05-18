//! Gotham &ndash; A flexible web framework that promotes stability, safety, security and speed.
//!
//! You can find out more about Gotham, including where to get help, at <https://gotham.rs>.
//!
//! We look forward to welcoming you into the Gotham community!
#![doc(html_root_url = "https://docs.rs/gotham/0.2.1")] // Update when changed in Cargo.toml
#![warn(missing_docs, deprecated)]
// Stricter requirements once we get to pull request stage, all warnings must be resolved.
#![cfg_attr(feature = "ci", deny(warnings))]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

extern crate base64;
extern crate bincode;
extern crate borrow_bag;
extern crate chrono;
#[cfg(windows)]
extern crate crossbeam;
extern crate futures;
#[macro_use]
extern crate hyper;
extern crate linked_hash_map;
#[macro_use]
extern crate log;
extern crate mime;
extern crate mio;
extern crate num_cpus;
extern crate rand;
extern crate regex;
#[macro_use]
extern crate serde;
extern crate tokio_core;
extern crate url;
extern crate uuid;

#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub mod extractor;
pub mod handler;
pub mod helpers;
pub mod middleware;
pub mod pipeline;
pub mod router;
mod service;
pub mod state;
pub mod test;
mod os;

pub use os::current::{run_with_num_threads_until, start_with_num_threads};

use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::time::Duration;
use futures::Future;
use handler::NewHandler;

/// Starts a Gotham application, with the default number of threads (equal to the number of CPUs) and ability to
/// gracefully shut down.
///
/// This function blocks current thread until `shutdown_signal` resolved or panic occur.
///
/// When `shutdown_timeout` is not equal to `Duration::default()`, function waits for remaining open connections to
/// finish for specified time.
///
/// ## Windows
///
/// An additional thread is used on Windows to accept connections.
pub fn run_until<NH, A, F>(addr: A, new_handler: NH, shutdown_signal: F, shutdown_timeout: Duration)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
    F: Future<Item = (), Error = ()>,
{
    let threads = num_cpus::get();
    run_with_num_threads_until(
        addr,
        threads,
        new_handler,
        shutdown_signal,
        shutdown_timeout,
    )
}

/// Starts a Gotham application, with the default number of threads (equal to the number of CPUs).
///
/// This function never return but may panic because of errors.
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
