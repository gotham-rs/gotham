use std::panic::RefUnwindSafe;
use std::path::{Path, PathBuf};

use extractor::{PathExtractor, QueryStringExtractor};
use pipeline::chain::PipelineHandleChain;
use router::builder::{ExtendRouteMatcher, ReplacePathExtractor, ReplaceQueryStringExtractor,
                      SingleRouteBuilder};
use router::route::{Delegation, Extractors, RouteImpl};
use router::route::matcher::RouteMatcher;
use router::route::dispatch::DispatcherImpl;
use handler::{Handler, NewHandler};
use handler::static_file::{FileHandler, FilePathExtractor, FileSystemHandler};

/// Describes the API for defining a single route, after determining which request paths will be
/// dispatched here. The API here uses chained function calls to build and add the route into the
/// `RouterBuilder` which created it.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::builder::*;
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::pipeline::single::*;
/// # use gotham::middleware::session::NewSessionMiddleware;
/// # use gotham::test::TestServer;
/// #
/// fn my_handler(state: State) -> (State, Response) {
///     // Handler implementation elided.
/// #   (state, Response::new().with_status(StatusCode::Accepted))
/// }
/// #
/// # fn router() -> Router {
/// #   let (chain, pipelines) = single_pipeline(
/// #       new_pipeline().add(NewSessionMiddleware::default()).build()
/// #   );
/// #
/// build_router(chain, pipelines, |route| {
///     route.get("/request/path") // <- This value implements `DefineSingleRoute`
///          .to(my_handler);
/// })
/// # }
/// #
/// # fn main() {
/// #   let test_server = TestServer::new(router()).unwrap();
/// #   let response = test_server.client()
/// #       .get("https://example.com/request/path")
/// #       .perform()
/// #       .unwrap();
/// #   assert_eq!(response.status(), StatusCode::Accepted);
/// # }
/// ```
pub trait DefineSingleRoute {
    /// Directs the route to the given `Handler`, automatically creating a `NewHandler` which
    /// copies the `Handler`. This is the easiest option for code which is using bare functions as
    /// `Handler` functions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::single::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::test::TestServer;
    /// #
    /// fn my_handler(state: State) -> (State, Response) {
    ///     // Handler implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    /// #
    /// # fn router() -> Router {
    /// #   let (chain, pipelines) = single_pipeline(
    /// #       new_pipeline().add(NewSessionMiddleware::default()).build()
    /// #   );
    ///
    /// build_router(chain, pipelines, |route| {
    ///     route.get("/request/path").to(my_handler);
    /// })
    /// #
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/request/path")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn to<H>(self, handler: H)
    where
        H: Handler + RefUnwindSafe + Copy + Send + Sync + 'static;

    /// Directs the route to the given `NewHandler`. This gives more control over how `Handler`
    /// values are constructed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate futures;
    /// #
    /// # use std::io;
    /// # use hyper::{Response, StatusCode};
    /// # use futures::future;
    /// # use gotham::handler::{Handler, HandlerFuture, NewHandler};
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::single::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::test::TestServer;
    /// #
    /// struct MyNewHandler;
    /// struct MyHandler;
    ///
    /// impl NewHandler for MyNewHandler {
    ///     type Instance = MyHandler;
    ///
    ///     fn new_handler(&self) -> io::Result<Self::Instance> {
    ///         Ok(MyHandler)
    ///     }
    /// }
    ///
    /// impl Handler for MyHandler {
    ///     fn handle(self, state: State) -> Box<HandlerFuture> {
    ///         // Handler implementation elided.
    /// #       let response = Response::new().with_status(StatusCode::Accepted);
    /// #       Box::new(future::ok((state, response)))
    ///     }
    /// }
    /// #
    /// # fn router() -> Router {
    /// #   let (chain, pipelines) = single_pipeline(
    /// #       new_pipeline().add(NewSessionMiddleware::default()).build()
    /// #   );
    ///
    /// build_router(chain, pipelines, |route| {
    ///     route.get("/request/path").to_new_handler(MyNewHandler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/request/path")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn to_new_handler<NH>(self, new_handler: NH)
    where
        NH: NewHandler + 'static;

