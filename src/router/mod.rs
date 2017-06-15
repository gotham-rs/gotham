//! Defines a `Router` and supporting types.

pub mod tree;
pub mod route;
pub mod request_matcher;
pub mod response_extender;

use std::io;
use std::sync::Arc;

use borrow_bag::BorrowBag;
use futures::{future, Future};
use hyper::{Headers, StatusCode, Uri, HttpVersion, Method};
use hyper::server::{Request, Response};
use handler::{NewHandler, Handler, HandlerFuture};
use router::response_extender::ResponseExtender;
use router::route::Route;
use router::tree::{SegmentMapping, Tree};
use state::request_id::set_request_id;
use state::{State, StateData};
use http::{request_path, query_string};

// Holds data for Router which lives behind single Arc instance
// so that otherwise non Clone-able structs are able to be used via NewHandler
struct RouterData<'n, P> {
    tree: Tree<'n, P>,
    pipelines: BorrowBag<P>,
    response_extender: ResponseExtender,
}

impl<'n, P> RouterData<'n, P> {
    pub fn new(tree: Tree<'n, P>,
               pipelines: BorrowBag<P>,
               response_extender: ResponseExtender)
               -> RouterData<'n, P> {
        RouterData {
            tree,
            pipelines,
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
///   let pipelines = borrow_bag::new_borrow_bag();
///   let response_extender = ResponseExtenderBuilder::new().finalize();
///
///   Router::new(tree, pipelines, response_extender);
/// # }
/// ```
pub struct Router<'n, P>
    where P: Sync
{
    data: Arc<RouterData<'n, P>>,
}

impl<'n, P> Clone for Router<'n, P>
    where P: Sync
{
    fn clone(&self) -> Router<'n, P> {
        Router { data: self.data.clone() }
    }
}

impl<'n, P> NewHandler for Router<'n, P>
    where P: Send + Sync
{
    type Instance = Router<'n, P>;

    // Creates a new Router instance to route new HTTP requests
    fn new_handler(&self) -> io::Result<Self::Instance> {
        Ok((*self).clone())
    }
}

impl<'n, P> Handler for Router<'n, P>
    where P: Send + Sync
{
    /// Handles the request by determining the correct `Route` from the internal
    /// `Tree`, storing any path related variables in `State` and dispatching
    /// appropriately to the configured `Handler`.
    fn handle(self, mut state: State, req: Request) -> Box<HandlerFuture> {
        set_request_id(&mut state, &req);

        let uri = req.uri().clone();
        self.populate_state(&mut state, &req);
        let response = self.route(uri, state, req);
        self.finalize_response(response)
    }
}

impl<'n, P> Router<'n, P>
    where P: Sync
{
    /// Creates a `Router` instance.
    pub fn new(tree: Tree<'n, P>,
               pipelines: BorrowBag<P>,
               response_extender: ResponseExtender)
               -> Router<'n, P> {

        let router_data = RouterData::new(tree, pipelines, response_extender);
        Router { data: Arc::new(router_data) }
    }

    fn populate_state(&self, state: &mut State, req: &Request) {
        state.put(req.method().clone());
        state.put(req.uri().clone());
        state.put(req.version().clone());
        state.put(req.headers().clone());
    }

    fn route(&self, uri: Uri, state: State, req: Request) -> Box<HandlerFuture> {
        let rp = request_path::split(uri.path());
        if let Some((_, leaf, segment_mapping)) = self.data.tree.traverse(rp.as_slice()) {
            // Valid path for the application, determine if any configured
            // routes will accept the request
            if let Some(route) = leaf.borrow_routes()
                   .iter()
                   .find(|r| r.is_match(&req).is_ok()) {
                self.dispatch(state, req, &uri, segment_mapping, route)
            } else {
                // No routes accepted the request, the error status associated with the first route
                // is then chosen as the status code for the response.
                let status = leaf.borrow_routes().first().unwrap().is_match(&req);
                self.generate_response(status.unwrap_err(), state)
            }
        } else {
            self.generate_response(StatusCode::NotFound, state)
        }
    }

    fn dispatch(&self,
                mut state: State,
                req: Request,
                uri: &Uri,
                segment_mapping: SegmentMapping,
                route: &Box<Route<P> + Send + Sync>)
                -> Box<HandlerFuture> {
        match route.extract_request_path(&mut state, segment_mapping) {
            Ok(()) => {
                if let Some(q) = uri.query() {
                    match route.extract_query_string(&mut state, query_string::split(q)) {
                        Ok(()) => return route.dispatch(&self.data.pipelines, state, req),
                        Err(_) => (),
                    }
                } else {
                    return route.dispatch(&self.data.pipelines, state, req);
                }
            }
            Err(_) => (),
        };

        let mut res = Response::new();
        res.set_status(StatusCode::InternalServerError);
        future::ok((state, res)).boxed()
    }

    fn generate_response(&self, status: StatusCode, state: State) -> Box<HandlerFuture> {
        let mut res = Response::new();
        res.set_status(status);
        future::ok((state, res)).boxed()
    }

    fn finalize_response(&self, result: Box<HandlerFuture>) -> Box<HandlerFuture> {
        let response_extender = self.data.response_extender.clone();
        result
            .and_then(move |(state, res)| response_extender.extend(state, res))
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
    use uuid::Uuid;

    use borrow_bag;

    use router::tree::TreeBuilder;
    use router::response_extender::ResponseExtenderBuilder;
    use state::request_id;

    #[test]
    fn executes_response_extender_when_present() {
        let tree_builder = TreeBuilder::new();
        let tree = tree_builder.finalize();
        let pipelines = borrow_bag::new_borrow_bag();

        let mut response_extender_builder = ResponseExtenderBuilder::new();
        let not_found_extender = |s, mut r: Response| {
            r.headers_mut().set(ContentLength(3u64));
            future::ok((s, r)).boxed()
        };
        response_extender_builder.add(StatusCode::NotFound, Box::new(not_found_extender));
        let response_extender = response_extender_builder.finalize();

        let router = Router::new(tree, pipelines, response_extender);
        let method = Method::Get;
        let uri = Uri::from_str("https://test.gotham.rs").unwrap();
        let request: Request<Body> = Request::new(method, uri);

        let state = State::new();
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
        let pipelines = borrow_bag::new_borrow_bag();
        let response_extender = ResponseExtenderBuilder::new().finalize();

        let router = Router::new(tree, pipelines, response_extender);
        let method = Method::Get;
        let uri = Uri::from_str("https://test.gotham.rs").unwrap();
        let version = HttpVersion::H2;
        let mut request: Request<Body> = Request::new(method.clone(), uri.clone());
        request.set_version(version.clone());
        request.headers_mut().set(ContentType::json());

        let state = State::new();
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

    #[test]
    fn populates_request_id() {
        let tree_builder = TreeBuilder::new();
        let tree = tree_builder.finalize();
        let pipelines = borrow_bag::new_borrow_bag();
        let response_extender = ResponseExtenderBuilder::new().finalize();

        let router = Router::new(tree, pipelines, response_extender);
        let method = Method::Get;
        let uri = Uri::from_str("https://test.gotham.rs").unwrap();
        let request: Request<Body> = Request::new(method, uri);

        let state = State::new();
        let result = router.handle(state, request).wait();

        match result {
            Ok((state, _res)) => {
                assert_eq!(4,
                           Uuid::parse_str(request_id(&state))
                               .unwrap()
                               .get_version_num())
            }
            Err(_) => panic!("Router should have correctly handled request"),
        }
    }
}
