//! Defines the `GothamService` type which is used to wrap a Gotham application and interface with
//! Hyper.

use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::thread;

use futures::Future;
use hyper;
use hyper::server::Service;
use hyper::{Request, Response};

use handler::NewHandler;
use helpers::http::request::path::RequestPathSegments;
use state::client_addr::put_client_addr;
use state::{set_request_id, State};

mod timing;
mod trap;

/// Wraps a `NewHandler` which will be used to serve requests. Used in `gotham::os::*` to bind
/// incoming connections to `ConnectedGothamService` values.
pub(crate) struct GothamService<T>
where
    T: NewHandler + 'static,
{
    client_addr: Option<SocketAddr>,
    handler: Arc<T>,
}

impl<T> GothamService<T>
where
    T: NewHandler + 'static,
{
    pub(crate) fn new(handler: T) -> GothamService<T> {
        GothamService {
            client_addr: None,
            handler: Arc::new(handler)
        }
    }
    pub(crate) fn connect(&self, client_addr: SocketAddr) -> GothamService<T> {
        GothamService{client_addr : Some(client_addr), handler: self.handler.clone()}
    }
}

impl<T> Service for GothamService<T>
where
    T: NewHandler,
{
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error> + Send>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let mut state = State::new();

        put_client_addr(&mut state, self.client_addr.expect("service not connected!"));

        let (method, uri, version, headers, body) = req.deconstruct();

        state.put(RequestPathSegments::new(uri.path()));
        state.put(method);
        state.put(uri);
        state.put(version);
        state.put(headers);
        state.put(body);

        {
            let request_id = set_request_id(&mut state);
            debug!(
                "[DEBUG][{}][Thread][{:?}]",
                request_id,
                thread::current().id(),
            );
        };

        trap::call_handler(&*self.handler, AssertUnwindSafe(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::{Method, StatusCode};

    use helpers::http::response::create_response;
    use router::builder::*;
    use state::State;

    fn handler(state: State) -> (State, Response) {
        let res = create_response(&state, StatusCode::Accepted, None);
        (state, res)
    }

    #[test]
    fn new_handler_closure() {
        let service = GothamService::new(|| Ok(handler));

        let req = Request::new(Method::Get, "http://localhost/".parse().unwrap());
        let f = service
            .connect("127.0.0.1:10000".parse().unwrap())
            .call(req);
        let response = f.wait().unwrap();
        assert_eq!(response.status(), StatusCode::Accepted);
    }

    #[test]
    fn router() {
        let router = build_simple_router(|route| {
            route.get("/").to(handler);
        });

        let service = GothamService::new(router);

        let req = Request::new(Method::Get, "http://localhost/".parse().unwrap());
        let f = service
            .connect("127.0.0.1:10000".parse().unwrap())
            .call(req);
        let response = f.wait().unwrap();
        assert_eq!(response.status(), StatusCode::Accepted);
    }
}
