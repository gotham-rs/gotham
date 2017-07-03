//! Defines a `Router` and supporting types.

pub mod tree;
pub mod route;
pub mod request_matcher;
pub mod response_extender;

use std::io;
use std::sync::Arc;

use futures::{future, Future};
use hyper::{Headers, StatusCode, Uri, HttpVersion, Method};
use hyper::server::{Request, Response};

use handler::{NewHandler, Handler, HandlerFuture};
use http::query_string;
use http::request_path::RequestPathSegments;
use router::response_extender::ResponseExtender;
use router::route::Route;
use router::tree::{SegmentMapping, Tree};
use router::tree::node::{Node, NodeSegmentType};
use state::{State, StateData, request_id};

// Holds data for Router which lives behind single Arc instance
// so that otherwise non Clone-able structs are able to be used via NewHandler
struct RouterData {
    tree: Tree,
    response_extender: ResponseExtender,
}

impl RouterData {
    pub fn new(tree: Tree, response_extender: ResponseExtender) -> RouterData {
        RouterData {
            tree,
            response_extender,
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
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::Router;
/// # use gotham::router::response_extender::ResponseExtenderBuilder;
/// #
/// # fn main() {
///   let tree_builder = TreeBuilder::new();
///   let tree = tree_builder.finalize();
///   let response_extender = ResponseExtenderBuilder::new().finalize();
///
///   Router::new(tree, response_extender);
/// # }
/// ```
#[derive(Clone)]
pub struct Router {
    data: Arc<RouterData>,
}

impl NewHandler for Router {
    type Instance = Router;

    // Creates a new Router instance to route new HTTP requests
    fn new_handler(&self) -> io::Result<Self::Instance> {
        trace!(" cloning instance");
        Ok((*self).clone())
    }
}

impl Handler for Router {
    /// Handles the request by determining the correct `Route` from the internal
    /// `Tree`, storing any path related variables in `State` and dispatching
    /// appropriately to the configured `Handler`.
    fn handle(self, mut state: State, req: Request) -> Box<HandlerFuture> {
        trace!("[{}] starting", request_id(&state));

        let response = match state.take::<RequestPathSegments>() {
            Some(rps) => {
                if let Some((_, leaf, sm)) = self.data.tree.traverse(&rps.segments()) {
                    match leaf.segment_type() {
                        &NodeSegmentType::Delegator => {
                            self.delegate_request(rps.clone(), leaf, sm, state, req)
                        }
                        _ => self.process_request(leaf, sm, state, req),
                    }
                } else {
                    trace!("[{}] did not find routable node", request_id(&state));
                    self.generate_response(StatusCode::NotFound, state)
                }
            }
            None => {
                trace!("[{}] invalid request path segments", request_id(&state));
                self.generate_response(StatusCode::InternalServerError, state)
            }
        };

        self.finalize_response(response)
    }
}

impl Router {
    /// Creates a `Router` instance.
    pub fn new(tree: Tree, response_extender: ResponseExtender) -> Router {

        let router_data = RouterData::new(tree, response_extender);
        Router { data: Arc::new(router_data) }
    }

    fn process_request(&self,
                       leaf: &Node,
                       sm: SegmentMapping,
                       state: State,
                       req: Request)
                       -> Box<HandlerFuture> {
        if let Some(route) = self.find_matching_route(leaf, &state, &req) {
            trace!("[{}] dispatching to route", request_id(&state));
            self.dispatch(state, req, sm, route)
        } else {
            self.dispatch_error(leaf, state, req)
        }
    }

    fn delegate_request(&self,
                        mut rps: RequestPathSegments,
                        leaf: &Node,
                        sm: SegmentMapping,
                        mut state: State,
                        req: Request)
                        -> Box<HandlerFuture> {
        trace!("[{}] attempting to delegate request", request_id(&state));
        trace!("[{}] segment_mapping: [{:?}]", request_id(&state), sm);

        if let Some(route) = leaf.borrow_routes().first() {
            trace!("[{}] delegating to secondary router", request_id(&state));

            rps.increase_offset(sm.len());
            state.put(rps);

            route.dispatch(state, req)
        } else {
            trace!("[{}] did not find routable delegator node",
                   request_id(&state));
            self.generate_response(StatusCode::NotFound, state)
        }
    }

    fn find_matching_route<'a>(&self,
                               leaf: &'a Node,
                               state: &State,
                               req: &Request)
                               -> Option<&'a Box<Route + Send + Sync>> {
        leaf.borrow_routes()
            .iter()
            .find(|r| r.is_match(&state, &req).is_ok())
    }

    fn dispatch(&self,
                mut state: State,
                req: Request,
                sm: SegmentMapping,
                route: &Box<Route + Send + Sync>)
                -> Box<HandlerFuture> {
        match route.extract_request_path(&mut state, sm) {
            Ok(()) => {
                trace!("[{}] extracted request path", request_id(&state));

                let uri = req.uri().clone();
                if let Some(q) = uri.query() {
                    match route.extract_query_string(&mut state, query_string::split(q)) {
                        Ok(()) => {
                            trace!("[{}] extracted query string", request_id(&state));
                            trace!("[{}] dispatching", request_id(&state));
                            return route.dispatch(state, req)
                        }
                        Err(_) => (),
                    }
                } else {
                    trace!("[{}] dispatching", request_id(&state));
                    return route.dispatch(state, req);
                }
            }
            Err(_) => (),
        };

        let mut res = Response::new();
        res.set_status(StatusCode::InternalServerError);
        error!("[{}] internal server error, failed to dispatch",
               request_id(&state));
        future::ok((state, res)).boxed()
    }

    fn dispatch_error(&self, leaf: &Node, state: State, req: Request) -> Box<HandlerFuture> {
        // No routes accepted the request, the error status associated with the first route
        // is then chosen as the status code for the response.
        trace!("[{}] no routes accepted the request", request_id(&state));
        if let Some(route) = leaf.borrow_routes().first() {
            let status = route.is_match(&state, &req);
            trace!("[{}] responding with error status", request_id(&state));
            self.generate_response(status.unwrap_err(), state)
        } else {
            trace!("[{}] no routes, default error response", request_id(&state));
            self.generate_response(StatusCode::InternalServerError, state)
        }
    }

    fn generate_response(&self, status: StatusCode, state: State) -> Box<HandlerFuture> {
        trace!("[{}][{}] generating response", request_id(&state), status);
        let mut res = Response::new();
        res.set_status(status);
        future::ok((state, res)).boxed()
    }

    fn finalize_response(&self, result: Box<HandlerFuture>) -> Box<HandlerFuture> {
        let response_extender = self.data.response_extender.clone();
        result
            .and_then(move |(state, res)| {
                          trace!("[{}] handler complete", request_id(&state));
                          response_extender.extend(state, res)
                      })
            .boxed()
    }
}

impl StateData for Method {}
impl StateData for Uri {}
impl StateData for HttpVersion {}
impl StateData for Headers {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use hyper::{Request, Method, Uri, Body};
    use hyper::header::ContentLength;

    use router::tree::TreeBuilder;
    use router::response_extender::ResponseExtenderBuilder;
    use state::set_request_id;

    #[test]
    fn executes_response_extender_when_present() {
        let tree_builder = TreeBuilder::new();
        let tree = tree_builder.finalize();

        let mut response_extender_builder = ResponseExtenderBuilder::new();
        let not_found_extender = |s, mut r: Response| {
            r.headers_mut().set(ContentLength(3u64));
            future::ok((s, r)).boxed()
        };
        response_extender_builder.add(StatusCode::NotFound, Box::new(not_found_extender));
        let response_extender = response_extender_builder.finalize();

        let router = Router::new(tree, response_extender);
        let method = Method::Get;
        let uri = Uri::from_str("https://test.gotham.rs").unwrap();
        let request: Request<Body> = Request::new(method, uri);

        let mut state = State::new();
        set_request_id(&mut state, &request);
        state.put(RequestPathSegments::new(request.uri().path().clone()));

        let result = router.handle(state, request).wait();

        match result {
            Ok((_state, res)) => {
                assert_eq!(*res.headers().get::<ContentLength>().unwrap(),
                           ContentLength(3u64));
            }
            Err(_) => panic!("Router should have correctly handled request"),
        };
    }
}
