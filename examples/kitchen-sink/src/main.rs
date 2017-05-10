#![deny(warnings)]
extern crate futures;
extern crate hyper;
extern crate pretty_env_logger;
extern crate gotham;

mod middleware;

use futures::{future, Future};

use hyper::header::ContentLength;
use hyper::server::{Http, Request, Response};
use hyper::Method;
use hyper::status::StatusCode;

use gotham::router::Router;
use gotham::router::route::{Route, RouteImpl};
use gotham::dispatch::{Dispatcher};
use gotham::router::request_matcher::MethodOnlyRequestMatcher;
use gotham::router::tree::Tree;
use gotham::router::tree::node::Node;
use gotham::router::tree::segment_matcher::StaticSegmentMatcher;
use gotham::handler::{NewHandler, HandlerFuture, NewHandlerService};
use gotham::state::State;

use self::middleware::KitchenSinkData; // TODO {KitchenSinkData, KitchenSinkMiddleware};

struct Echo;

static INDEX: &'static [u8] = b"Try POST /echo";
static ASYNC: &'static [u8] = b"Got async response";
static HELLO: &'static [u8] = b"Hello world!!";

fn basic_route<NH>(methods: Vec<Method>, new_handler: NH) -> Box<Route + Send + Sync>
    where NH: NewHandler + 'static
{
    let matcher = MethodOnlyRequestMatcher::new(methods);
    let dispatcher = Dispatcher::new(new_handler, ());
    Box::new(RouteImpl::new(matcher, dispatcher))
}

// Builds a tree that looks like:
//
// /                     --> (Get Route)
// | - echo              --> (Get + Post Routes)
// | - async             --> (Get Route)
// | - header_value      --> (Get Route)
// | - hello
//     | - world         --> (Get Route)
fn add_routes(tree: &mut Tree) {
    tree.add_route(basic_route(vec![Method::Get], || Ok(Echo::get)));

    let mut echo = Node::new("echo", Box::new(StaticSegmentMatcher::new()));
    echo.add_route(basic_route(vec![Method::Get], || Ok(Echo::get)));
    echo.add_route(basic_route(vec![Method::Post], || Ok(Echo::post)));
    tree.add_child(echo);

    let mut async = Node::new("async", Box::new(StaticSegmentMatcher::new()));
    async.add_route(basic_route(vec![Method::Get], || Ok(Echo::async)));
    tree.add_child(async);

    let mut header_value = Node::new("header_value", Box::new(StaticSegmentMatcher::new()));
    header_value.add_route(basic_route(vec![Method::Get], || Ok(Echo::header_value)));
    tree.add_child(header_value);

    let mut hello = Node::new("hello", Box::new(StaticSegmentMatcher::new()));
    let mut world = Node::new("world", Box::new(StaticSegmentMatcher::new()));
    world.add_route(basic_route(vec![Method::Get], || Ok(Echo::world)));
    hello.add_child(world);
    tree.add_child(hello);
}

// TODO: Re-instate Pipelines once direction decided
//fn pipeline() -> Pipeline<(KitchenSinkMiddleware, ())> {
    //new_pipeline().add(KitchenSinkMiddleware { header_name: "X-Kitchen-Sink" }).build()
//}

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

    fn world(state: State, _req: Request) -> (State, Response) {
        (state, Response::new().with_header(ContentLength(HELLO.len() as u64)).with_body(HELLO))
    }

}

fn main() {
    pretty_env_logger::init().unwrap();
    let addr = "127.0.0.1:1337".parse().unwrap();

    let mut tree = Tree::new();
    add_routes(&mut tree);

    let not_found = || Ok(Echo::not_found);
    let internal_server_error = || Ok(Echo::internal_server_error);
    let router = Router::new(tree, not_found, internal_server_error);

    let server = Http::new().bind(&addr, NewHandlerService::new(router)).unwrap();

    println!("Listening on http://{} with 1 thread.",
             server.local_addr().unwrap());
    server.run().unwrap();
}
