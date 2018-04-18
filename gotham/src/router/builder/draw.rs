use std::marker::PhantomData;
use std::panic::RefUnwindSafe;

use hyper::Method;

use pipeline::chain::PipelineHandleChain;
use pipeline::set::PipelineSet;
use router::route::matcher::{IntoRouteMatcher, MethodOnlyRouteMatcher, RouteMatcher};
use extractor::{NoopPathExtractor, NoopQueryStringExtractor};
use router::builder::{AssociatedRouteBuilder, DelegateRouteBuilder, RouterBuilder, ScopeBuilder,
                      SingleRouteBuilder};
use router::tree::node::{NodeBuilder, SegmentType};
use router::tree::regex::ConstrainedSegmentRegex;

/// The type returned when building a route that only considers path and http verb(s) when
/// determining if it matches a request.
///
/// See `router::builder::DefineSingleRoute` for an overview of route specification.
pub type DefaultSingleRouteBuilder<'a, C, P> = SingleRouteBuilder<
    'a,
    MethodOnlyRouteMatcher,
    C,
    P,
    NoopPathExtractor,
    NoopQueryStringExtractor,
>;

/// The type returned when building a route with explicit matching requirements.
///
/// See `router::builder::DefineSingleRoute` for an overview of route specification.
pub type ExplicitSingleRouteBuilder<'a, M, C, P> =
    SingleRouteBuilder<'a, M, C, P, NoopPathExtractor, NoopQueryStringExtractor>;

/// The type passed to the function used when building associated routes. See
/// `AssociatedRouteBuilder` for information about the API available for associated routes.
pub type DefaultAssociatedRouteBuilder<'a, C, P> =
    AssociatedRouteBuilder<'a, C, P, NoopPathExtractor, NoopQueryStringExtractor>;

