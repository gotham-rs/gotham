//! Defines the `GothamService` type which is used to wrap a Gotham application and interface with
//! Hyper.

use std::thread;
use std::net::SocketAddr;
use std::sync::Arc;
use std::panic::AssertUnwindSafe;

use hyper;
use hyper::server::Service;
use hyper::{Request, Response};
use futures::Future;
use tokio_core::reactor::Handle;

use handler::NewHandler;
use state::{request_id, set_request_id, State};
use state::client_addr::put_client_addr;
use http::request::path::RequestPathSegments;

mod timing;
mod trap;

/// Wraps a `NewHandler` which will be used to serve requests. Used in `gotham::os::*` to bind
/// incoming connections to `ConnectedGothamService` values.
pub(crate) struct GothamService<T>
where
    T: NewHandler + 'static,
{
    t: Arc<T>,
    handle: Handle,
}

impl<T> GothamService<T>
where
    T: NewHandler + 'static,
{
    pub(crate) fn new(t: Arc<T>, handle: Handle) -> GothamService<T> {
        GothamService { t, handle }
    }

    pub(crate) fn connect(&self, client_addr: SocketAddr) -> ConnectedGothamService<T> {
        ConnectedGothamService {
            t: self.t.clone(),
            handle: self.handle.clone(),
            client_addr,
        }
    }
}

/// A `GothamService` which has been connected to a client. The major difference is that a
/// `client_addr` has been assigned (as this isn't available from Hyper).
pub(crate) struct ConnectedGothamService<T>
where
    T: NewHandler + 'static,
{
    t: Arc<T>,
    handle: Handle,
    client_addr: SocketAddr,
}

impl<T> Service for ConnectedGothamService<T>
where
    T: NewHandler,
{
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let mut state = State::new();

        put_client_addr(&mut state, self.client_addr);

        let (method, uri, version, headers, body) = req.deconstruct();

        state.put(self.handle.clone());
        state.put(RequestPathSegments::new(uri.path()));
        state.put(method);
        state.put(uri);
        state.put(version);
        state.put(headers);
        state.put(body);
        set_request_id(&mut state);

        debug!(
            "[DEBUG][{}][Thread][{:?}]",
            request_id(&state),
            thread::current().id(),
        );

        trap::call_handler(self.t.as_ref(), AssertUnwindSafe(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::{Method, StatusCode};
    use tokio_core::reactor::Core;

    use http::response::create_response;
    use router::builder::*;
    use state::State;

    fn handler(state: State) -> (State, Response) {
        let res = create_response(&state, StatusCode::Accepted, None);
        (state, res)
    }

    #[test]
    fn new_handler_closure() {
        let mut core = Core::new().unwrap();
        let service = GothamService::new(Arc::new(|| Ok(handler)), core.handle());

        let req = Request::new(Method::Get, "http://localhost/".parse().unwrap());
        let f = service
            .connect("127.0.0.1:10000".parse().unwrap())
            .call(req);
        let response = core.run(f).unwrap();
        assert_eq!(response.status(), StatusCode::Accepted);
    }

    #[test]
    fn router() {
        let router = build_simple_router(|route| {
            route.get("/").to(handler);
        });

        let mut core = Core::new().unwrap();
        let service = GothamService::new(Arc::new(router), core.handle());

        let req = Request::new(Method::Get, "http://localhost/".parse().unwrap());
        let f = service
            .connect("127.0.0.1:10000".parse().unwrap())
            .call(req);
        let response = core.run(f).unwrap();
        assert_eq!(response.status(), StatusCode::Accepted);
    }
}
