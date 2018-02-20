//! Defines a builder API for constructing a `Router`.

mod draw;
mod single;
mod replace;

use std::marker::PhantomData;
use std::panic::RefUnwindSafe;

use hyper::{Method, StatusCode};

use pipeline::chain::PipelineHandleChain;
use pipeline::set::{finalize_pipeline_set, new_pipeline_set, PipelineSet};
use router::Router;
use router::tree::TreeBuilder;
use router::response::extender::ResponseExtender;
use router::response::finalizer::ResponseFinalizerBuilder;
use router::route::{Delegation, Extractors, RouteImpl};
use router::route::matcher::{MethodOnlyRouteMatcher, RouteMatcher};
use router::route::matcher::any::AnyRouteMatcher;
use router::route::dispatch::DispatcherImpl;
use extractor::{NoopPathExtractor, NoopQueryStringExtractor, PathExtractor, QueryStringExtractor};
use router::tree::node::NodeBuilder;

pub use self::single::DefineSingleRoute;
pub use self::draw::DrawRoutes;
pub use self::replace::{ReplacePathExtractor, ReplaceQueryStringExtractor};

/// The default type returned when building a single associated route. See
/// `router::builder::DefineSingleRoute` for an overview of the ways that a route can be specified.
pub type AssociatedSingleRouteBuilder<'a, C, P, PE, QSE> =
    SingleRouteBuilder<'a, MethodOnlyRouteMatcher, C, P, PE, QSE>;

/// Builds a `Router` using the provided closure. Routes are defined using the `RouterBuilder`
/// value passed to the closure, and the `Router` is constructed before returning.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # #[macro_use]
/// # extern crate serde_derive;
/// #
/// # use hyper::{Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::builder::*;
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::pipeline::single::*;
/// # use gotham::middleware::session::{NewSessionMiddleware, SessionData};
/// # use gotham::test::TestServer;
/// #
/// # #[derive(Serialize, Deserialize, Default)]
/// # struct Session;
/// #
/// # fn my_handler(state: State) -> (State, Response) {
/// #   assert!(state.has::<SessionData<Session>>());
/// #   (state, Response::new().with_status(StatusCode::Accepted))
/// # }
/// #
/// fn router() -> Router {
///     let (chain, pipelines) = single_pipeline(
///         new_pipeline()
///             .add(NewSessionMiddleware::default().with_session_type::<Session>())
///             .build()
///     );
///
///     build_router(chain, pipelines, |route| {
///         route.get("/request/path").to(my_handler);
///     })
/// }
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
pub fn build_router<C, P, F>(pipeline_chain: C, pipelines: PipelineSet<P>, f: F) -> Router
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
    F: FnOnce(&mut RouterBuilder<C, P>),
{
    let mut tree_builder = TreeBuilder::new();

    let response_finalizer = {
        let mut builder = RouterBuilder {
            node_builder: tree_builder.borrow_root_mut(),
            pipeline_chain,
            pipelines,
            response_finalizer_builder: ResponseFinalizerBuilder::new(),
        };

        f(&mut builder);

        builder.response_finalizer_builder.finalize()
    };

    Router::internal_new(tree_builder.finalize(), response_finalizer)
}

/// Builds a `Router` with **no** middleware using the provided closure. Routes are defined using
/// the `RouterBuilder` value passed to the closure, and the `Router` is constructed before
/// returning.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::builder::*;
/// # use gotham::test::TestServer;
/// #
/// # fn my_handler(state: State) -> (State, Response) {
/// #   (state, Response::new().with_status(StatusCode::Accepted))
/// # }
/// #
/// fn router() -> Router {
///     build_simple_router(|route| {
///         route.get("/request/path").to(my_handler);
///     })
/// }
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
pub fn build_simple_router<F>(f: F) -> Router
where
    F: FnOnce(&mut RouterBuilder<(), ()>),
{
    let pipelines = finalize_pipeline_set(new_pipeline_set());
    let pipeline_chain = ();

    build_router(pipeline_chain, pipelines, f)
}

/// The top-level builder which is created by `build_router` and passed to the provided closure.
/// See the `build_router` function and the `DrawRoutes` trait for usage.
pub struct RouterBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    node_builder: &'a mut NodeBuilder,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
    response_finalizer_builder: ResponseFinalizerBuilder,
}

