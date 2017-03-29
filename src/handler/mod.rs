use hyper;
use hyper::server;
use hyper::server::Request;
use futures::Future;

pub type HandlerFuture = Future<Item = server::Response, Error = hyper::Error>;

pub struct HandlerService<T>
    where T: Handler
{
    handler: T,
}

impl<T> HandlerService<T>
    where T: Handler
{
    pub fn new(t: T) -> HandlerService<T> {
        HandlerService { handler: t }
    }
}

impl<T> server::Service for HandlerService<T>
    where T: Handler
{
    type Request = server::Request;
    type Response = server::Response;
    type Error = hyper::Error;
    type Future = Box<HandlerFuture>;

    fn call(&self, req: Self::Request) -> Self::Future {
        self.handler.handle(req)
    }
}

pub trait Handler: Send + Sync {
    fn handle(&self, req: Request) -> Box<HandlerFuture>;
}

impl<F> Handler for F
    where F: Fn(Request) -> Box<HandlerFuture> + Send + Sync
{
    fn handle(&self, req: Request) -> Box<HandlerFuture> {
        self(req)
    }
}
