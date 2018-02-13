//! Defines the Gotham `Router` and supporting types.

pub mod builder;
pub mod tree;
pub mod route;
pub mod response;
pub mod non_match;

use std::io;
use std::sync::Arc;

use futures::{future, Future};
use hyper::{Response, StatusCode};
use hyper::header::Allow;

use handler::{Handler, HandlerFuture, IntoResponse, NewHandler};
use http::request::path::RequestPathSegments;
use http::response::create_response;
use router::response::finalizer::ResponseFinalizer;
use router::route::{Delegation, Route};
use router::tree::{SegmentMapping, Tree};
use state::{request_id, State};

struct RouterData {
    tree: Tree,
    response_finalizer: ResponseFinalizer,
}

impl RouterData {
    pub fn new(tree: Tree, response_finalizer: ResponseFinalizer) -> RouterData {
        RouterData {
            tree,
            response_finalizer,
        }
    }
}

/// Responsible for dispatching `Requests` to a linked `Route` and
/// dispatching error states when a valid `Route` is unable to be determined or internal error
/// states occur.
///
/// The `Router` is capable of delegating `Requests` to secondary `Router` instances which allows it
/// to support "Modular Applications". A modular application contains multiple
/// applications within a single binary but have clear boundaries between them, via Rust module
/// seperation. Modular applications live within a single repository. Modular applications
/// are roughly a halfway point between monolithic application design and
/// microservice application design. Modular Applications may share modules that are not
/// specifically asigned to any one application e.g. Authentication/Authorization/Identity.
///
/// Please see the documentation for `Route` in order to create routes that delegate to secondary
/// `Routers`.
///
/// # Examples
///
/// ```
/// # extern crate gotham;
/// #
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::Router;
/// # use gotham::router::response::finalizer::ResponseFinalizerBuilder;
/// #
/// # fn main() {
///   let tree_builder = TreeBuilder::new();
///   let tree = tree_builder.finalize();
///   let response_finalizer = ResponseFinalizerBuilder::new().finalize();
///
///   Router::new(tree, response_finalizer);
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
    /// Handles the `Request` by determining the correct `Route` from the internal `Tree`, storing
    /// any path related variables in `State` and dispatching to the associated `Handler`.
    fn handle(self, mut state: State) -> Box<HandlerFuture> {
        trace!("[{}] starting", request_id(&state));

        let future = match state.try_take::<RequestPathSegments>() {
            Some(rps) => {
                if let Some((_, leaf, sp, sm)) = self.data.tree.traverse(&rps.segments()) {
                    match leaf.select_route(&state) {
                        Ok(route) => match route.delegation() {
                            Delegation::External => {
                                trace!("[{}] delegating to secondary router", request_id(&state));

                                let mut rps = rps.clone();
                                rps.increase_offset(sp);
                                state.put(rps);

                                route.dispatch(state)
                            }
                            Delegation::Internal => {
                                trace!("[{}] dispatching to route", request_id(&state));
                                self.dispatch(state, sm, route)
                            }
                        },
                        Err(non_match) => {
                            let (status, allow) = non_match.deconstruct();

                            trace!("[{}] responding with error status", request_id(&state));
                            let mut res = create_response(&state, status, None);
                            if let StatusCode::MethodNotAllowed = status {
                                res.headers_mut().set(Allow(allow));
                            }
                            Box::new(future::ok((state, res)))
                        }
                    }
                } else {
                    trace!("[{}] did not find routable node", request_id(&state));
                    let res = create_response(&state, StatusCode::NotFound, None);
                    Box::new(future::ok((state, res)))
                }
            }
            None => {
                trace!("[{}] invalid request path segments", request_id(&state));
                let res = create_response(&state, StatusCode::InternalServerError, None);
                Box::new(future::ok((state, res)))
            }
        };

        self.finalize_response(future)
    }
}

impl Router {
    /// Creates a `Router` instance.
    pub fn new(tree: Tree, response_finalizer: ResponseFinalizer) -> Router {
        let router_data = RouterData::new(tree, response_finalizer);
        Router {
            data: Arc::new(router_data),
        }
    }

    fn dispatch(
        &self,
        mut state: State,
        sm: SegmentMapping,
        route: &Box<Route + Send + Sync>,
    ) -> Box<HandlerFuture> {
        match route.extract_request_path(&mut state, sm) {
            Ok(()) => {
                trace!("[{}] extracted request path", request_id(&state));
                match route.extract_query_string(&mut state) {
                    Ok(()) => {
                        trace!("[{}] extracted query string", request_id(&state));
                        trace!("[{}] dispatching", request_id(&state));
                        route.dispatch(state)
                    }
                    Err(_) => {
                        error!("[{}] the server cannot or will not process the request due to a client error within the query string",
                               request_id(&state));

                        let mut res = Response::new();
                        route.extend_response_on_query_string_error(&mut state, &mut res);
                        Box::new(future::ok((state, res)))
                    }
                }
            }
            Err(_) => {
                error!(
                    "[{}] the server cannot or will not process the request due to a client error on the request path",
                    request_id(&state)
                );
                let mut res = Response::new();
                route.extend_response_on_path_error(&mut state, &mut res);
                Box::new(future::ok((state, res)))
            }
        }
    }

