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
extern crate tokio;
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

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use futures::{Future, Stream};
use hyper::server::Http;
use hyper::Chunk;
use tokio::executor::thread_pool;
use tokio::net::TcpListener;
use tokio::runtime::{self, Runtime, TaskExecutor};

use handler::NewHandler;
use service::GothamService;

/// Starts a Gotham application with the default number of threads.
pub fn start<NH, A>(addr: A, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    start_with_num_threads(addr, new_handler, num_cpus::get())
}

/// Starts a Gotham application with a designated number of threads.
pub fn start_with_num_threads<NH, A>(addr: A, new_handler: NH, threads: usize)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let runtime = new_runtime(threads);
    start_on_executor(addr, new_handler, runtime.executor());
    runtime.shutdown_on_idle().wait().unwrap();
}

/// Starts a Gotham application with a designated backing `TaskExecutor`.
///
/// This function can be used to spawn the server on an existing `Runtime`.
pub fn start_on_executor<NH, A>(addr: A, new_handler: NH, executor: TaskExecutor)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let (listener, addr) = tcp_listener(addr);
    let gotham_service = GothamService::new(new_handler);
    let protocol = Arc::new(Http::<Chunk>::new());

    info!(
        target: "gotham::start",
        " Gotham listening on http://{}",
        addr
    );

    let executor = Arc::new(executor);

    let main = executor.clone();
    let executor = executor.clone();

    let server = listener
        .incoming()
        .map_err(|e| panic!("error = {:?}", e))
        .for_each(move |socket| {
            let service = gotham_service.connect(socket.peer_addr().unwrap());
            let handler = protocol.serve_connection(socket, service).then(|_| Ok(()));

            Ok(executor.spawn(handler))
        });

    main.spawn(server);
}

fn new_runtime(threads: usize) -> Runtime {
    let mut pool_builder = thread_pool::Builder::new();

    pool_builder
        .name_prefix("gotham-worker-")
        .pool_size(threads);

    runtime::Builder::new()
        .threadpool_builder(pool_builder)
        .build()
        .unwrap()
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

    let listener = TcpListener::bind(&addr).expect("unable to open TCP listener");

    (listener, addr)
}
