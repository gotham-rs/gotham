use std::net::{SocketAddr, TcpListener};
use tokio_core::net::{self, Incoming};
use tokio_core::reactor::Handle;

pub struct Listener {
    tcp: TcpListener,
    addr: SocketAddr,
}

impl Clone for Listener {
    fn clone(&self) -> Self {
        Listener {
            tcp: self.tcp.try_clone().unwrap(),
            addr: self.addr.clone(),
        }
    }
}

impl ::GothamListener for Listener {
    type Stream = Incoming;
    fn incoming(self, handle: Handle) -> Self::Stream {
        let tcp = self.tcp.try_clone().expect("Couldn't clone TCP listener.");
        let listener = net::TcpListener::from_listener(tcp, &self.addr, &handle)
            .expect("unable to convert TCP listener to tokio listener");
        listener.incoming()
    }
}

/// Constructs a GothamListener to handle incoming TCP connections.
///
/// The Unix implementation of GothamListener clones the std::net::TcpListener,
/// wraps it in a tokio::net::TcpListener and returns its Incoming.
pub fn new_gotham_listener(addr: SocketAddr) -> Listener {
    let tcp = TcpListener::bind(addr).expect("unable to open TCP listener");
    Listener { tcp, addr }
}
