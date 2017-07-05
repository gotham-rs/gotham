//! Defines a `Router` and supporting types.

pub mod tree;
pub mod route;
pub mod request_matcher;
pub mod response_extender;

use std::io;
use std::sync::Arc;

use futures::{future, Future};
use hyper::{Request, Response, Headers, StatusCode, Uri, HttpVersion, Method};

use handler::{NewHandler, Handler, HandlerFuture};
use http::query_string;
use http::request_path::RequestPathSegments;
use router::response_extender::ResponseExtender;
use router::route::Route;
use router::tree::{SegmentMapping, Tree};
use state::{State, StateData, request_id};

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
/// The `Router` is capable of delegating `Requests` to secondary `Router` instances which allows it
/// to support "Umbrella Applications". An umbrella application contains multiple
/// applications that are run together but have clear boundaries between them, via module
/// seperation. Umbrella applications live within a single repository. This style of application
/// is roughly a halfway point between monolithic application design and
/// microservice application design. Umbrella Applications may also share modules.
/// e.g. Authentication/Authorization/Identity.
///
/// Please see the documentation for `Route` in order to create routes that delegate to secondary
/// `Routers`.
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
                    match leaf.select_route(&state, &req) {
                        Ok(route) => {
                            if route.is_delegating() {
                                trace!("[{}] delegating to secondary router", request_id(&state));

                                let mut rps = rps.clone();
                                rps.increase_offset(sm.len());
                                state.put(rps);

                                route.dispatch(state, req)
                            } else {
                                trace!("[{}] dispatching to route", request_id(&state));
                                self.dispatch(state, req, sm, route)
                            }
                        }
                        Err(status) => {
                            trace!("[{}] responding with error status", request_id(&state));
                            self.generate_response(status, state)
                        }
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
    use hyper::{Error, Request, Method, Uri, Body};
    use hyper::header::ContentLength;

    use router::tree::TreeBuilder;
    use router::tree::node::{SegmentType, NodeBuilder};
    use router::route::{RouteImpl, Extractors};
    use http::request_path::NoopRequestPathExtractor;
    use http::query_string::NoopQueryStringExtractor;
    use dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
    use router::request_matcher::MethodOnlyRequestMatcher;
    use router::response_extender::ResponseExtenderBuilder;
    use state::set_request_id;

    fn handler(state: State, _req: Request) -> (State, Response) {
        (state, Response::new())
    }

    fn send_request(r: Router, m: Method, uri: &str) -> Result<(State, Response), (State, Error)> {
        let uri = Uri::from_str(uri).unwrap();
        let request: Request<Body> = Request::new(m, uri);

        let mut state = State::new();
        set_request_id(&mut state, &request);
        state.put(RequestPathSegments::new(request.uri().path().clone()));

        r.handle(state, request).wait()
    }

    #[test]
    fn internal_server_error_if_no_request_path_segments() {
        let tree_builder = TreeBuilder::new();
        let tree = tree_builder.finalize();
        let router = Router::new(tree, ResponseExtenderBuilder::new().finalize());

        let method = Method::Get;
        let uri = Uri::from_str("https://test.gotham.rs").unwrap();
        let request: Request<Body> = Request::new(method, uri);

        let mut state = State::new();
        set_request_id(&mut state, &request);

        match router.handle(state, request).wait() {
            Ok((_state, res)) => {
                assert_eq!(res.status(), StatusCode::InternalServerError);
            }
            Err(_) => panic!("Router should have handled request"),
        };
    }

    #[test]
    fn not_found_error_if_request_path_is_not_found() {
        let tree_builder = TreeBuilder::new();
        let tree = tree_builder.finalize();
        let router = Router::new(tree, ResponseExtenderBuilder::new().finalize());

        match send_request(router, Method::Get, "https://test.gotham.rs") {
            Ok((_state, res)) => {
                assert_eq!(res.status(), StatusCode::NotFound);
            }
            Err(_) => panic!("Router should have handled request"),
        };
    }

    #[test]
    fn custom_error_if_leaf_found_but_matching_route_not_found() {
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut tree_builder = TreeBuilder::new();

        let route = {
            let methods = vec![Method::Post];
            let matcher = MethodOnlyRequestMatcher::new(methods);
            let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
            let extractors: Extractors<NoopRequestPathExtractor,
                                       NoopQueryStringExtractor> = Extractors::new();
            let route = RouteImpl::new(matcher, dispatcher, extractors, false);
            Box::new(route)
        };
        tree_builder.add_route(route);
        let tree = tree_builder.finalize();
        let router = Router::new(tree, ResponseExtenderBuilder::new().finalize());

        match send_request(router, Method::Get, "https://test.gotham.rs") {
            Ok((_state, res)) => {
                assert_eq!(res.status(), StatusCode::MethodNotAllowed);
            }
            Err(_) => panic!("Router should have handled request"),
        };
    }

    #[test]
    fn success_if_leaf_and_route_found() {
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut tree_builder = TreeBuilder::new();

        let route = {
            let methods = vec![Method::Get];
            let matcher = MethodOnlyRequestMatcher::new(methods);
            let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
            let extractors: Extractors<NoopRequestPathExtractor,
                                       NoopQueryStringExtractor> = Extractors::new();
            let route = RouteImpl::new(matcher, dispatcher, extractors, false);
            Box::new(route)
        };
        tree_builder.add_route(route);
        let tree = tree_builder.finalize();
        let router = Router::new(tree, ResponseExtenderBuilder::new().finalize());

        match send_request(router, Method::Get, "https://test.gotham.rs") {
            Ok((_state, res)) => {
                assert_eq!(res.status(), StatusCode::Ok);
            }
            Err(_) => panic!("Router should have handled request"),
        };
    }

    #[test]
    fn delegates_to_secondary_router() {
        let delegated_router = {
            let pipeline_set = finalize_pipeline_set(new_pipeline_set());
            let mut tree_builder = TreeBuilder::new();

            let route = {
                let methods = vec![Method::Get];
                let matcher = MethodOnlyRequestMatcher::new(methods);
                let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
                let extractors: Extractors<NoopRequestPathExtractor,
                                           NoopQueryStringExtractor> = Extractors::new();
                let route = RouteImpl::new(matcher, dispatcher, extractors, false);
                Box::new(route)
            };
            tree_builder.add_route(route);

            let tree = tree_builder.finalize();
            Router::new(tree, ResponseExtenderBuilder::new().finalize())
        };

        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut tree_builder = TreeBuilder::new();
        let mut delegated_node = NodeBuilder::new("var", SegmentType::Dynamic);

        let route = {
            let methods = vec![Method::Get];
            let matcher = MethodOnlyRequestMatcher::new(methods);
            let dispatcher = Box::new(DispatcherImpl::new(delegated_router, (), pipeline_set));
            let extractors: Extractors<NoopRequestPathExtractor,
                                       NoopQueryStringExtractor> = Extractors::new();
            let route = RouteImpl::new(matcher, dispatcher, extractors, true);
            Box::new(route)
        };

        delegated_node.add_route(route);
        tree_builder.add_child(delegated_node);
        let tree = tree_builder.finalize();
        let router = Router::new(tree, ResponseExtenderBuilder::new().finalize());

        // Ensure that top level tree has no route
        match send_request(router.clone(), Method::Get, "https://test.gotham.rs") {
            Ok((_state, res)) => {
                assert_eq!(res.status(), StatusCode::NotFound);
            }
            Err(_) => panic!("Router should have handled request"),
        };

        // Ensure that top level tree of delegated router has route that responds correctly
        match send_request(router, Method::Get, "https://test.gotham.rs/api") {
            Ok((_state, res)) => {
                assert_eq!(res.status(), StatusCode::Ok);
            }
            Err(_) => panic!("Router should have handled request"),
        };
    }

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

        match send_request(router, Method::Get, "https://test.gotham.rs/api") {
            Ok((_state, res)) => {
                assert_eq!(*res.headers().get::<ContentLength>().unwrap(),
                           ContentLength(3u64));
            }
            Err(_) => panic!("Router should have correctly handled request"),
        };
    }
}