/// Defines functions used by a builder to determine which request paths will be dispatched to a
/// route. This trait is implemented by the top-level `RouterBuilder`, and also the `ScopedBuilder`
/// created by `DrawRoutes::scope`.
pub trait DrawRoutes<C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
{
    /// Creates a route which matches `GET` and `HEAD` requests to the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
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
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.get_or_head("/request/path").to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/request/path")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// #
    /// #   let response = test_server.client()
    /// #       .head("https://example.com/request/path")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn get_or_head<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Get, Method::Head], path)
    }

    /// Creates a route which matches **only** `GET` requests to the given path (ignoring `HEAD`
    /// requests).
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
    /// # use gotham::test::TestServer;
    /// #
    /// # fn my_handler(state: State) -> (State, Response) {
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.get("/request/path").to(my_handler);
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
    fn get<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Get], path)
    }

    /// Creates a route which matches `HEAD` requests to the given path.
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
    /// # use gotham::test::TestServer;
    /// #
    /// # fn my_handler(state: State) -> (State, Response) {
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.head("/request/path").to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .head("https://example.com/request/path")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn head<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Head], path)
    }

    /// Creates a route which matches `POST` requests to the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate mime;
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
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.post("/request/path").to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .post("https://example.com/request/path", b"".to_vec(), mime::TEXT_PLAIN)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn post<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Post], path)
    }

    /// Creates a route which matches `PUT` requests to the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate mime;
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
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.put("/request/path").to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .put("https://example.com/request/path", b"".to_vec(), mime::TEXT_PLAIN)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn put<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Put], path)
    }

    /// Creates a route which matches `PATCH` requests to the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate mime;
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
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.patch("/request/path").to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .patch("https://example.com/request/path", b"".to_vec(), mime::TEXT_PLAIN)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn patch<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Patch], path)
    }

    /// Creates a route which matches `DELETE` requests to the given path.
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
    /// # use gotham::test::TestServer;
    /// #
    /// # fn my_handler(state: State) -> (State, Response) {
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.delete("/request/path").to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .delete("https://example.com/request/path")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn delete<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Delete], path)
    }

    /// Creates a route which matches `OPTIONS` requests to the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Method, Request, Response, StatusCode};
    /// # use gotham::state::State;
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
    ///     route.options("/request/path").to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let request = Request::new(
    /// #       Method::Options,
    /// #       "https://example.com/request/path".parse().unwrap()
    /// #   );
    /// #   let response = test_server.client().perform(request).unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn options<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Options], path)
    }

    /// Creates a single route which matches any requests to the given `path` with one of the
    /// given `methods`. The `path` can consist of static or dynamic segments, for example:
    ///
    /// * `"/hello/world"` - a static path, matching only a request for exactly `"/hello/world"`
    /// * `"/hello/:name"` - a dynamic path, matching requests for `"/hello/any_value_here"`
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use hyper::Method::*;
    /// # use gotham::state::State;
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
    ///     route.request(vec![Get, Head], "/request/path").to(my_handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/request/path")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// #
    /// #   let response = test_server.client()
    /// #       .head("https://example.com/request/path")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    ///
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
    ///     route.request(matcher, "/request/path").to(my_handler);
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
    /// #
    /// #   // No Accept type being provided is valid for the AcceptHeaderRouterMatcher
    /// #   // Proves the method is not considered
    /// #   let response = test_server.client()
    /// #       .delete("https://example.com/request/path")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn request<'b, IRM, M>(
        &'b mut self,
        matcher: IRM,
        path: &str,
    ) -> ExplicitSingleRouteBuilder<'b, M, C, P>
    where
        IRM: IntoRouteMatcher<Output = M>,
        M: RouteMatcher + Send + Sync + 'static,
    {
        let (node_builder, pipeline_chain, pipelines) = self.component_refs();
        let node_builder = descend(node_builder, path);
        let matcher = matcher.into_route_matcher();

        SingleRouteBuilder {
            matcher,
            node_builder,
            pipeline_chain: *pipeline_chain,
            pipelines: pipelines.clone(),
            phantom: PhantomData,
        }
    }

    /// Begins defining a new scope, based on a given `path` prefix.
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
    /// # use gotham::test::TestServer;
    /// #
    /// # mod api {
    /// #   use super::*;
    /// #   pub fn list(state: State) -> (State, Response) {
    /// #       (state, Response::new().with_status(StatusCode::Accepted))
    /// #   }
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.scope("/api", |route| {
    ///         // Match requests to `/api/list`
    ///         route.get("/list").to(api::list);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/api/list")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn scope<F>(&mut self, path: &str, f: F)
    where
        F: FnOnce(&mut ScopeBuilder<C, P>),
    {
        let (node_builder, pipeline_chain, pipelines) = self.component_refs();
        let node_builder = descend(node_builder, path);

        let mut scope_builder = ScopeBuilder {
            node_builder,
            pipeline_chain: *pipeline_chain,
            pipelines: pipelines.clone(),
        };

        f(&mut scope_builder)
    }

    /// Begins a new scope at the current location, with an alternate pipeline chain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # #[macro_use]
    /// # extern crate serde_derive;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::state::State;
    /// # use gotham::middleware::session::{NewSessionMiddleware, SessionData};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::set::{finalize_pipeline_set, new_pipeline_set};
    /// # use gotham::test::TestServer;
    /// #
    /// # #[derive(Default, Serialize, Deserialize)]
    /// # struct Session;
    /// #
    /// # #[derive(Default, Serialize, Deserialize)]
    /// # struct AdminSession;
    /// #
    /// # mod resource {
    /// #   use super::*;
    /// #   pub fn list(state: State) -> (State, Response) {
    /// #       assert!(state.has::<SessionData<Session>>());
    /// #       assert!(!state.has::<SessionData<AdminSession>>());
    /// #       (state, Response::new().with_status(StatusCode::Accepted))
    /// #   }
    /// # }
    /// #
    /// # mod admin {
    /// #   use super::*;
    /// #   pub fn handler(state: State) -> (State, Response) {
    /// #       assert!(state.has::<SessionData<Session>>());
    /// #       assert!(state.has::<SessionData<AdminSession>>());
    /// #       (state, Response::new().with_status(StatusCode::Accepted))
    /// #   }
    /// # }
    /// #
    /// # fn handler(state: State) -> (State, Response) {
    /// #   assert!(!state.has::<SessionData<Session>>());
    /// #   assert!(!state.has::<SessionData<AdminSession>>());
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// # }
    /// #
    /// # fn router() -> Router {
    /// let pipelines = new_pipeline_set();
    /// let (pipelines, default) = pipelines.add(
    ///     new_pipeline()
    ///         .add(NewSessionMiddleware::default().with_session_type::<Session>())
    ///         .build()
    /// );
    /// let (pipelines, extended) = pipelines.add(
    ///     new_pipeline()
    ///         .add(NewSessionMiddleware::default().with_session_type::<AdminSession>())
    ///         .build()
    /// );
    /// let pipeline_set = finalize_pipeline_set(pipelines);
    ///
    /// let default_chain = (default, ());
    /// let extended_chain = (extended, default_chain);
    ///
    /// build_router(default_chain, pipeline_set, |route| {
    ///     // Requests for the root handler use an empty set of pipelines, skipping the session
    ///     // middlewares.
    ///     route.with_pipeline_chain((), |route| {
    ///         route.get("/").to(handler);
    ///     });
    ///
    ///     // Requests dispatched to the resource module will only invoke one session
    ///     // middleware which is the default behavior.
    ///     route.get("/resource/list").to(resource::list);
    ///
    ///     // Requests for the admin handler will additionally invoke the admin session
    ///     // middleware.
    ///     route.with_pipeline_chain(extended_chain, |route| {
    ///         route.get("/admin").to(admin::handler);
    ///     });
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/resource/list")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn with_pipeline_chain<F, NC>(&mut self, pipeline_chain: NC, f: F)
    where
        F: FnOnce(&mut ScopeBuilder<NC, P>),
        NC: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    {
        let (node_builder, _pipeline_chain, pipelines) = self.component_refs();

        let mut scope_builder = ScopeBuilder {
            node_builder,
            pipeline_chain,
            pipelines: pipelines.clone(),
        };

        f(&mut scope_builder)
    }

    /// Begins delegating a subpath of the tree.
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
    /// fn admin_router() -> Router {
    ///     // Implementation elided
    /// #   fn handler(state: State) -> (State, Response) {
    /// #       (state, Response::new().with_status(StatusCode::Accepted))
    /// #   }
    /// #
    /// #   build_simple_router(|route| {
    /// #       route.get("/").to(handler);
    /// #   })
    /// }
    ///
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.delegate("/admin").to_router(admin_router());
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/admin")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn delegate<'b>(&'b mut self, path: &str) -> DelegateRouteBuilder<'b, C, P> {
        let (node_builder, pipeline_chain, pipelines) = self.component_refs();
        let node_builder = descend(node_builder, path);

        DelegateRouteBuilder {
            node_builder,
            pipeline_chain: *pipeline_chain,
            pipelines: pipelines.clone(),
        }
    }

    /// Begins delegating a subpath of the tree, but does not dispatch the requests via this
    /// router's `PipelineChain`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # #[macro_use]
    /// # extern crate serde_derive;
    /// #
    /// # use hyper::{Response, StatusCode};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::single::single_pipeline;
    /// # use gotham::state::State;
    /// # use gotham::middleware::session::{NewSessionMiddleware, SessionData};
    /// # use gotham::test::TestServer;
    /// #
    /// # #[derive(Default, Serialize, Deserialize)]
    /// # struct Session;
    /// #
    /// // API routes which don't require sessions.
    /// fn api_router() -> Router {
    ///     // Implementation elided
    /// #   fn handler(state: State) -> (State, Response) {
    /// #       assert!(!state.has::<SessionData<Session>>());
    /// #       (state, Response::new().with_status(StatusCode::Accepted))
    /// #   }
    /// #
    /// #   build_simple_router(|route| {
    /// #       route.get("/").to(handler);
    /// #   })
    /// }
    /// # fn handler(state: State) -> (State, Response) {
    /// #   assert!(state.has::<SessionData<Session>>());
    /// #   (state, Response::new().with_status(StatusCode::Accepted))
    /// # }
    ///
    /// # fn router() -> Router {
    /// let (chain, pipelines) = single_pipeline(
    ///     new_pipeline()
    ///         .add(NewSessionMiddleware::default().with_session_type::<Session>())
    ///         .build()
    /// );
    ///
    /// build_router(chain, pipelines, |route| {
    ///     // Requests dispatched to the `/api` router will not invoke the session middleware.
    ///     route.delegate_without_pipelines("/api").to_router(api_router());
    ///
    ///     // Other requests will invoke the session middleware as normal.
    ///     route.get("/").to(handler);
    /// })
    /// # }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/api")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn delegate_without_pipelines<'b>(&'b mut self, path: &str) -> DelegateRouteBuilder<'b, (), P> {
        let (node_builder, _pipeline_chain, pipelines) = self.component_refs();
        let node_builder = descend(node_builder, path);

        DelegateRouteBuilder {
            node_builder,
            pipeline_chain: (),
            pipelines: pipelines.clone(),
        }
    }

    /// Begins associating routes with a fixed path in the tree. In this way, multiple routes can
    /// be quickly associated with a single location.
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
    /// mod resource {
    /// #   use super::*;
    ///     pub fn show(state: State) -> (State, Response) {
    ///         // Implementation elided.
    /// #       (state, Response::new().with_status(StatusCode::NoContent))
    ///     }
    ///
    ///     pub fn update(state: State) -> (State, Response) {
    ///         // Implementation elided.
    /// #       (state, Response::new().with_status(StatusCode::Created))
    ///     }
    ///
    ///     pub fn delete(state: State) -> (State, Response) {
    ///         // Implementation elided.
    /// #       (state, Response::new().with_status(StatusCode::Accepted))
    ///     }
    /// }
    ///
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.associate("/resource", |assoc| {
    ///         assoc.get_or_head().to(resource::show);
    ///         assoc.patch().to(resource::update);
    ///         assoc.delete().to(resource::delete);
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
    /// #   assert_eq!(response.status(), StatusCode::NoContent);
    /// #
    /// #   let response = test_server.client()
    /// #       .patch("https://example.com/resource", b"".to_vec(), mime::TEXT_PLAIN)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Created);
    /// #
    /// #   let response = test_server.client()
    /// #       .delete("https://example.com/resource")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// # }
    /// ```
    fn associate<'b, F>(&'b mut self, path: &str, f: F)
    where
        F: FnOnce(&mut DefaultAssociatedRouteBuilder<'b, C, P>),
    {
        let (node_builder, pipeline_chain, pipelines) = self.component_refs();
        let node_builder = descend(node_builder, path);

        let mut builder = AssociatedRouteBuilder {
            node_builder,
            pipeline_chain: *pipeline_chain,
            pipelines: pipelines.clone(),
            phantom: PhantomData,
        };

        f(&mut builder)
    }

    /// Return the components that comprise this builder. For internal use only.
    #[doc(hidden)]
    fn component_refs(&mut self) -> (&mut NodeBuilder, &mut C, &PipelineSet<P>);
}

fn descend<'n>(node_builder: &'n mut NodeBuilder, path: &str) -> &'n mut NodeBuilder {
    trace!("[walking to: {}]", path);

    let path = if path.starts_with("/") {
        &path[1..]
    } else {
        path
    };

    if path.is_empty() {
        node_builder
    } else {
        build_subtree(node_builder, path.split("/"))
    }
}

