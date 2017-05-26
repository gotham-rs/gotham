//! Defines a `Router` and supporting types.

pub mod tree;
pub mod route;
pub mod request_matcher;

use std::io;
use std::sync::Arc;

use borrow_bag::BorrowBag;
use futures::{future, Future};
use hyper::server::Request;

use handler::{NewHandler, Handler, HandlerFuture};
use router::tree::Tree;
use state::State;
use http::split_request_path;

// Holds data for Router which lives behind single Arc instance
// so that otherwise non Clone-able structs are able to be used via NewHandler
struct RouterData<'n, P, NFH, ISEH>
    where NFH: NewHandler,
          ISEH: NewHandler
{
    tree: Tree<'n, P>,
    pipelines: BorrowBag<P>,
    not_found_handler: NFH,
    internal_server_error_handler: ISEH,
}

impl<'n, P, NFH, ISEH> RouterData<'n, P, NFH, ISEH>
    where NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    pub fn new(tree: Tree<'n, P>,
               pipelines: BorrowBag<P>,
               not_found_handler: NFH,
               internal_server_error_handler: ISEH)
               -> RouterData<'n, P, NFH, ISEH> {
        RouterData {
            tree,
            pipelines,
            not_found_handler,
            internal_server_error_handler,
        }
    }
}

/// Responsible for dispatching `Requests` to a linked `Route` and
/// dispatching error states when a valid `Route` is unable to be determined.
///
/// # Examples
///
/// ```
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate borrow_bag;
/// #
/// # use hyper::server::{Request, Response};
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::Router;
/// # use gotham::state::State;
/// #
/// # fn handler(state: State, _req: Request) -> (State, Response) {
/// #   (state, Response::new())
/// # }
/// #
/// # fn handler2(state: State, _req: Request) -> (State, Response) {
/// #   (state, Response::new())
/// # }
/// #
/// # fn main() {
///   let tree_builder = TreeBuilder::new();
///   let tree = tree_builder.finalize();
///   let not_found = || Ok(handler);
///   let internal_server_error = || Ok(handler2);
///   let pipelines = borrow_bag::new_borrow_bag();
///
///   Router::new(tree, pipelines, not_found, internal_server_error);
/// # }
/// ```
pub struct Router<'n, P, NFH, ISEH>
    where P: Sync,
          NFH: NewHandler,
          ISEH: NewHandler
{
    data: Arc<RouterData<'n, P, NFH, ISEH>>,
}

impl<'n, P, NFH, ISEH> Router<'n, P, NFH, ISEH>
    where P: Sync,
          NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    /// Creates a new `Router` instance, internal `Tree` and establishes global error
    /// handlers for NotFound and InternalServerError responses.
    pub fn new(tree: Tree<'n, P>,
               pipelines: BorrowBag<P>,
               not_found_handler: NFH,
               internal_server_error_handler: ISEH)
               -> Router<'n, P, NFH, ISEH> {

        let router_data = RouterData::new(tree,
                                          pipelines,
                                          not_found_handler,
                                          internal_server_error_handler);
        Router { data: Arc::new(router_data) }
    }

    // Attempt to respond to client with a 404 NotFound response
    fn not_found(&self, state: State, req: Request) -> Box<HandlerFuture> {
        match self.data.not_found_handler.new_handler() {
            Ok(handler) => handler.handle(state, req),
            Err(_error) => self.internal_server_error(state, req),
        }
    }

    // Attempt to respond to client with a 500 InternalServerError response.
    //
    // Failing this all we have left is to fall back to generic future error within Tokio as we've
    // exhausted all options.
    //
    // TODO: Ensure all future errors are appropriately logged.
    fn internal_server_error(&self, state: State, req: Request) -> Box<HandlerFuture> {
        match self.data.internal_server_error_handler.new_handler() {
            Ok(handler) => handler.handle(state, req),
            Err(error) => future::err((state, error.into())).boxed(),
        }
    }
}

impl<'n, P, NFH, ISEH> Clone for Router<'n, P, NFH, ISEH>
    where P: Sync,
          NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    fn clone(&self) -> Router<'n, P, NFH, ISEH> {
        Router { data: self.data.clone() }
    }
}

impl<'n, P, NFH, ISEH> NewHandler for Router<'n, P, NFH, ISEH>
    where P: Send + Sync,
          NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    type Instance = Router<'n, P, NFH, ISEH>;

    // Creates a new Router instance to route new HTTP requests
    fn new_handler(&self) -> io::Result<Self::Instance> {
        Ok((*self).clone())
    }
}

impl<'n, P, NFH, ISEH> Handler for Router<'n, P, NFH, ISEH>
    where P: Send + Sync,
          NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    /// Handles the request by determining the correct `Route` from the internal
    /// `Tree`, storing any path related variables in `State` and dispatching
    /// appropriately to the configured `Handler`.
    ///
    /// # Errors
    ///
    /// If no `Route` is present a 404 `NotFound` response will be returned to the client
    /// via `not_found_handler`.
    ///
    /// If an unexpected error occurs handling the request a 500 `InternalServerError` response
    /// will be returned to client via `internal_server_error_handler`.
    ///
    /// For unrecoverable error states `future::err` will be called, dropping the
    /// connection to the client without response.
    fn handle(&self, mut state: State, req: Request) -> Box<HandlerFuture> {
        let uri = req.uri().clone();
        match split_request_path(uri.path()) {
            Some(rp) => {
                match self.data.tree.traverse(rp.as_slice()) {
                    Some((tree_path, segment_mapping)) => {
                        if let Some(leaf) = tree_path.last() {
                            match leaf.borrow_routes().iter().find(|r| r.is_match(&req)) {
                                Some(route) => {
                                    match route.extract_request_path(&mut state, segment_mapping) {
                                        Ok(()) => {
                                    // TODO Extract Query Params
                                    // TODO Extract Body
                                    route.dispatch(&self.data.pipelines, state, req)
                                }
                                        Err(_) => self.internal_server_error(state, req),
                                    }
                                }
                                None => self.internal_server_error(state, req),
                            }
                        } else {
                            self.internal_server_error(state, req)
                        }
                    }
                    None => self.not_found(state, req),
                }
            }
            None => self.internal_server_error(state, req),
        }
    }
}
