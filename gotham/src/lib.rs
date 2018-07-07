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
use tokio::net::TcpListener;

use handler::NewHandler;
use service::GothamService;

/// Starts a Gotham application.
pub fn start<NH, A>(addr: A, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static,
{
    let server = create_server(addr, new_handler);
    tokio::run(server);
}

/// Returns future which combines TCP listener with Gotham application.
pub fn create_server<NH, A>(addr: A, new_handler: NH) -> impl Future<Item = (), Error = ()>
    where
        NH: NewHandler + 'static,
        A: ToSocketAddrs,
{
    let (listener, addr) = tcp_listener(addr);
    let gotham_service = GothamService::new(new_handler);
    let protocol = Arc::new(Http::<Chunk>::new());

    info!(
        target: "gotham::start",
        " Starting Gotham to listen on http://{}",
        addr
    );

    listener
        .incoming()
        .map_err(|e| panic!("error = {:?}", e))
        .for_each(move |socket| {
            let service = gotham_service.connect(socket.peer_addr().unwrap());
            let f = protocol.serve_connection(socket, service).then(|_| Ok(()));

            tokio::spawn(f);
            Ok(())
        })
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
