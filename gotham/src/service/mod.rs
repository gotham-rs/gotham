//! Defines the `GothamService` type which is used to wrap a Gotham application and interface with
//! Hyper.

use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::thread;

use futures::Future;
use hyper;
use hyper::service::Service;
use hyper::{Body, Request, Response};

use handler::NewHandler;
use helpers::http::request::path::RequestPathSegments;
use state::client_addr::put_client_addr;
use state::{request_id, set_request_id, State};

mod timing;
mod trap;

/// Wraps a `NewHandler` which will be used to serve requests. Used in `gotham::os::*` to bind
/// incoming connections to `ConnectedGothamService` values.
pub(crate) struct GothamService<T, B>
where
    T: NewHandler<B> + 'static,
{
    t: Arc<T>,
}

impl<T, B> GothamService<T, B>
where
    T: NewHandler<B> + 'static,
{
    pub(crate) fn new(t: Arc<T>) -> GothamService<T, B> {
        GothamService { t }
    }

    pub(crate) fn connect(&self, client_addr: SocketAddr) -> ConnectedGothamService<T, B> {
        ConnectedGothamService {
            t: self.t.clone(),
            client_addr,
        }
    }
}

/// A `GothamService` which has been connected to a client. The major difference is that a
/// `client_addr` has been assigned (as this isn't available from Hyper).
pub(crate) struct ConnectedGothamService<T, B>
where
    T: NewHandler<B> + 'static,
{
    t: Arc<T>,
    client_addr: SocketAddr,
}

impl<T, B> Service for ConnectedGothamService<T, B>
where
    T: NewHandler<B>,
{
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response<Self::ResBody>, Error = Self::Error>>;

    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        let mut state = State::new();

        put_client_addr(&mut state, self.client_addr);

        let (method, uri, version, headers, body) = req.deconstruct();

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

    use helpers::http::response::create_response;
    use router::builder::*;
    use state::State;

    fn handler(state: State) -> (State, Response<()>) {
        let res = create_response(&state, StatusCode::ACCEPTED, None);
        (state, res)
    }

    #[test]
    fn new_handler_closure() {
        let service = GothamService::new(Arc::new(|| Ok(handler)));

        let req = Request::new(Method::GET, "http://localhost/".parse().unwrap());
        let f = service
            .connect("127.0.0.1:10000".parse().unwrap())
            .call(req);
        let response = f.wait().unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn router() {
        let router = build_simple_router(|route| {
            route.get("/").to(handler);
        });

        let service = GothamService::new(Arc::new(router));

        let req = Request::new(Method::GET, "http://localhost/".parse().unwrap());
        let f = service
            .connect("127.0.0.1:10000".parse().unwrap())
            .call(req);
        let response = f.wait().unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }
}
