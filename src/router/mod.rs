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
use http::{request_path, query_string};
use router::response_extender::ResponseExtender;
use router::route::Route;
use router::tree::{SegmentMapping, Tree};
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

        let uri = req.uri().clone();
        self.populate_state(&mut state, &req);

        let response = self.route(uri, state, req);
        self.finalize_response(response)
    }
}

impl Router {
    /// Creates a `Router` instance.
    pub fn new(tree: Tree, response_extender: ResponseExtender) -> Router {

        let router_data = RouterData::new(tree, response_extender);
        Router { data: Arc::new(router_data) }
    }

    fn populate_state(&self, state: &mut State, req: &Request) {
        trace!("[{}] populating immutable request data into state",
               request_id(&state));
        state.put(req.method().clone());
        state.put(req.uri().clone());
        state.put(req.version().clone());
        state.put(req.headers().clone());
    }

    fn route(&self, uri: Uri, state: State, req: Request) -> Box<HandlerFuture> {
        trace!("[{}] attempting to route: {}",
               request_id(&state),
               uri.path());

        let rp = request_path::RequestPathSegments::new(uri.path());
        if let Some((_, leaf, segment_mapping)) =
            self.data.tree.traverse(rp.segments().as_slice()) {
            // Valid path for the application, determine if any configured
            // routes will accept the request
            if let Some(route) = leaf.borrow_routes()
                   .iter()
                   .find(|r| r.is_match(&state, &req).is_ok()) {
                trace!("[{}] starting dispatch for matched route",
                       request_id(&state));
                self.dispatch(state, req, &uri, segment_mapping, route)
            } else {
                // No routes accepted the request, the error status associated with the first route
                // is then chosen as the status code for the response.
                trace!("[{}] no routes accepted the request", request_id(&state));
                let status = leaf.borrow_routes().first().unwrap().is_match(&state, &req);
                self.generate_response(status.unwrap_err(), state)
            }
        } else {
            trace!("[{}] did not find routable leaf node", request_id(&state));
            self.generate_response(StatusCode::NotFound, state)
        }
    }

    fn dispatch(&self,
                mut state: State,
                req: Request,
                uri: &Uri,
                segment_mapping: SegmentMapping,
                route: &Box<Route + Send + Sync>)
                -> Box<HandlerFuture> {
        match route.extract_request_path(&mut state, segment_mapping) {
            Ok(()) => {
                trace!("[{}] extracted request path", request_id(&state));
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
    use hyper::header::{ContentType, ContentLength};

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
        let result = router.handle(state, request).wait();

        match result {
            Ok((_state, res)) => {
                assert_eq!(*res.headers().get::<ContentLength>().unwrap(),
                           ContentLength(3u64));
            }
            Err(_) => panic!("Router should have correctly handled request"),
        };
    }

    #[test]
    fn populates_core_request_data_into_state() {
        let tree_builder = TreeBuilder::new();
        let tree = tree_builder.finalize();
        let response_extender = ResponseExtenderBuilder::new().finalize();

        let router = Router::new(tree, response_extender);
        let method = Method::Get;
        let uri = Uri::from_str("https://test.gotham.rs").unwrap();
        let version = HttpVersion::H2;
        let mut request: Request<Body> = Request::new(method.clone(), uri.clone());
        request.set_version(version.clone());
        request.headers_mut().set(ContentType::json());

        let mut state = State::new();
        set_request_id(&mut state, &request);
        let result = router.handle(state, request).wait();

        match result {
            Ok((state, _res)) => {
                assert_eq!(*state.borrow::<Method>().unwrap(), method);
                assert_eq!(*state.borrow::<Uri>().unwrap(), uri);
                assert_eq!(*state.borrow::<HttpVersion>().unwrap(), version);
                assert_eq!(*state
                                .borrow::<Headers>()
                                .unwrap()
                                .get::<ContentType>()
                                .unwrap(),
                           ContentType::json());
            }
            Err(_) => panic!("Router should have correctly handled request"),
        };
    }
}
