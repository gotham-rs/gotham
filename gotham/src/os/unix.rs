use std::net::ToSocketAddrs;
use std::thread;
use std::sync::Arc;
use tokio_core::net::{Incoming, TcpListener};
use tokio_core::reactor::Handle;

use hyper::server::Http;

use handler::NewHandler;

impl ::GothamListener for Incoming {}

/// Starts a Gotham application, with the given number of threads.
pub fn start_with_num_threads<NH, A>(addr: A, threads: usize, new_handler: NH)
    where NH: NewHandler + 'static,
          A: ToSocketAddrs
{
    let listener = new_gotham_listener(addr);

    let protocol = Arc::new(Http::new());
    let new_handler = Arc::new(new_handler);

    /*
    info!(
        target: "gotham::start",
        " Gotham listening on http://{:?} with {} threads",
        addr, // ToSocketAddrs can't be formatted...
        threads,
    );
    */

    for _ in 0..threads - 1 {
        let listener = listener.clone();
        let protocol = protocol.clone();
        let new_handler = new_handler.clone();
        thread::spawn(move || ::serve_blocking(listener, &protocol, new_handler));
    }

    ::serve_blocking(listener, &protocol, new_handler);
}


fn new_gotham_listener<A: ToSocketAddrs>(addr: A, handle: Handle) -> Incoming {
    let (listener, addr) = ::tcp_listener(addr);
    let listener = TcpListener::from_listener(listener, &addr, &handle)
        .expect("unable to convert TCP listener to tokio listener");
    listener.incoming()
}
