use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;
use std::sync::Arc;

use hyper::server::Http;
use tokio_core;
use tokio_core::reactor::Core;
use futures::{Future, Stream};

use handler::NewHandler;
use service::GothamService;

/// Starts a Gotham application, with the given number of threads.
pub fn start_with_num_threads<NH, A>(addr: A, threads: usize, new_handler: NH)
    where NH: NewHandler + 'static,
          A: ToSocketAddrs
{
    let listener = new_gotham_listener(addr);

    let protocol = Arc::new(Http::new());
    let new_handler = Arc::new(new_handler);

    info!(
        target: "gotham::start",
        " Gotham listening on http://{} with {} threads",
        addr,
        threads,
    );

    for _ in 0..threads - 1 {
        let listener = listener
            .try_clone()
            .expect("unable to clone TCP listener");
        let protocol = protocol.clone();
        let new_handler = new_handler.clone();
        thread::spawn(move || ::serve_blocking(listener, &addr, &protocol, new_handler));
    }

    ::serve_blocking(listener, &addr, &protocol, new_handler);
}


fn new_gotham_listener<A: ToSocketAddrs>(addr: A) -> Incoming {
    ::tcp_listener(addr)
    let listener = tokio_core::net::TcpListener::from_listener(listener, addr, &handle)
        .expect("unable to convert TCP listener to tokio listener");
    listener.incoming()
}
