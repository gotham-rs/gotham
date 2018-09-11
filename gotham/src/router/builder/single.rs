use std::panic::RefUnwindSafe;

use extractor::{PathExtractor, QueryStringExtractor};
use handler::assets::{DirHandler, FileHandler, FileOptions, FilePathExtractor};
use handler::{Handler, NewHandler};
use hyper::Body;
use pipeline::chain::PipelineHandleChain;
use router::builder::{
    ExtendRouteMatcher, ReplacePathExtractor, ReplaceQueryStringExtractor, SingleRouteBuilder,
};
use router::route::dispatch::DispatcherImpl;
use router::route::matcher::RouteMatcher;
use router::route::{Delegation, Extractors, RouteImpl};

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
/// # use hyper::{Body, Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::builder::*;
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::pipeline::single::*;
/// # use gotham::middleware::session::NewSessionMiddleware;
/// # use gotham::test::TestServer;
/// #
/// fn my_handler(state: State) -> (State, Response<Body>) {
///     // Handler implementation elided.
/// #   (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
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
/// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
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
    /// # use hyper::{Body, Response, StatusCode};
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::single::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::test::TestServer;
    /// #
    /// fn my_handler(state: State) -> (State, Response<Body>) {
    ///     // Handler implementation elided.
    /// #   (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
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
    /// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
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
    /// # use hyper::{Body, Response, StatusCode};
    /// # use futures::future;
    /// # use gotham::handler::{Handler, HandlerFuture, NewHandler};
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::single::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::test::TestServer;
    /// # use gotham::error::*;
    /// #
    /// struct MyNewHandler;
    /// struct MyHandler;
    ///
    /// impl NewHandler for MyNewHandler {
    ///     type Instance = MyHandler;
    ///
    ///     fn new_handler(&self) -> Result<Self::Instance> {
    ///         Ok(MyHandler)
    ///     }
    /// }
    ///
    /// impl Handler for MyHandler {
    ///     fn handle(self, state: State) -> Box<HandlerFuture> {
    ///         // Handler implementation elided.
    /// #       let response = Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap();
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
    /// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
    /// # }
    /// ```
    fn to_new_handler<NH>(self, new_handler: NH)
    where
        NH: NewHandler + 'static;

    /// Directs the route to serve static files from the given root directory.
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
    ///     route.get("/*").to_dir("resources/test/assets");
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/doc.html")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::OK);
    /// # }
    /// ```
    fn to_dir<P>(self, options: P)
    where
        Self: Sized,
        Self: ReplacePathExtractor<FilePathExtractor>,
        Self::Output: DefineSingleRoute,
        FileOptions: From<P>,
    {
        self.with_path_extractor::<FilePathExtractor>()
            .to_new_handler(DirHandler::new(options));
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
    ///     route.get("/").to_file("resources/test/assets/doc.html");
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::OK);
    /// # }
    /// ```
    fn to_file<P>(self, options: P)
    where
        Self: Sized,
        FileOptions: From<P>,
    {
        self.to_new_handler(FileHandler::new(options));
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
    /// # use hyper::{Body, Response, StatusCode};
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
    /// fn my_handler(state: State) -> (State, Response<Body>) {
    /// #   {
    ///     let params = MyPathParams::borrow_from(&state);
    ///
    ///     // Handler implementation elided.
    /// #   assert_eq!(params.name, "world");
    /// #   }
    /// #   (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
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
    /// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
    /// # }
    /// ```
    fn with_path_extractor<NPE>(self) -> <Self as ReplacePathExtractor<NPE>>::Output
    where
        NPE: PathExtractor<Body> + Send + Sync + 'static,
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
    /// # use hyper::{Body, Response, StatusCode};
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
    /// fn my_handler(state: State) -> (State, Response<Body>) {
    ///     let id = MyQueryParams::borrow_from(&state).id;
    ///
    ///     // Handler implementation elided.
    /// #   assert_eq!(id, 42);
    /// #   (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
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
    /// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
    /// # }
    /// ```
    fn with_query_string_extractor<NQSE>(
        self,
    ) -> <Self as ReplaceQueryStringExtractor<NQSE>>::Output
    where
        NQSE: QueryStringExtractor<Body> + Send + Sync + 'static,
        Self: ReplaceQueryStringExtractor<NQSE>,
        Self::Output: DefineSingleRoute;

    /// Adds additional `RouteMatcher` requirements to the current route.
    ///
    /// ```
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate mime;
    /// #
    /// # use hyper::{Body, Response, StatusCode};
    /// # use hyper::header::ACCEPT;
    /// # use gotham::state::State;
    /// # use gotham::router::route::matcher::AcceptHeaderRouteMatcher;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::test::TestServer;
    /// #
    /// # fn my_handler(state: State) -> (State, Response<Body>) {
    /// #   (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
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
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/request/path")
    /// #       .with_header(ACCEPT, mime::APPLICATION_JSON.to_string().parse().unwrap())
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/request/path")
    /// #       .with_header(ACCEPT, mime::TEXT_PLAIN.to_string().parse().unwrap())
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::NOT_ACCEPTABLE);
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
    PE: PathExtractor<Body> + Send + Sync + 'static,
    QSE: QueryStringExtractor<Body> + Send + Sync + 'static,
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
        NPE: PathExtractor<Body> + Send + Sync + 'static,
    {
        self.replace_path_extractor()
    }

    fn with_query_string_extractor<NQSE>(
        self,
    ) -> <Self as ReplaceQueryStringExtractor<NQSE>>::Output
    where
        NQSE: QueryStringExtractor<Body> + Send + Sync + 'static,
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
