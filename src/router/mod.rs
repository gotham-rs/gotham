use std::io;
use std::sync::Arc;
use handler::{Handler, HandlerFuture, HandlerService};
use hyper::{self, Method, StatusCode};
use hyper::server::{Request, Response, NewService};

#[derive(Clone)]
pub struct Router {
    routes: Arc<Vec<Route>>,
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

impl NewService for Router {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = HandlerService<Router>;

    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(HandlerService::new(self.clone()))
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

pub struct RouterBuilderTo<'a> {
    builder: &'a mut RouterBuilder,
    matcher: Box<RouteMatcher>,
}

impl RouterBuilder {
    fn new() -> RouterBuilder {
        RouterBuilder { routes: Vec::default() }
    }

    fn into_router(mut self) -> Router {
        Router { routes: Arc::new(self.routes.drain(..).collect()) }
    }

    pub fn match_direct<'a>(&'a mut self,
                            method: Method,
                            path: &'static str)
                            -> RouterBuilderTo<'a> {
        RouterBuilderTo {
            builder: self,
            matcher: Box::new(DirectRouteMatcher {
                                  method: method,
                                  path: path,
                              }),
        }
    }
}

impl<'a> RouterBuilderTo<'a> {
    pub fn to<H>(self, handler: H)
        where H: Handler + 'static
    {
        let route = Route {
            matcher: self.matcher,
            handler: Box::new(handler),
        };

        self.builder.routes.push(route)
    }
}

struct Route {
    matcher: Box<RouteMatcher>,
    handler: Box<Handler>,
}

trait RouteMatcher: Send + Sync {
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

struct DirectRouteMatcher {
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
    use hyper::Method::*;
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
