//! Defines the `Service` which is used by a Gotham application to interface to Hyper.

use std::{io, thread};
use std::net::SocketAddr;
use std::sync::Arc;
use std::panic::AssertUnwindSafe;

use hyper;
use hyper::server::{NewService, Service};
use hyper::{Request, Response};
use futures::Future;
use tokio_core::reactor::Handle;

use handler::NewHandler;
use state::{request_id, set_request_id, State};
use state::client_addr::put_client_addr;
use http::request::path::RequestPathSegments;

mod timing;
mod trap;

/// Wraps a `NewHandler` to provide a `hyper::server::NewService` implementation for Gotham
/// handlers.
pub struct GothamService<T>
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
    /// Creates a `GothamService` for the given `NewHandler`.
    ///
    /// # Examples
    ///
    /// Using a closure which is a `NewHandler`:
    ///
    /// ```rust,no_run
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate tokio_core;
    /// #
    /// # use std::sync::Arc;
    /// # use gotham::http::response::create_response;
    /// # use gotham::service::GothamService;
    /// # use gotham::state::State;
    /// # use hyper::Response;
    /// # use hyper::StatusCode;
    /// # use tokio_core::reactor::Core;
    /// #
    /// # fn main() {
    /// fn handler(state: State) -> (State, Response) {
    ///     let res = create_response(&state, StatusCode::Accepted, None);
    ///     (state, res)
    /// }
    ///
    /// let core = Core::new().unwrap(); // tokio-core
    ///
    /// GothamService::new(Arc::new(|| Ok(handler)), core.handle());
    /// # }
    /// ```
    ///
    /// Using a `Router`:
    ///
    /// ```rust,no_run
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate tokio_core;
    /// #
    /// # use std::sync::Arc;
    /// # use gotham::http::response::create_response;
    /// # use gotham::service::GothamService;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::tree::TreeBuilder;
    /// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
    /// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
    /// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
    /// # use gotham::router::request::path::NoopPathExtractor;
    /// # use gotham::router::request::query_string::NoopQueryStringExtractor;
    /// # use gotham::router::response::finalizer::ResponseFinalizerBuilder;
    /// # use hyper::Response;
    /// # use hyper::{StatusCode, Method};
    /// # use tokio_core::reactor::Core;
    /// #
    /// # fn main() {
    /// fn handler(state: State) -> (State, Response) {
    ///     let res = create_response(&state, StatusCode::Accepted, None);
    ///     (state, res)
    /// }
    ///
    /// let mut tree_builder = TreeBuilder::new();
    /// let pipeline_set = finalize_pipeline_set(new_pipeline_set());
    /// let finalizer = ResponseFinalizerBuilder::new().finalize();
    ///
    /// let matcher = MethodOnlyRouteMatcher::new(vec![Method::Get]);
    /// let dispatcher = DispatcherImpl::new(|| Ok(handler), (), pipeline_set);
    /// let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
    /// let route = RouteImpl::new(matcher, Box::new(dispatcher), extractors, Delegation::Internal);
    ///
    /// tree_builder.add_route(Box::new(route));
    /// let tree = tree_builder.finalize();
    /// let router = Router::new(tree, finalizer);
    ///
    /// let core = Core::new().unwrap(); // tokio-core
    ///
    /// GothamService::new(Arc::new(router), core.handle());
    /// # }
    /// ```
    pub fn new(t: Arc<T>, handle: Handle) -> GothamService<T> {
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

pub(crate) struct ConnectedGothamService<T>
where
    T: NewHandler + 'static,
{
    t: Arc<T>,
    handle: Handle,
    client_addr: SocketAddr,
}

impl<T> Clone for ConnectedGothamService<T>
where
    T: NewHandler + 'static,
{
    fn clone(&self) -> Self {
        ConnectedGothamService {
            t: self.t.clone(),
            handle: self.handle.clone(),
            client_addr: self.client_addr,
        }
    }
}

impl<T> NewService for ConnectedGothamService<T>
where
    T: NewHandler + 'static,
{
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = Self;

    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
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
