//! Defines the Gotham `Router`, which dispatches requests to the correct `Handler`

use std::io;
use std::sync::Arc;
use handler::{Handler, HandlerFuture, HandlerService};
use hyper::{self, Method};
use hyper::server::{Request, Response, NewService};

/// The `Router` type is the main entry point into a Gotham app, and it implements
/// `hyper::server::NewService` so that it can be passed directly to hyper after creation.
///
/// To create a `Router`, call `Router::build` with a closure which receives a `RouterBuilder` and
/// uses it to define routes.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// use gotham::router::Router;
/// use gotham::handler::HandlerFuture;
/// use hyper::server::{Http, Request, Response};
/// use hyper::Method::Get;
///
/// fn router() -> Router {
///     Router::build(|routes| {
///         routes.direct(Get, "/").to(MyApp::top);
///         routes.direct(Get, "/profile").to(MyApp::profile);
///     })
/// }
///
/// struct MyApp;
///
/// impl MyApp {
///     fn top(req: Request) -> Box<HandlerFuture> {
///         // Handler logic here
/// #       unimplemented!()
///     }
///
///     fn profile(req: Request) -> Box<HandlerFuture> {
///         // Handler logic here
/// #       unimplemented!()
///     }
/// }
///
/// fn main() {
///     let addr = "127.0.0.1:9000".parse().unwrap();
///     let server = Http::new().bind(&addr, router()).unwrap();
///     // As normal:
///     // server.run().unwrap()
/// }
/// ```
#[derive(Clone)]
pub struct Router {
    routes: Arc<Vec<Route>>,
}

impl Router {
    /// Calls the provided closure with a `RouterBuilder`, and then compiles the routes into a
    /// `Router`. See [`RouterBuilder`][RouterBuilder] for the available API.
    ///
    /// [RouterBuilder]: struct.RouterBuilder.html
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

/// `RouterBuilder` provides an API for constructing a `Router`. This is only instantiated by
/// [`Router::build(_)`][Router::build] and passed to the provided closure.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use gotham::router::{Router, RouterBuilder};
/// # use hyper::Method::Get;
/// # use hyper::server::{Http, Request, Response};
/// #
/// # fn handler(req: Request) -> Response {
/// #     Response::new()
/// # }
/// #
/// # fn main() {
/// Router::build(|routes: &mut RouterBuilder| {
///     routes.direct(Get, "/").to(handler);
/// })
/// # ;()
/// # }
/// ```
///
/// [Router::build]: struct.Router.html#method.build
pub struct RouterBuilder {
    routes: Vec<Route>,
}

/// Provides an API for a route matcher to be targeted at a `Handler`. This is instantiated by
/// `RouterBuilder`. See [RouterBuilder][] for a usage example.
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

    /// Creates a route matching a single HTTP method and a fixed string.
    ///
    /// The provided `path` must match the complete path of the request. For example, a request for
    /// `https://example.com/path/to/my/handler?query=params+go+here` would be matched by:
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # use gotham::router::Router;
    /// # use hyper::Method::Get;
    /// # use hyper::server::{Request, Response};
    /// #
    /// #
    /// fn handler(req: Request) -> Response {
    ///     // Handler implementation here
    /// #   Response::new()
    /// }
    ///
    /// fn router() -> Router {
    ///     Router::build(|routes| {
    ///         routes.direct(Get, "/path/to/my/handler").to(handler);
    ///     })
    /// }
    /// #
    /// # fn main() {
    /// #   router();
    /// # }
    /// ```
    pub fn direct<'a>(&'a mut self, method: Method, path: &'static str) -> RouterBuilderTo<'a> {
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
    /// Targets the current route at a specific handler.
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
    use hyper::StatusCode;
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
            let router = Router::build(|route| route.direct(Get, "/").to(Root::index));
            Ok(HandlerService::new(router))
        };
        let mut test_server = TestServer::new(new_service).unwrap();
        let client = test_server.client("127.0.0.1:10000".parse().unwrap()).unwrap();
        let uri = "http://example.com/".parse().unwrap();
        let response = test_server.run_request(client.get(uri)).unwrap();
        assert_eq!(*response.status(), StatusCode::Ok);
        assert_eq!(test_server.read_body(response).unwrap(), "Index".as_bytes());

    }

    #[test]
    fn route_direct_request_ignoring_query_params() {
        let new_service = || {
            let router = Router::build(|route| route.direct(Get, "/").to(Root::index));
            Ok(HandlerService::new(router))
        };
        let mut test_server = TestServer::new(new_service).unwrap();
        let client = test_server.client("127.0.0.1:10000".parse().unwrap()).unwrap();
        let uri = "http://example.com/?x=y".parse().unwrap();
        let response = test_server.run_request(client.get(uri)).unwrap();
        assert_eq!(*response.status(), StatusCode::Ok);
        assert_eq!(test_server.read_body(response).unwrap(), "Index".as_bytes());
    }
}
