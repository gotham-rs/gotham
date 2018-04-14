use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;
use std::sync::Arc;
use tokio_core::net::{self, Incoming};
use tokio_core::reactor::Handle;

use hyper::server::Http;

use handler::NewHandler;

#[derive(Clone)]
struct Listener {
    wrapped: Arc<TcpListener>,
    addr: SocketAddr,
}

impl ::GothamListener for Listener {
    type Stream = Incoming;
    fn incoming(self, handle: Handle) -> Self::Stream {
        let tcp = (*self.wrapped)
            .try_clone()
            .expect("Couldn't clone TCP listener.");
        let listener = net::TcpListener::from_listener(tcp, &self.addr, &handle)
            .expect("unable to convert TCP listener to tokio listener");
        listener.incoming()
    }
}

/// Starts a Gotham application, with the given number of threads.
pub fn start_with_num_threads<NH, A>(addr: A, threads: usize, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let addr = ::pick_addr(addr);
    let tcp = ::tcp_listener(addr);

    let listener = new_gotham_listener(tcp, addr);

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
        thread::spawn(move || ::run_and_serve(listener, protocol, new_handler));
    }

    ::run_and_serve(listener, protocol, new_handler);
}

fn new_gotham_listener(tcp: TcpListener, addr: SocketAddr) -> Listener {
    Listener {
        wrapped: Arc::new(tcp),
        addr: addr,
    }
}
