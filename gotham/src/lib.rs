//! Gotham - A flexible web framework that does not sacrifice safety, security or speed.
//!
//! You can find out more about Gotham, including where to get help,  at https://gotham.rs.
//!
//! We look forward to welcoming you into the Gotham community!
#![doc(html_root_url = "https://docs.rs/gotham/0.2.0")] // Update when changed in Cargo.toml
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
//#[cfg(windows)] //xxx put this back
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

pub use os::current::new_gotham_listener;

use handler::NewHandler;
use hyper::server::Http;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::sync::Arc;
use std::io;
use std::thread;
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::TcpStream;

use service::GothamService;
use futures::{future, Future, Stream};

/// Abstracts over TCPListener to provide OS independence for handling incoming TCP connections.
pub trait GothamListener {
    /// The type for incoming stream of TCP connections.
    type Stream;

    /// Incoming is called in each processing thread to get a stream of TCP connections.
    fn incoming(self, Handle) -> Self::Stream;
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

fn start_with_num_threads<NH, A>(addr: A, threads: usize, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let addr = pick_addr(addr);
    let tcp = tcp_listener(addr);

    let listener = new_gotham_listener(tcp, addr);

    let protocol = Arc::new(Http::new());
    let new_handler = Arc::new(new_handler);

    for _ in 0..threads - 1 {
        let listener = listener.clone();
        let protocol = protocol.clone();
        let new_handler = new_handler.clone();
        thread::spawn(move || run_and_serve(listener, protocol, new_handler));
    }

    run_and_serve(listener, protocol, new_handler);
}

fn run_and_serve<'a, G, NH>(listener: G, protocol: Arc<Http>, new_handler: Arc<NH>)
where
    G: GothamListener,
    <G as GothamListener>::Stream: futures::Stream<Item = (TcpStream, SocketAddr), Error = io::Error>
        + 'static,
    NH: NewHandler + 'static,
{
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();

    serve(listener, protocol, new_handler, handle);

    core.run::<future::FutureResult<(), ()>>(future::ok(()))
        .expect("unable to run reactor over listener");
}

/// Serves a Gotham handler on a GothamListener.  Useful when you're folding Gotham into an existing Tokio application.
pub fn serve<G, NH>(listener: G, protocol: Arc<Http>, new_handler: Arc<NH>, handle: Handle)
where
    G: GothamListener,
    <G as GothamListener>::Stream: futures::Stream<Item = (TcpStream, SocketAddr), Error = io::Error>
        + 'static,
    NH: NewHandler + 'static,
{
    let gotham_service = GothamService::new(new_handler, handle.clone());
    let stream = listener.incoming(handle.clone());
    let inner = handle.clone();

    handle.spawn(
        stream
            .for_each(move |(socket, addr)| {
                let service = gotham_service.connect(addr);
                let f = protocol.serve_connection(socket, service).then(|_| Ok(()));

                inner.spawn(f);
                Ok(())
            })
            .or_else(|_| future::ok(())),
    )
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