    fn finalize_response(&self, result: Box<HandlerFuture>) -> Box<HandlerFuture> {
        let response_finalizer = self.data.response_finalizer.clone();
        let f = result
            .or_else(|(state, err)| {
                trace!(
                    "[{}] converting error into http response \
                     during finalization: {:?}",
                    request_id(&state),
                    err
                );
                let response = err.into_response(&state);
                future::ok((state, response))
            })
            .and_then(move |(state, res)| {
                trace!("[{}] handler complete", request_id(&state));
                response_finalizer.finalize(state, res)
            });

        Box::new(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use hyper::{Method, Uri};
    use hyper::header::{ContentLength, Headers};

    use extractor::{NoopPathExtractor, NoopQueryStringExtractor};
    use handler::HandlerError;
    use pipeline::set::*;
    use router::response::finalizer::ResponseFinalizerBuilder;
    use router::route::dispatch::DispatcherImpl;
    use router::route::matcher::MethodOnlyRouteMatcher;
    use router::route::{Extractors, RouteImpl};
    use router::tree::TreeBuilder;
    use router::tree::node::{NodeBuilder, SegmentType};
    use state::set_request_id;

    fn handler(state: State) -> (State, Response) {
        (state, Response::new())
    }

    fn send_request(
        r: Router,
        method: Method,
        uri: &str,
    ) -> Result<(State, Response), (State, HandlerError)> {
        let uri = Uri::from_str(uri).unwrap();

        let mut state = State::new();
        state.put(RequestPathSegments::new(uri.path()));
        state.put(method);
        state.put(uri);
        state.put(Headers::new());
        set_request_id(&mut state);

        r.handle(state).wait()
    }

    #[test]
    fn internal_server_error_if_no_request_path_segments() {
        let tree_builder = TreeBuilder::new();
        let tree = tree_builder.finalize();
        let router = Router::new(tree, ResponseFinalizerBuilder::new().finalize());

        let method = Method::Get;
        let uri = Uri::from_str("https://test.gotham.rs").unwrap();

        let mut state = State::new();
        state.put(method);
        state.put(uri);
        state.put(Headers::new());
        set_request_id(&mut state);

        match router.handle(state).wait() {
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
        let router = Router::new(tree, ResponseFinalizerBuilder::new().finalize());

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
            let matcher = MethodOnlyRouteMatcher::new(methods);
            let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
            let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> =
                Extractors::new();
            let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
            Box::new(route)
        };
        tree_builder.add_route(route);
        let tree = tree_builder.finalize();
        let router = Router::new(tree, ResponseFinalizerBuilder::new().finalize());

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
            let matcher = MethodOnlyRouteMatcher::new(methods);
            let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
            let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> =
                Extractors::new();
            let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
            Box::new(route)
        };
        tree_builder.add_route(route);
        let tree = tree_builder.finalize();
        let router = Router::new(tree, ResponseFinalizerBuilder::new().finalize());

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
                let matcher = MethodOnlyRouteMatcher::new(methods);
                let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
                let extractors: Extractors<
                    NoopPathExtractor,
                    NoopQueryStringExtractor,
                > = Extractors::new();
                let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
                Box::new(route)
            };
            tree_builder.add_route(route);

            let tree = tree_builder.finalize();
            Router::new(tree, ResponseFinalizerBuilder::new().finalize())
        };

        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut tree_builder = TreeBuilder::new();
        let mut delegated_node = NodeBuilder::new("var", SegmentType::Dynamic);

        let route = {
            let methods = vec![Method::Get];
            let matcher = MethodOnlyRouteMatcher::new(methods);
            let dispatcher = Box::new(DispatcherImpl::new(delegated_router, (), pipeline_set));
            let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> =
                Extractors::new();
            let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::External);
            Box::new(route)
        };

        delegated_node.add_route(route);
        tree_builder.add_child(delegated_node);
        let tree = tree_builder.finalize();
        let router = Router::new(tree, ResponseFinalizerBuilder::new().finalize());

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
    fn executes_response_finalizer_when_present() {
        let tree_builder = TreeBuilder::new();
        let tree = tree_builder.finalize();

        let mut response_finalizer_builder = ResponseFinalizerBuilder::new();
        let not_found_extender = |_s: &mut State, r: &mut Response| {
            r.headers_mut().set(ContentLength(3u64));
        };
        response_finalizer_builder.add(StatusCode::NotFound, Box::new(not_found_extender));
        let response_finalizer = response_finalizer_builder.finalize();
        let router = Router::new(tree, response_finalizer);

        match send_request(router, Method::Get, "https://test.gotham.rs/api") {
            Ok((_state, res)) => {
                assert_eq!(
                    *res.headers().get::<ContentLength>().unwrap(),
                    ContentLength(3u64)
                );
            }
            Err(_) => panic!("Router should have correctly handled request"),
        };
    }
}
