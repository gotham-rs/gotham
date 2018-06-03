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

use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::sync::Arc;
use std::thread;

use futures::{Future, Stream};
use hyper::server::Http;
use tokio_core::reactor::Core;

use handler::NewHandler;
use service::GothamService;

/// Starts a Gotham application, with the default number of threads (equal to the number of CPUs).
pub fn start<NH, A>(addr: A, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let threads = num_cpus::get();
    let (listener, addr) = ::tcp_listener(addr);

    let protocol = Arc::new(Http::new());
    let new_handler = Arc::new(new_handler);

    info!(
        target: "gotham::start",
        " Gotham listening on http://{} with {} threads",
        addr,
        threads,
    );

    for thread_n in 0..threads - 1 {
        let listener = listener.try_clone().expect("unable to clone TCP listener");
        let protocol = protocol.clone();
        let new_handler = new_handler.clone();
        thread::Builder::new()
            .name(format!("gotham-{}", thread_n))
            .spawn(move || start_core(listener, &addr, &protocol, new_handler))
            .expect("unable to spawn thread");
    }

    start_core(listener, &addr, &protocol, new_handler);
}

fn start_core<NH>(listener: TcpListener, addr: &SocketAddr, protocol: &Http, new_handler: Arc<NH>)
where
    NH: NewHandler + 'static,
{
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();

    let gotham_service = GothamService::new(new_handler, handle.clone());

    let listener = tokio_core::net::TcpListener::from_listener(listener, addr, &handle)
        .expect("unable to convert TCP listener to tokio listener");

    let f = Box::new(listener.incoming().for_each(move |(socket, addr)| {
        let service = gotham_service.connect(addr);
        let f = protocol.serve_connection(socket, service).then(|_| Ok(()));

        handle.spawn(f);
        Ok(())
    }));

    core.run(f).expect("unable to run reactor over listener");
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