    /// Directs the route to serve static files from the given root path.
    /// The route must contain a trailing glob segment, which will be used
    /// to serve any matching names under the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::StatusCode;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::single::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::test::TestServer;
    /// #
    /// #
    /// # fn router() -> Router {
    /// #   let (chain, pipelines) = single_pipeline(
    /// #       new_pipeline().add(NewSessionMiddleware::default()).build()
    /// #   );
    ///
    /// build_router(chain, pipelines, |route| {
    ///     route.get("/*").to_filesystem("resources/test/static_files");
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/doc.html")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Ok);
    /// # }
    /// ```
    fn to_filesystem<P: AsRef<Path>>(self, root: P)
    where
        Self: Sized,
        Self: ReplacePathExtractor<FilePathExtractor>,
        Self::Output: DefineSingleRoute,
        PathBuf: From<P>,
    {
        self.with_path_extractor::<FilePathExtractor>()
            .to_new_handler(FileSystemHandler::new(root));
    }

    /// Directs the route to serve a single static file from the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::StatusCode;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::single::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::test::TestServer;
    /// #
    /// #
    /// # fn router() -> Router {
    /// #   let (chain, pipelines) = single_pipeline(
    /// #       new_pipeline().add(NewSessionMiddleware::default()).build()
    /// #   );
    ///
    /// build_router(chain, pipelines, |route| {
    ///     route.get("/").to_file("resources/test/static_files/doc.html");
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Ok);
    /// # }
    /// ```
    fn to_file<P: AsRef<Path>>(self, path: P)
    where
        Self: Sized,
        PathBuf: From<P>,
    {
        self.to_new_handler(FileHandler::new(path));
    }

    /// Applies a `PathExtractor` type to the current route, to extract path parameters into
    /// `State` with the given type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate gotham_derive;
    /// # #[macro_use]
    /// # extern crate serde_derive;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::state::{State, FromState};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::set::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::test::TestServer;
    /// #
    /// #[derive(Deserialize, StateData, StaticResponseExtender)]
    /// struct MyPathParams {
    /// #   #[allow(dead_code)]
    ///     name: String,
    /// }
    ///
    /// fn my_handler(state: State) -> (State, Response) {
    /// #   {
    ///     let params = MyPathParams::borrow_from(&state);
    ///
    ///     // Handler implementation elided.
    /// #   assert_eq!(params.name, "world");
    /// #   }
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    /// #
    /// # fn router() -> Router {
    /// #   let pipelines = new_pipeline_set();
    /// #   let (pipelines, default) =
    /// #       pipelines.add(new_pipeline().add(NewSessionMiddleware::default()).build());
    /// #
    /// #   let pipelines = finalize_pipeline_set(pipelines);
    /// #
    /// #   let default_pipeline_chain = (default, ());
    ///
    /// build_router(default_pipeline_chain, pipelines, |route| {
    ///     route.get("/hello/:name")
    ///          .with_path_extractor::<MyPathParams>()
    ///          .to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/hello/world")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn with_path_extractor<NPE>(self) -> <Self as ReplacePathExtractor<NPE>>::Output
    where
        NPE: PathExtractor + Send + Sync + 'static,
        Self: ReplacePathExtractor<NPE>,
        Self::Output: DefineSingleRoute;

    /// Applies a `QueryStringExtractor` type to the current route, to extract query parameters into
    /// `State` with the given type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate gotham_derive;
    /// # extern crate hyper;
    /// # extern crate serde;
    /// # #[macro_use]
    /// # extern crate serde_derive;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::state::{State, FromState};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::set::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::test::TestServer;
    /// #
    /// #[derive(StateData, Deserialize, StaticResponseExtender)]
    /// struct MyQueryParams {
    /// #   #[allow(dead_code)]
    ///     id: u64,
    /// }
    ///
    /// fn my_handler(state: State) -> (State, Response) {
    ///     let id = MyQueryParams::borrow_from(&state).id;
    ///
    ///     // Handler implementation elided.
    /// #   assert_eq!(id, 42);
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    /// #
    /// # fn router() -> Router {
    /// #   let pipelines = new_pipeline_set();
    /// #   let (pipelines, default) =
    /// #       pipelines.add(new_pipeline().add(NewSessionMiddleware::default()).build());
    /// #
    /// #   let pipelines = finalize_pipeline_set(pipelines);
    /// #
    /// #   let default_pipeline_chain = (default, ());
    ///
    /// build_router(default_pipeline_chain, pipelines, |route| {
    ///     route.get("/request/path")
    ///          .with_query_string_extractor::<MyQueryParams>()
    ///          .to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/request/path?id=42")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn with_query_string_extractor<NQSE>(
        self,
    ) -> <Self as ReplaceQueryStringExtractor<NQSE>>::Output
    where
        NQSE: QueryStringExtractor + Send + Sync + 'static,
        Self: ReplaceQueryStringExtractor<NQSE>,
        Self::Output: DefineSingleRoute;

    /// ```
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate mime;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use hyper::header::{Accept, qitem};
    /// # use gotham::state::State;
    /// # use gotham::router::route::matcher::AcceptHeaderRouteMatcher;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::test::TestServer;
    /// #
    /// # fn my_handler(state: State) -> (State, Response) {
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     // All we match on is the Accept header, the method is not considered.
    ///     let matcher = AcceptHeaderRouteMatcher::new(vec![mime::APPLICATION_JSON]);
    ///     route.get("/request/path")
    ///          .add_route_matcher(matcher)
    ///          .to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #
    /// #   let accept_header = Accept(vec![
    /// #     qitem(mime::APPLICATION_JSON),
    /// #   ]);
    /// #
    /// #   let text_accept_header = Accept(vec![
    /// #     qitem(mime::TEXT_PLAIN),
    /// #   ]);
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/request/path")
    /// #       .with_header(accept_header)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/request/path")
    /// #       .with_header(text_accept_header)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::NotAcceptable);
    /// # }
    /// ```
    fn add_route_matcher<NRM>(self, matcher: NRM) -> <Self as ExtendRouteMatcher<NRM>>::Output
    where
        NRM: RouteMatcher + Send + Sync + 'static,
        Self: ExtendRouteMatcher<NRM>,
        Self::Output: DefineSingleRoute;
}

