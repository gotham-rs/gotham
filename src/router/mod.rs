//! Defines Gotham's `Router` and supporting types.

pub mod tree;
pub mod route;
pub mod request_matcher;

use std::io;
use std::sync::Arc;

use futures::{future, Future};
use hyper::server::Request;

use router::tree::Tree;
use handler::{NewHandler, Handler, HandlerFuture};
use state::State;

// Holds data for Router which lives behind single Arc instance
// so that otherwise non Clone-able structs are able to be used via NewHandler
struct RouterData<'n, NFH, ISEH>
    where NFH: NewHandler,
          ISEH: NewHandler
{
    tree: Tree<'n>,
    not_found_handler: NFH,
    internal_server_error_handler: ISEH,
}

impl<'n, NFH, ISEH> RouterData<'n, NFH, ISEH>
    where NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    pub fn new(tree: Tree<'n>,
               not_found_handler: NFH,
               internal_server_error_handler: ISEH)
               -> RouterData<'n, NFH, ISEH> {
        RouterData {
            tree: tree,
            not_found_handler,
            internal_server_error_handler,
        }
    }
}

/// Responsible for dispatching [`Requests`][request] to a linked [`Route`][route] and
/// dispatching error states when a valid [`Route`][route] is unable to be determined.
///
/// # Examples
///
/// ```
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::server::{Request, Response};
/// # use gotham::router::tree::Tree;
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
///   let tree = Tree::new();
///   let not_found = || Ok(handler);
///   let internal_server_error = || Ok(handler2);
///
///   Router::new(tree, not_found, internal_server_error);
/// # }
/// ```
///
/// [request]: ../../hyper/server/struct.Request.html
/// [route]: route/trait.Route.html
pub struct Router<'n, NFH, ISEH>
    where NFH: NewHandler,
          ISEH: NewHandler
{
    data: Arc<RouterData<'n, NFH, ISEH>>,
}

impl<'n, NFH, ISEH> Router<'n, NFH, ISEH>
    where NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    /// Creates a new `Router` instance, internal [`Tree`][tree] and establishes global error
    /// handlers for NotFound and InternalServerError responses.
    ///
    /// [tree]: tree/struct.Tree.html
    pub fn new(tree: Tree<'n>,
               not_found_handler: NFH,
               internal_server_error_handler: ISEH)
               -> Router<'n, NFH, ISEH> {

        let router_data = RouterData::new(tree, not_found_handler, internal_server_error_handler);
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

impl<'n, NFH, ISEH> Clone for Router<'n, NFH, ISEH>
    where NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    fn clone(&self) -> Router<'n, NFH, ISEH> {
        Router { data: self.data.clone() }
    }
}

impl<'n, NFH, ISEH> NewHandler for Router<'n, NFH, ISEH>
    where NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    type Instance = Router<'n, NFH, ISEH>;

    // Creates a new Router instance to route new HTTP requests
    fn new_handler(&self) -> io::Result<Self::Instance> {
        Ok((*self).clone())
    }
}

impl<'n, NFH, ISEH> Handler for Router<'n, NFH, ISEH>
    where NFH: NewHandler,
          NFH::Instance: 'static,
          ISEH: NewHandler,
          ISEH::Instance: 'static
{
    /// Handles the request by determining the correct [`Route`][route] from the internal
    /// [`Tree`][tree], storing any path related variables in [`State`][state] and dispatching
    /// appropriately to the configured [`Handler`][handler].
    ///
    /// # Errors
    ///
    /// If no [`Route`][route] is present a 404 `NotFound` response will be returned to the client
    /// via `not_found_handler`.
    ///
    /// If an unexpected error occurs handling the request a 500 `InternalServerError` response
    /// will be returned to client via `internal_server_error_handler`.
    ///
    /// For unrecoverable error states [`future::err`][future-err] will be called, dropping the
    /// connection to the client without response.
    ///
    /// [route]: route/trait.Route.html
    /// [state]: ../state/struct.State.html
    /// [handler]: ../handler/trait.Handler.html
    /// [tree]: tree/struct.Tree.html
    /// [future-err]: ../../futures/future/fn.err.html
    fn handle(&self, state: State, req: Request) -> Box<HandlerFuture> {
        match self.data.tree.traverse(req.path()) {
            Some(tree_path) => {
                // TODO: populate path variables

                // acquire leaf and routes
                if let Some(leaf) = tree_path.last() {
                    // dispatch
                    match leaf.borrow_routes().iter().find(|r| r.is_match(&req)) {
                        Some(route) => route.dispatch(state, req),
                        None => self.internal_server_error(state, req),
                    }
                } else {
                    self.internal_server_error(state, req)
                }
            }
            None => self.not_found(state, req),
        }
    }
}
