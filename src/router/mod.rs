use handler::{Handler, HandlerFuture};

pub use hyper::{Method, StatusCode};
pub use hyper::Method::*;
pub use hyper::server::{Request, Response};

pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn build<F>(f: F) -> Router
        where F: FnOnce(&mut RouterBuilder) -> ()
    {
        let mut builder = RouterBuilder::new();
        f(&mut builder);
        builder.into_router()
    }
}

impl Handler for Router {
    fn handle(&self, req: Request) -> Box<HandlerFuture> {
        // Deliberately obtuse implementation while we hash out the API.
        match self.routes
                  .iter()
                  .filter(|r| r.matcher.matches(&req))
                  .take(1)
                  .next() {
            Some(ref route_box) => route_box.handler.handle(req),
            None => unimplemented!(),
        }
    }
}

pub struct RouterBuilder {
    routes: Vec<Route>,
}

pub struct RouterBuilderTo<'a, M>
    where M: RouteMatcher
{
    builder: &'a mut RouterBuilder,
    matcher: M,
}

impl RouterBuilder {
    fn new() -> RouterBuilder {
        RouterBuilder { routes: Vec::default() }
    }

    fn into_router(mut self) -> Router {
        Router { routes: self.routes.drain(..).collect() }
    }

    pub fn match_direct<'a>(&'a mut self,
                            method: Method,
                            path: &'static str)
                            -> RouterBuilderTo<'a, DirectRouteMatcher> {
        RouterBuilderTo {
            builder: self,
            matcher: DirectRouteMatcher {
                method: method,
                path: path,
            },
        }
    }
}

impl<'a, M> RouterBuilderTo<'a, M>
    where M: RouteMatcher + 'static
{
    pub fn to<H>(self, handler: H)
        where H: Handler + 'static
    {
        let route = Route {
            matcher: Box::new(self.matcher),
            handler: Box::new(handler),
        };

        self.builder.routes.push(route)
    }
}

pub struct Route {
    matcher: Box<RouteMatcher>,
    handler: Box<Handler>,
}

pub trait RouteMatcher {
    fn matches(&self, req: &Request) -> bool;

    fn to<H>(self, h: H) -> Route
        where H: Handler + 'static,
              Self: Sized + 'static
    {
        Route {
            matcher: Box::new(self),
            handler: Box::new(h),
        }
    }
}

pub struct DirectRouteMatcher {
    method: Method,
    path: &'static str,
}

impl RouteMatcher for DirectRouteMatcher {
    fn matches(&self, req: &Request) -> bool {
        *req.method() == self.method && req.path() == self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handler::{HandlerService, HandlerFuture};
    use futures::{future, Future};
    use test::TestServer;

    struct Root {}

    impl Root {
        fn index(_req: Request) -> Box<HandlerFuture> {
            future::ok(Response::new().with_status(StatusCode::Ok).with_body("Index")).boxed()
        }
    }

    #[test]
    fn route_direct_request() {
        let new_service = || {
            let router = Router::build(|route| route.match_direct(Get, "/").to(Root::index));
            Ok(HandlerService::new(router))
        };
        let mut test_server = TestServer::new(new_service).unwrap();
        let client = test_server.client("127.0.0.1:10000".parse().unwrap()).unwrap();
        let uri = "http://example.com/".parse().unwrap();
        let response = test_server.run_request(client.get(uri)).unwrap();
        assert_eq!(*response.status(), StatusCode::Ok);
        assert_eq!(test_server.read_body(response).unwrap(), "Index".as_bytes());
    }
}