impl<'a, M, C, P, PE, QSE> DefineSingleRoute for SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
{
    fn to<H>(self, handler: H)
    where
        H: Handler + RefUnwindSafe + Copy + Send + Sync + 'static,
    {
        self.to_new_handler(move || Ok(handler))
    }

    fn to_new_handler<NH>(self, new_handler: NH)
    where
        NH: NewHandler + 'static,
    {
        let dispatcher = DispatcherImpl::new(new_handler, self.pipeline_chain, self.pipelines);
        let route: RouteImpl<M, PE, QSE> = RouteImpl::new(
            self.matcher,
            Box::new(dispatcher),
            Extractors::new(),
            Delegation::Internal,
        );
        self.node_builder.add_route(Box::new(route));
    }

    fn with_path_extractor<NPE>(self) -> <Self as ReplacePathExtractor<NPE>>::Output
    where
        NPE: PathExtractor + Send + Sync + 'static,
    {
        self.replace_path_extractor()
    }

    fn with_query_string_extractor<NQSE>(
        self,
    ) -> <Self as ReplaceQueryStringExtractor<NQSE>>::Output
    where
        NQSE: QueryStringExtractor + Send + Sync + 'static,
    {
        self.replace_query_string_extractor()
    }

    fn add_route_matcher<NRM>(self, matcher: NRM) -> <Self as ExtendRouteMatcher<NRM>>::Output
    where
        NRM: RouteMatcher + Send + Sync + 'static,
    {
        self.extend_route_matcher(matcher)
    }
}
