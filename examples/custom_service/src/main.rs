//! An example usage of Gotham from another service.

use anyhow::{Context as _, Error};
use futures_util::future::{BoxFuture, FutureExt};
use gotham::helpers::http::Body;
use gotham::http::{Request, Response};
use gotham::hyper::body::Incoming;
use gotham::hyper::service::Service;
use gotham::hyper_util::rt::{TokioExecutor, TokioIo};
use gotham::hyper_util::server::conn::auto::Builder as ServerBuilder;
use gotham::prelude::*;
use gotham::router::{build_simple_router, Router};
use gotham::service::call_handler;
use gotham::state::State;
use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use tokio::net::TcpListener;

#[derive(Clone)]
struct MyService {
    router: Router,
    addr: SocketAddr,
}

impl Service<Request<Incoming>> for MyService {
    type Response = Response<Body>;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        // NOTE: You don't *have* to use call_handler for this (you could use `router.handle`), but
        // call_handler will catch panics and return en error response.
        let state = State::from_request(req, self.addr);
        call_handler(self.router.clone(), AssertUnwindSafe(state)).boxed()
    }
}

pub fn say_hello(state: State) -> (State, &'static str) {
    (state, "hello world")
}

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let router = build_simple_router(|route| {
        // For the path "/" invoke the handler "say_hello"
        route.get("/").to(say_hello);
    });

    let addr = "127.0.0.1:7878";
    let listener = TcpListener::bind(&addr).await?;

    println!("Listening for requests at http://{}", addr);

    loop {
        let (socket, addr) = listener
            .accept()
            .await
            .context("Error accepting connection")?;

        let service = MyService {
            router: router.clone(),
            addr,
        };

        let task = async move {
            ServerBuilder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(socket), service)
                .await
                .map_err(anyhow::Error::from_boxed)
                .context("Error serving connection")?;

            Result::<_, Error>::Ok(())
        };

        tokio::spawn(task);
    }
}
