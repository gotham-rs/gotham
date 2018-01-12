//! Defines storage for the remote address of the client

use std::net::SocketAddr;
use state::{FromState, State, StateData};

struct ClientAddr {
    addr: SocketAddr,
}

impl StateData for ClientAddr {}

pub(crate) fn put_client_addr(state: &mut State, addr: SocketAddr) {
    state.put(ClientAddr { addr })
}

/// Returns the client `SocketAddr` as reported by hyper, if one was present. Certain connections
/// do not report a client address, in which case this will return `None`.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Response, StatusCode};
/// # use gotham::state::{State, client_addr};
/// # use gotham::test::TestServer;
/// #
/// fn my_handler(state: State) -> (State, Response) {
///     let addr = client_addr(&state).expect("no client address");
///     let body = format!("{}", addr);
///     let response = Response::new().with_status(StatusCode::Ok).with_body(body);
///     (state, response)
/// }
/// #
/// # fn main() {
/// #   let test_server = TestServer::new(|| Ok(my_handler)).unwrap();
/// #   let response = test_server
/// #       .client_with_address("127.0.0.1:9816".parse().unwrap())
/// #       .get("http://localhost/")
/// #       .perform()
/// #       .unwrap();
/// #
/// #   assert_eq!(response.status(), StatusCode::Ok);
/// #
/// #   let buf = response.read_body().unwrap();
/// #   assert_eq!(buf.as_slice(), b"127.0.0.1:9816");
/// # }
pub fn client_addr(state: &State) -> Option<SocketAddr> {
    ClientAddr::try_borrow_from(&state).map(|c| c.addr)
}
