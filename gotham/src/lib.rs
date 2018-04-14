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
extern crate serde;
extern crate tokio_core;
extern crate url;
extern crate uuid;

#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub mod handler;
pub mod middleware;
pub mod pipeline;
pub mod http;
pub mod router;
mod service;
pub mod state;
pub mod test;
mod os;

pub use os::current::start_with_num_threads;

use handler::NewHandler;
use hyper::server::Http;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::sync::Arc;
use std::io;
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::TcpStream;

use service::GothamService;
use futures::{future, Future, Stream};

trait GothamListener {
  type Stream;

  fn incoming(self, &Handle) -> Self::Stream;
}

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

fn run_and_serve<'a, G, NH>(listener: G, protocol: &'a Http, new_handler: Arc<NH>)
where
    G: GothamListener,
    <G as GothamListener>::Stream: futures::Stream<Item = (TcpStream, SocketAddr), Error=io::Error>,
    NH: NewHandler + 'static,
{
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();

    serve(listener, protocol, new_handler, &handle);

    core.run(future::ok(None));
    //.expect("unable to run reactor over listener");
}

fn serve<'a, G, NH>(listener: G, protocol: &'a Http, new_handler: Arc<NH>, handle: &'a Handle)
where
    G: GothamListener,
    <G as GothamListener>::Stream: futures::Stream<Item = (TcpStream, SocketAddr), Error=io::Error>,
    NH: NewHandler + 'static,
{
    let gotham_service = GothamService::new(new_handler, handle.clone());
    let stream = listener.incoming(handle);

    handle.spawn(stream.for_each(|(socket, addr)| {
        let service = gotham_service.connect(addr);
        let f = protocol.serve_connection(socket, service).then(|_| Ok(()));

        handle.spawn(f);
        Ok(())
    }))
}

fn pick_addr<A: ToSocketAddrs>(addr: A) -> SocketAddr {
    match addr.to_socket_addrs().map(|ref mut i| i.next()) {
        Ok(Some(a)) => a,
        Ok(_) => panic!("unable to resolve listener address"),
        Err(_) => panic!("unable to parse listener address"),
    }
}

fn tcp_listener(addr: SocketAddr) -> TcpListener {
    let addr = pick_addr(addr);

    let listener = TcpListener::bind(addr).expect("unable to open TCP listener");

    listener
}
