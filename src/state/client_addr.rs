//! Defines storage for the remote address of the client

use std::net::SocketAddr;
use state::{State, FromState, StateData};

struct ClientAddr {
    addr: SocketAddr,
}

impl StateData for ClientAddr {}

pub(crate) fn put_client_addr(state: &mut State, addr: SocketAddr) {
    state.put(ClientAddr { addr })
}

/// Returns the client `SocketAddr` as reported by hyper, if one was present. Certain connections
/// do not report a client address, in which case this will return `None`.
pub fn client_addr(state: &State) -> Option<SocketAddr> {
    ClientAddr::try_borrow_from(&state).map(|c| c.addr)
}
