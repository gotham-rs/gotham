#![deny(warnings)]
extern crate futures;
extern crate hyper;
extern crate pretty_env_logger;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate borrow_bag;

mod middleware;

use futures::{future, Future};

use hyper::header::ContentLength;
use hyper::server::{Http, Request, Response};
use hyper::Method;
use hyper::status::StatusCode;

use gotham::router::Router;
use gotham::router::route::{Route, RouteImpl};
use gotham::dispatch::{Dispatcher, PipelineHandleChain};
use gotham::router::request_matcher::MethodOnlyRequestMatcher;
use gotham::router::tree::Tree;
use gotham::router::tree::node::Node;
use gotham::router::tree::node::NodeSegmentType;
use gotham::handler::{NewHandler, HandlerFuture, NewHandlerService};
use gotham::middleware::pipeline::new_pipeline;
use gotham::state::State;

use self::middleware::{KitchenSinkData, KitchenSinkMiddleware};

struct Echo;

#[derive(RequestPathExtractor)]
struct SharedRequestPath {
    name: f64,

    // Ideally RequestPathExtractors that are implemented by applications won't have any
    // Option fields.
    //
    // Instead have a fully specified Struct to represent every route with different segments
    // or meanings to ensure type safety.
    from: Option<String>,
}

static INDEX: &'static [u8] = b"Try POST /echo";
static ASYNC: &'static [u8] = b"Got async response";

fn basic_route<NH, P, C>(methods: Vec<Method>,
                         new_handler: NH,
                         pipelines: C)
                         -> Box<Route<P> + Send + Sync>
    where NH: NewHandler + 'static,
          C: PipelineHandleChain<P> + Send + Sync + 'static,
          P: Send + Sync + 'static
{
    let matcher = MethodOnlyRequestMatcher::new(methods);
    let dispatcher = Dispatcher::new(new_handler, pipelines);
    let route: RouteImpl<_, _, _, _, SharedRequestPath> = RouteImpl::new(matcher, dispatcher);
    Box::new(route)
}

// Builds a tree that looks like:
//
// /                     --> (Get Route)
// | - echo              --> (Get + Post Routes)
// | - async             --> (Get Route)
// | - header_value      --> (Get Route)
// | - hello
//     | - :var          --> (Get Route)
fn add_routes<P, C>(tree: &mut Tree<P>, pipelines: C)
    where C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
          P: Send + Sync + 'static
{
    tree.add_route(basic_route(vec![Method::Get], || Ok(Echo::get), pipelines));

    let mut echo = Node::new("echo", NodeSegmentType::Static);
    echo.add_route(basic_route(vec![Method::Get], || Ok(Echo::get), pipelines));
    echo.add_route(basic_route(vec![Method::Post], || Ok(Echo::post), pipelines));
    tree.add_child(echo);

    let mut async = Node::new("async", NodeSegmentType::Static);
    async.add_route(basic_route(vec![Method::Get], || Ok(Echo::async), pipelines));
    tree.add_child(async);

    let mut header_value = Node::new("header_value", NodeSegmentType::Static);
    header_value.add_route(basic_route(vec![Method::Get], || Ok(Echo::header_value), pipelines));
    tree.add_child(header_value);

    let mut hello = Node::new("hello", NodeSegmentType::Static);

    let mut var = Node::new("name", NodeSegmentType::Dynamic);
    var.add_route(basic_route(vec![Method::Get], || Ok(Echo::hello), pipelines));

    let mut var2 = Node::new("from", NodeSegmentType::Dynamic);
    var2.add_route(basic_route(vec![Method::Get], || Ok(Echo::greeting), pipelines));

    var.add_child(var2);
    hello.add_child(var);
    tree.add_child(hello);
}

impl Echo {
    fn not_found(state: State, _req: Request) -> (State, Response) {
        (state, Response::new().with_status(StatusCode::NotFound))
    }

    fn internal_server_error(state: State, _req: Request) -> (State, Response) {
        (state, Response::new().with_status(StatusCode::InternalServerError))
    }

    fn get(state: State, _req: Request) -> (State, Response) {
        (state, Response::new().with_header(ContentLength(INDEX.len() as u64)).with_body(INDEX))
    }

    fn post(state: State, req: Request) -> (State, Response) {
        let mut res = Response::new();
        if let Some(len) = req.headers().get::<ContentLength>() {
            res.headers_mut().set(len.clone());
        }
        (state, res.with_body(req.body()))
    }

    fn async(state: State, _req: Request) -> Box<HandlerFuture> {
        let mut res = Response::new();
        res = res.with_header(ContentLength(ASYNC.len() as u64)).with_body(ASYNC);
        future::lazy(move || future::ok((state, res))).boxed()
    }

    fn header_value(mut state: State, _req: Request) -> (State, Response) {
        state.borrow_mut::<KitchenSinkData>().unwrap().header_value = "different value!".to_owned();
        (state, Response::new().with_header(ContentLength(INDEX.len() as u64)).with_body(INDEX))
    }

    fn hello(state: State, _req: Request) -> (State, Response) {
        let hello = format!("Hello, {}\n",
                            state.borrow::<SharedRequestPath>().unwrap().name);

        (state, Response::new().with_header(ContentLength(hello.len() as u64)).with_body(hello))
    }

    fn greeting(state: State, _req: Request) -> (State, Response) {
        let res = {
            let srp = state.borrow::<SharedRequestPath>().unwrap();
            let name = srp.name;
            let from = match srp.from {
                Some(ref s) => &s,
                None => "",
            };

            let g = format!("Greetings, {} from {}\n", name, from);
            Response::new().with_header(ContentLength(g.len() as u64)).with_body(g)
        };
        (state, res)
    }
}

fn main() {
    pretty_env_logger::init().unwrap();
    let addr = "127.0.0.1:1337".parse().unwrap();

    let mut tree = Tree::new();
    let pipelines = borrow_bag::new_borrow_bag();
    let (pipelines, pipeline) =
        pipelines.add(new_pipeline()
                          .add(KitchenSinkMiddleware { header_name: "X-Kitchen-Sink" })
                          .build());

    add_routes(&mut tree, (pipeline, ()));
    tree.finalize();

    let not_found = || Ok(Echo::not_found);
    let internal_server_error = || Ok(Echo::internal_server_error);
    let router = Router::new(tree, pipelines, not_found, internal_server_error);

    let server = Http::new().bind(&addr, NewHandlerService::new(router)).unwrap();

    println!("Listening on http://{} with 1 thread.",
             server.local_addr().unwrap());
    server.run().unwrap();
}
