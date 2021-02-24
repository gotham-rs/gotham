//! Defines the `GothamService` type which is used to wrap a Gotham application and interface with
//! Hyper.

use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;

use futures::prelude::*;
use futures::task::{self, Poll};
use hyper::service::Service;
use hyper::{Body, Request, Response};

use crate::handler::NewHandler;
use crate::state::State;

mod trap;

pub use trap::call_handler;

/// Wraps a `NewHandler` which will be used to serve requests. Used in `gotham::os::*` to bind
/// incoming connections to `ConnectedGothamService` values.
pub(crate) struct GothamService<T>
where
    T: NewHandler + 'static,
{
    handler: Arc<T>,
}

impl<T> GothamService<T>
where
    T: NewHandler + 'static,
{
    pub(crate) fn new(handler: T) -> GothamService<T> {
        GothamService {
            handler: Arc::new(handler),
        }
    }

    pub(crate) fn connect(&self, client_addr: SocketAddr) -> ConnectedGothamService<T> {
        ConnectedGothamService {
            client_addr,
            handler: self.handler.clone(),
        }
    }
}

/// A `GothamService` which has been connected to a client. The major difference is that a
/// `client_addr` has been assigned (as this isn't available from Hyper).
pub(crate) struct ConnectedGothamService<T>
where
    T: NewHandler + 'static,
{
    handler: Arc<T>,
    client_addr: SocketAddr,
}

impl<T> Service<Request<Body>> for ConnectedGothamService<T>
where
    T: NewHandler,
{
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _cx: &mut task::Context<'_>,
    ) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call<'a>(&'a mut self, req: Request<Body>) -> Self::Future {
        let state = State::from_request(req, self.client_addr);
        call_handler(self.handler.clone(), AssertUnwindSafe(state)).boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::{Body, StatusCode};

    use crate::helpers::http::response::create_empty_response;
    use crate::router::builder::*;
    use crate::state::State;

    fn handler(state: State) -> (State, Response<Body>) {
        let res = create_empty_response(&state, StatusCode::ACCEPTED);
        (state, res)
    }

    #[test]
    fn new_handler_closure() {
        let service = GothamService::new(|| Ok(handler));

        let req = Request::get("http://localhost/")
            .body(Body::empty())
            .unwrap();
        let f = service
            .connect("127.0.0.1:10000".parse().unwrap())
            .call(req);
        let response = futures::executor::block_on(f).unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn router() {
        let router = build_simple_router(|route| {
            route.get("/").to(handler);
        });

        let service = GothamService::new(router);

        let req = Request::get("http://localhost/")
            .body(Body::empty())
            .unwrap();
        let f = service
            .connect("127.0.0.1:10000".parse().unwrap())
            .call(req);
        let response = futures::executor::block_on(f).unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }
}