impl<'a, C, P> RouterBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    /// Adds a `ResponseExtender` to the `ResponseFinalizer` in the `Router`.
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use hyper::header::Warning;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::response::extender::ResponseExtender;
    /// # use gotham::router::builder::*;
    /// # use gotham::test::TestServer;
    /// #
    /// # fn my_handler(state: State) -> (State, Response) {
    /// #   (state, Response::new().with_status(StatusCode::InternalServerError))
    /// # }
    /// #
    /// struct MyExtender;
    ///
    /// impl ResponseExtender for MyExtender {
    ///     fn extend(&self, state: &mut State, response: &mut Response) {
    ///         // Extender implementation omitted.
    /// #       let _ = state;
    /// #       response.headers_mut().set(
    /// #           Warning {
    /// #               code: 299,
    /// #               agent: "example.com".to_owned(),
    /// #               text: "Deprecated".to_owned(),
    /// #               date: None,
    /// #           }
    /// #       );
    ///     }
    /// }
    ///
    /// fn router() -> Router {
    ///     build_simple_router(|route| {
    ///         route.add_response_extender(StatusCode::InternalServerError, MyExtender);
    /// #
    /// #       route.get("/").to(my_handler);
    ///     })
    /// }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::InternalServerError);
    /// #
    /// #   {
    /// #       let warning = response.headers().get::<Warning>().unwrap();
    /// #       assert_eq!(warning.code, 299);
    /// #       assert_eq!(warning.agent, "example.com");
    /// #       assert_eq!(warning.text, "Deprecated");
    /// #       assert!(warning.date.is_none());
    /// #   }
    /// # }
    /// ```
    pub fn add_response_extender<E>(&mut self, status_code: StatusCode, extender: E)
    where
        E: ResponseExtender + Send + Sync + 'static,
    {
        self.response_finalizer_builder
            .add(status_code, Box::new(extender))
    }
}

/// A scoped builder, which is created by `DrawRoutes::scope` and passed to the provided closure.
/// The `DrawRoutes` trait has documentation for using this type.
pub struct ScopeBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    node_builder: &'a mut NodeBuilder,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
}

/// A delegated builder, which is created by `DrawRoutes::delegate` and returned. The `DrawRoutes`
/// trait has documentation for using this type.
pub struct DelegateRouteBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    node_builder: &'a mut NodeBuilder,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
}

type DelegatedRoute = RouteImpl<AnyRouteMatcher, NoopPathExtractor, NoopQueryStringExtractor>;

impl<'a, C, P> DelegateRouteBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
{
    /// Directs the delegated route to the given `Router`.
    pub fn to_router(self, router: Router) {
        let dispatcher = DispatcherImpl::new(router, self.pipeline_chain, self.pipelines);
        let route: DelegatedRoute = DelegatedRoute::new(
            AnyRouteMatcher::new(),
            Box::new(dispatcher),
            Extractors::new(),
            Delegation::External,
        );

        self.node_builder.add_route(Box::new(route));
    }
}

/// Implements the traits required to define a single route, after determining which request paths
/// will be dispatched here. The `DefineSingleRoute` trait has documentation for using this type.
pub struct SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
{
    node_builder: &'a mut NodeBuilder,
    matcher: M,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
    phantom: PhantomData<(PE, QSE)>,
}

// Trait impls live with the traits.
impl<'a, M, C, P, PE, QSE> SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
{
    /// Coerces the type of the internal `PhantomData`, to replace an extractor by changing the
    /// type parameter without changing anything else.
    fn coerce<NPE, NQSE>(self) -> SingleRouteBuilder<'a, M, C, P, NPE, NQSE>
    where
        NPE: PathExtractor + Send + Sync + 'static,
        NQSE: QueryStringExtractor + Send + Sync + 'static,
    {
        SingleRouteBuilder {
            node_builder: self.node_builder,
            matcher: self.matcher,
            pipeline_chain: self.pipeline_chain,
            pipelines: self.pipelines,
            phantom: PhantomData,
        }
    }
}

/// Implements the methods required for associating a number of routes with a single path. This is
/// used by `DrawRoutes::associated`.
pub struct AssociatedRouteBuilder<'a, C, P, PE, QSE>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
{
    node_builder: &'a mut NodeBuilder,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
    phantom: PhantomData<(PE, QSE)>,
}