fn build_subtree<'n, 's, I>(node: &'n mut NodeBuilder, mut i: I) -> &'n mut NodeBuilder
where
    I: Iterator<Item = &'s str>,
{
    match i.next() {
        Some(segment) => {
            trace!("[descending into {}]", segment);

            let (segment, segment_type) = match segment.chars().next() {
                Some(':') => {
                    let segment = &segment[1..];
                    match segment.find(":") {
                        Some(n) => {
                            let (segment, pattern) = segment.split_at(n);
                            let regex = ConstrainedSegmentRegex::new(&pattern[1..]);
                            (segment, SegmentType::Constrained { regex })
                        }
                        None => (segment, SegmentType::Dynamic),
                    }
                }
                Some('*') if segment.len() == 1 => (segment, SegmentType::Glob),
                Some('\\') => (&segment[1..], SegmentType::Static),
                _ => (segment, SegmentType::Static),
            };

            if !node.has_child(segment, segment_type.clone()) {
                let node_builder = NodeBuilder::new(segment, segment_type.clone());
                node.add_child(node_builder);
            }

            let child = node.borrow_mut_child(segment, segment_type).unwrap();
            build_subtree(child, i)
        }
        None => {
            trace!("[reached node]");
            node
        }
    }
}

impl<'a, C, P> DrawRoutes<C, P> for RouterBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
{
    fn component_refs(&mut self) -> (&mut NodeBuilder, &mut C, &PipelineSet<P>) {
        (
            &mut self.node_builder,
            &mut self.pipeline_chain,
            &self.pipelines,
        )
    }
}

impl<'a, C, P> DrawRoutes<C, P> for ScopeBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
{
    fn component_refs(&mut self) -> (&mut NodeBuilder, &mut C, &PipelineSet<P>) {
        (
            &mut self.node_builder,
            &mut self.pipeline_chain,
            &self.pipelines,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use hyper::{Response, StatusCode};
    use futures::future;

    use handler::HandlerFuture;
    use middleware::{Middleware, NewMiddleware};
    use state::State;
    use router::builder::*;
    use pipeline::*;
    use pipeline::single::*;
    use http::response::create_response;
    use test::TestServer;

    #[derive(Clone, Copy)]
    struct QuickExitMiddleware;

    impl NewMiddleware for QuickExitMiddleware {
        type Instance = Self;

        fn new_middleware(&self) -> io::Result<Self> {
            Ok(*self)
        }
    }

    impl Middleware for QuickExitMiddleware {
        fn call<Chain>(self, state: State, _chain: Chain) -> Box<HandlerFuture>
        where
            Chain: FnOnce(State) -> Box<HandlerFuture> + 'static,
        {
            let f = future::ok((
                state,
                Response::new().with_status(StatusCode::InternalServerError),
            ));

            Box::new(f)
        }
    }

    fn test_handler(state: State) -> (State, Response) {
        let response = create_response(&state, StatusCode::Accepted, None);
        (state, response)
    }

    #[test]
    fn delegate_includes_pipelines() {
        let (chain, pipelines) = single_pipeline(new_pipeline().add(QuickExitMiddleware).build());

        let test_router = build_simple_router(|route| {
            route.get("/").to(test_handler);
        });

        let router = build_router(chain, pipelines, |route| {
            route.delegate("/test").to_router(test_router);
        });

        let test_server = TestServer::new(router).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/test/")
            .perform()
            .unwrap();
        assert_eq!(response.status(), StatusCode::InternalServerError);
    }

    #[test]
    fn delegate_without_pipelines_skips_pipelines() {
        let (chain, pipelines) = single_pipeline(new_pipeline().add(QuickExitMiddleware).build());

        let test_router = build_simple_router(|route| {
            route.get("/").to(test_handler);
        });

        let router = build_router(chain, pipelines, |route| {
            route
                .delegate_without_pipelines("/test")
                .to_router(test_router);
        });

        let test_server = TestServer::new(router).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/test/")
            .perform()
            .unwrap();
        assert_eq!(response.status(), StatusCode::Accepted);
    }
}
