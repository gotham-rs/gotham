#![deny(warnings)]
extern crate futures;
extern crate hyper;
extern crate pretty_env_logger;
extern crate gotham;

use futures::{future, Future};

use hyper::{Get, Post};
use hyper::header::ContentLength;
use hyper::server::{Http, Request, Response};

use gotham::router::Router;
use gotham::handler::HandlerFuture;

struct Echo;

static INDEX: &'static [u8] = b"Try POST /echo";
static ASYNC: &'static [u8] = b"Got async response";

fn router() -> Router {
    Router::build(|routes| {
                      routes.direct(Get, "/").to(Echo::get);
                      routes.direct(Get, "/echo").to(Echo::get);
                      routes.direct(Post, "/echo").to(Echo::post);
                      routes.direct(Get, "/async").to(Echo::async);
                  })
}

impl Echo {
    fn get(_req: Request) -> Response {
        Response::new().with_header(ContentLength(INDEX.len() as u64)).with_body(INDEX)
    }

    fn post(req: Request) -> Response {
        let mut res = Response::new();
        if let Some(len) = req.headers().get::<ContentLength>() {
            res.headers_mut().set(len.clone());
        }
        res.with_body(req.body())
    }

    fn async(_req: Request) -> Box<HandlerFuture> {
        let mut res = Response::new();
        res = res.with_header(ContentLength(ASYNC.len() as u64)).with_body(ASYNC);
        future::lazy(move || future::ok(res)).boxed()
    }
}

fn main() {
    pretty_env_logger::init().unwrap();
    let addr = "127.0.0.1:1337".parse().unwrap();

    let server = Http::new().bind(&addr, router()).unwrap();
    println!("Listening on http://{} with 1 thread.",
             server.local_addr().unwrap());
    server.run().unwrap();
}