impl<'a, C, P, PE, QSE> AssociatedRouteBuilder<'a, C, P, PE, QSE>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
{
    /// Binds a new `PathExtractor` to the associated routes.
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
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   assert_eq!(state.borrow::<MyPathExtractor>().id, 42);
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #[derive(Deserialize, StateData, StaticResponseExtender)]
    /// struct MyPathExtractor {
    /// #   #[allow(dead_code)]
    ///     id: u32,
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource/:id", |assoc| {
    ///         let mut assoc = assoc.with_path_extractor::<MyPathExtractor>();
    ///         assoc.get().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/resource/42")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn with_path_extractor<'b, NPE>(&'b mut self) -> AssociatedRouteBuilder<'b, C, P, NPE, QSE>
    where
        NPE: PathExtractor + Send + Sync + 'static,
    {
        AssociatedRouteBuilder {
            node_builder: self.node_builder,
            pipeline_chain: self.pipeline_chain,
            pipelines: self.pipelines.clone(),
            phantom: PhantomData,
        }
    }

    /// Binds a new `QueryStringExtractor` to the associated routes.
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
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   assert_eq!(state.borrow::<MyQueryStringExtractor>().val.as_str(), "test_val");
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #[derive(StateData, Deserialize, StaticResponseExtender)]
    /// struct MyQueryStringExtractor {
    /// #   #[allow(dead_code)]
    ///     val: String,
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         let mut assoc = assoc.with_query_string_extractor::<MyQueryStringExtractor>();
    ///         assoc.get().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/resource?val=test_val")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn with_query_string_extractor<'b, NQSE>(
        &'b mut self,
    ) -> AssociatedRouteBuilder<'b, C, P, PE, NQSE>
    where
        NQSE: QueryStringExtractor + Send + Sync + 'static,
    {
        AssociatedRouteBuilder {
            node_builder: self.node_builder,
            pipeline_chain: self.pipeline_chain,
            pipelines: self.pipelines.clone(),
            phantom: PhantomData,
        }
    }

    /// Associates a route which matches requests with any of the specified methods, to the current
    /// path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate mime;
    /// #
    /// # use hyper::{Response, Method, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.request(vec![Method::Get, Method::Head, Method::Post]).to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/resource")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// #
    /// #   let response = test_server.client()
    /// #       .head("https://example.com/resource")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// #
    /// #   let response = test_server.client()
    /// #       .post("https://example.com/resource", b"".to_vec(), mime::TEXT_PLAIN)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn request<'b>(
        &'b mut self,
        methods: Vec<Method>,
    ) -> AssociatedSingleRouteBuilder<'b, C, P, PE, QSE> {
        let AssociatedRouteBuilder {
            ref mut node_builder,
            ref pipeline_chain,
            ref pipelines,
            phantom,
        } = *self;

        let matcher = MethodOnlyRouteMatcher::new(methods);

        SingleRouteBuilder {
            matcher,
            phantom,
            node_builder: *node_builder,
            pipeline_chain: *pipeline_chain,
            pipelines: pipelines.clone(),
        }
    }

    /// Associates a route which matches `HEAD` requests to the current path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.head().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .head("https://example.com/resource")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn head<'b>(&'b mut self) -> AssociatedSingleRouteBuilder<'b, C, P, PE, QSE> {
        self.request(vec![Method::Head])
    }

    /// Associates a route which matches `GET` or `HEAD` requests to the current path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.get_or_head().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/resource")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// #
    /// #   let response = test_server.client()
    /// #       .head("https://example.com/resource")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn get_or_head<'b>(&'b mut self) -> AssociatedSingleRouteBuilder<'b, C, P, PE, QSE> {
        self.request(vec![Method::Get, Method::Head])
    }

    /// Associates a route which matches `GET` requests to the current path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.get().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/resource")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn get<'b>(&'b mut self) -> AssociatedSingleRouteBuilder<'b, C, P, PE, QSE> {
        self.request(vec![Method::Get])
    }

    /// Associates a route which matches `POST` requests to the current path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate mime;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.post().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .post("https://example.com/resource", b"".to_vec(), mime::TEXT_PLAIN)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn post<'b>(&'b mut self) -> AssociatedSingleRouteBuilder<'b, C, P, PE, QSE> {
        self.request(vec![Method::Post])
    }

    /// Associates a route which matches `PUT` requests to the current path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate mime;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.put().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .put("https://example.com/resource", b"".to_vec(), mime::TEXT_PLAIN)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn put<'b>(&'b mut self) -> AssociatedSingleRouteBuilder<'b, C, P, PE, QSE> {
        self.request(vec![Method::Put])
    }

    /// Associates a route which matches `PATCH` requests to the current path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate mime;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.patch().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .patch("https://example.com/resource", b"".to_vec(), mime::TEXT_PLAIN)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn patch<'b>(&'b mut self) -> AssociatedSingleRouteBuilder<'b, C, P, PE, QSE> {
        self.request(vec![Method::Patch])
    }

    /// Associates a route which matches `DELETE` requests to the current path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.delete().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .delete("https://example.com/resource")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn delete<'b>(&'b mut self) -> AssociatedSingleRouteBuilder<'b, C, P, PE, QSE> {
        self.request(vec![Method::Delete])
    }

    /// Associates a route which matches `OPTIONS` requests to the current path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Method, Request, Response, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::state::State;
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> (State, Response) {
    ///     // Implementation elided.
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.options().to(handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let request = Request::new(
    /// #       Method::Options,
    /// #       "https://example.com/resource".parse().unwrap()
    /// #   );
    /// #   let response = test_server.client().perform(request).unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    pub fn options<'b>(&'b mut self) -> AssociatedSingleRouteBuilder<'b, C, P, PE, QSE> {
        self.request(vec![Method::Options])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use hyper::{Method, Request, Response, StatusCode};
    use hyper::server::Service;
    use futures::{Future, Stream};
    use tokio_core::reactor::Core;

    use pipeline::new_pipeline;
    use middleware::session::NewSessionMiddleware;
    use state::{State, StateData};
    use service::GothamService;
    use router::response::extender::StaticResponseExtender;

    #[derive(Deserialize)]
    struct SalutationParams {
        name: String,
    }

    impl StateData for SalutationParams {}

    impl StaticResponseExtender for SalutationParams {
        fn extend(_: &mut State, _: &mut Response) {}
    }

    #[derive(Deserialize)]
    struct AddParams {
        x: u64,
        y: u64,
    }

    impl StateData for AddParams {}

    impl StaticResponseExtender for AddParams {
        fn extend(_: &mut State, _: &mut Response) {}
    }

    mod welcome {
        use super::*;
        pub fn index(state: State) -> (State, Response) {
            (state, Response::new().with_status(StatusCode::Ok))
        }

        pub fn literal(state: State) -> (State, Response) {
            (state, Response::new().with_status(StatusCode::Created))
        }

        pub fn hello(mut state: State) -> (State, Response) {
            let params = state.take::<SalutationParams>();
            let response = Response::new()
                .with_status(StatusCode::Ok)
                .with_body(format!("Hello, {}!", params.name));
            (state, response)
        }

        pub fn globbed(state: State) -> (State, Response) {
            let response = Response::new()
                .with_status(StatusCode::Ok)
                .with_body("Globbed");
            (state, response)
        }

        pub fn delegated(state: State) -> (State, Response) {
            let response = Response::new()
                .with_status(StatusCode::Ok)
                .with_body("Delegated");
            (state, response)
        }

        pub fn goodbye(mut state: State) -> (State, Response) {
            let params = state.take::<SalutationParams>();
            let response = Response::new()
                .with_status(StatusCode::Ok)
                .with_body(format!("Goodbye, {}!", params.name));
            (state, response)
        }

        pub fn add(mut state: State) -> (State, Response) {
            let params = state.take::<AddParams>();
            let response = Response::new()
                .with_status(StatusCode::Ok)
                .with_body(format!(
                    "{} + {} = {}",
                    params.x,
                    params.y,
                    params.x + params.y,
                ));
            (state, response)
        }
    }

    mod resource {
        use super::*;
        pub fn create(state: State) -> (State, Response) {
            let response = Response::new().with_status(StatusCode::Created);
            (state, response)
        }

        pub fn destroy(state: State) -> (State, Response) {
            let response = Response::new().with_status(StatusCode::Accepted);
            (state, response)
        }

        pub fn show(state: State) -> (State, Response) {
            let response = Response::new()
                .with_status(StatusCode::Ok)
                .with_body("It's a resource.");
            (state, response)
        }

        pub fn update(state: State) -> (State, Response) {
            let response = Response::new().with_status(StatusCode::Accepted);
            (state, response)
        }
    }

    mod api {
        use super::*;
        pub fn submit(state: State) -> (State, Response) {
            (state, Response::new().with_status(StatusCode::Accepted))
        }
    }

    #[test]
    fn build_router_test() {
        let pipelines = new_pipeline_set();
        let (pipelines, default) =
            pipelines.add(new_pipeline().add(NewSessionMiddleware::default()).build());

        let pipelines = finalize_pipeline_set(pipelines);

        let default_pipeline_chain = (default, ());

        let delegated_router = build_simple_router(|route| {
            route.get("/b").to(welcome::delegated);
        });

        let router = build_router(default_pipeline_chain, pipelines, |route| {
            route.get("/").to(welcome::index);

            route
                .get("/hello/:name")
                .with_path_extractor::<SalutationParams>()
                .to(welcome::hello);

            route
                .get("/hello/:name/*")
                .with_path_extractor::<SalutationParams>()
                .to(welcome::globbed);

            route
                .get("/goodbye/:name:[a-zA-Z]+")
                .with_path_extractor::<SalutationParams>()
                .to(welcome::goodbye);

            route
                .get("/add")
                .with_query_string_extractor::<AddParams>()
                .to(welcome::add);

            route.get(r"/literal/\:param/\*").to(welcome::literal);

            route.scope("/api", |route| {
                route.post("/submit").to(api::submit);
            });

            route.associate("/resource", |route| {
                route.post().to(resource::create);
                route.patch().to(resource::update);
                route.delete().to(resource::destroy);
                route.get_or_head().to(resource::show);
            });

            route.delegate("/delegated").to_router(delegated_router);
        });

        let mut core = Core::new().unwrap();
        let new_service = GothamService::new(Arc::new(router), core.handle());

        let mut call = move |req| {
            let service = new_service.connect("127.0.0.1:10000".parse().unwrap());
            core.run(service.call(req)).unwrap()
        };

        let response = call(Request::new(Method::Get, "/".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Ok);

        let response = call(Request::new(Method::Post, "/api/submit".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Accepted);

        let response = call(Request::new(Method::Get, "/hello/world".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Ok);
        let response_bytes = response.body().concat2().wait().unwrap().to_vec();
        assert_eq!(&String::from_utf8(response_bytes).unwrap(), "Hello, world!");

        let response = call(Request::new(
            Method::Get,
            "/hello/world/more/path/here/handled/by/glob"
                .parse()
                .unwrap(),
        ));
        assert_eq!(response.status(), StatusCode::Ok);
        let response_bytes = response.body().concat2().wait().unwrap().to_vec();
        assert_eq!(&String::from_utf8(response_bytes).unwrap(), "Globbed");

        let response = call(Request::new(Method::Get, "/delegated/b".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Ok);
        let response_bytes = response.body().concat2().wait().unwrap().to_vec();
        assert_eq!(&String::from_utf8(response_bytes).unwrap(), "Delegated");

        let response = call(Request::new(Method::Get, "/goodbye/world".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Ok);
        let response_bytes = response.body().concat2().wait().unwrap().to_vec();
        assert_eq!(
            &String::from_utf8(response_bytes).unwrap(),
            "Goodbye, world!"
        );

        let response = call(Request::new(Method::Get, "/goodbye/9875".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::NotFound);

        let response = call(Request::new(
            Method::Get,
            "/literal/:param/*".parse().unwrap(),
        ));
        assert_eq!(response.status(), StatusCode::Created);

        let response = call(Request::new(Method::Get, "/literal/a/b".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::NotFound);

        let response = call(Request::new(Method::Get, "/add?x=16&y=71".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Ok);
        let response_bytes = response.body().concat2().wait().unwrap().to_vec();
        assert_eq!(&String::from_utf8(response_bytes).unwrap(), "16 + 71 = 87");

        let response = call(Request::new(Method::Post, "/resource".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Created);

        let response = call(Request::new(Method::Patch, "/resource".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Accepted);

        let response = call(Request::new(Method::Delete, "/resource".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Accepted);

        let response = call(Request::new(Method::Get, "/resource".parse().unwrap()));
        assert_eq!(response.status(), StatusCode::Ok);
        let response_bytes = response.body().concat2().wait().unwrap().to_vec();
        assert_eq!(&response_bytes[..], b"It's a resource.");
    }
}
