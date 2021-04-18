//! An example usage of Gotham from another service.

use anyhow::{Context as _, Error};
use futures_util::future::{BoxFuture, FutureExt};
use gotham::{
    router::{builder::*, Router},
    service::call_handler,
    state::State,
};
use http::{Request, Response};
use hyper::{server::conn::Http, service::Service, Body};
use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::task;
use tokio::net::TcpListener;

#[derive(Clone)]
struct MyService {
    router: Router,
    addr: SocketAddr,
}

impl Service<Request<Body>> for MyService {
    type Response = Response<Body>;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
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
            Http::new()
                .serve_connection(socket, service)
                .await
                .context("Error serving connection")?;

            Result::<_, Error>::Ok(())
        };

        tokio::spawn(task);
    }
}
