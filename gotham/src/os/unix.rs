use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use tokio_core::net::{self, Incoming};
use tokio_core::reactor::Handle;

#[derive(Clone)]
pub struct Listener {
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

/// Constructs a GothamListener to handle incoming TCP connections.
///
/// The Unix implementation of GothamListener clones the std::net::TcpListener,
/// wraps it in a tokio::net::TcpListener and returns its Incoming.
pub fn new_gotham_listener(tcp: TcpListener, addr: SocketAddr) -> Listener {
    Listener {
        wrapped: Arc::new(tcp),
        addr: addr,
    }
}
