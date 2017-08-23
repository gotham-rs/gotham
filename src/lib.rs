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

use std::net::{SocketAddr, ToSocketAddrs, TcpListener};
use std::thread;
use std::sync::Arc;

use hyper::server::{Http, NewService};
use tokio_core::reactor::Core;
use futures::Stream;

use handler::{NewHandler, NewHandlerService};

pub fn start<NH, A>(addr: A, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let threads = num_cpus::get();
    start_with_num_threads(addr, threads, new_handler)
}

pub fn start_with_num_threads<NH, A>(addr: A, threads: usize, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let addr = match addr.to_socket_addrs().map(|ref mut i| i.next()) {
        Ok(Some(a)) => a,
        Ok(_) => panic!("unable to resolve listener address"),
        Err(_) => panic!("unable to parse listener address"),
    };

    let listener = TcpListener::bind(addr).expect("unable to open TCP listener");
    let protocol = Arc::new(Http::new());
    let service = NewHandlerService::new(new_handler);

    info!(
        target: "gotham::start",
        " Gotham listening on http://{} with {} threads",
        server.local_addr().unwrap(),
        threads,
    );

    for _ in 0..threads - 1 {
        let listener = listener.try_clone().expect("unable to clone TCP listener");
        let protocol = protocol.clone();
        let service = service.clone();
        thread::spawn(move || serve(listener, &addr, &protocol, &service));
    }

    serve(listener, &addr, &protocol, &service);
}

fn serve<NH>(
    listener: TcpListener,
    addr: &SocketAddr,
    protocol: &Http,
    new_service: &NewHandlerService<NH>,
) where
    NH: NewHandler + 'static,
{
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();

    let listener = tokio_core::net::TcpListener::from_listener(listener, addr, &handle)
        .expect("unable to convert TCP listener to tokio listener");

    core.run(listener.incoming().for_each(|(socket, addr)| {
        match new_service.new_service() {
            Ok(service) => protocol.bind_connection(&handle, socket, addr, service),
            Err(e) => error!(" unable to spawn service: {:?}", e),
        }
        Ok(())
    })).expect("unable to run reactor over listener");
}
