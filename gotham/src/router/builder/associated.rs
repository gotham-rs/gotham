use std::marker::PhantomData;
use std::panic::RefUnwindSafe;

use hyper::Method;

use pipeline::chain::PipelineHandleChain;
use pipeline::set::PipelineSet;
use router::route::matcher::{AndRouteMatcher, AnyRouteMatcher, MethodOnlyRouteMatcher,
                             RouteMatcher};
use extractor::{PathExtractor, QueryStringExtractor};
use router::tree::node::NodeBuilder;
use router::builder::SingleRouteBuilder;

pub type AssociatedRouteBuilderMatcher<M, NM> = AndRouteMatcher<M, NM>;
pub type AssociatedRouteMatcher<M> = AndRouteMatcher<MethodOnlyRouteMatcher, M>;

/// The default type returned when building a single associated route. See
/// `router::builder::DefineSingleRoute` for an overview of the ways that a route can be specified.
pub type AssociatedSingleRouteBuilder<'a, M, C, P, PE, QSE> =
    SingleRouteBuilder<'a, M, C, P, PE, QSE>;

/// Implements the methods required for associating a number of routes with a single path. This is
/// used by `DrawRoutes::associated`.
pub struct AssociatedRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
{
    node_builder: &'a mut NodeBuilder,
    matcher: M,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
    phantom: PhantomData<(PE, QSE)>,
}

impl<'a, C, P, PE, QSE> AssociatedRouteBuilder<'a, AnyRouteMatcher, C, P, PE, QSE>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
{
    /// Create an instance of AssociatedRouteBuilder
    pub fn new(
        node_builder: &'a mut NodeBuilder,
        pipeline_chain: C,
        pipelines: PipelineSet<P>,
    ) -> Self {
        AssociatedRouteBuilder {
            node_builder,
            matcher: AnyRouteMatcher::new(),
            pipeline_chain,
            pipelines,
            phantom: PhantomData,
        }
    }
}

impl<'a, M, C, P, PE, QSE> AssociatedRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
{
    /// Adds aadditional `RouteMatcher` requirements to all subsequently associated routes.
    ///
    /// # Examples
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
    ///     let matcher = AcceptHeaderRouteMatcher::new(vec![mime::APPLICATION_JSON]);
    ///
    ///     route.associate("/resource/path", |assoc| {
    ///         let mut assoc = assoc.add_route_matcher(matcher);
    ///
    ///         assoc.get().to(my_handler);
    ///     });
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
    /// #       .get("https://example.com/resource/path")
    /// #       .with_header(accept_header)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::Accepted);
    /// #
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/resource/path")
    /// #       .with_header(text_accept_header)
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::NotAcceptable);
    /// # }
    /// ```
    pub fn add_route_matcher<'b, NM>(
        &'b mut self,
        matcher: NM,
    ) -> AssociatedRouteBuilder<'b, AssociatedRouteBuilderMatcher<M, NM>, C, P, PE, QSE>
    where
        NM: RouteMatcher + Send + Sync + 'static,
    {
        let matcher = AndRouteMatcher::new(self.matcher.clone(), matcher);
        AssociatedRouteBuilder {
            node_builder: self.node_builder,
            matcher,
            pipeline_chain: self.pipeline_chain,
            pipelines: self.pipelines.clone(),
            phantom: PhantomData,
        }
    }

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
    pub fn with_path_extractor<'b, NPE>(
        &'b mut self,
    ) -> AssociatedRouteBuilder<'b, M, C, P, NPE, QSE>
    where
        NPE: PathExtractor + Send + Sync + 'static,
    {
        AssociatedRouteBuilder {
            node_builder: self.node_builder,
            matcher: self.matcher.clone(),
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
    ) -> AssociatedRouteBuilder<'b, M, C, P, PE, NQSE>
    where
        NQSE: QueryStringExtractor + Send + Sync + 'static,
    {
        AssociatedRouteBuilder {
            node_builder: self.node_builder,
            matcher: self.matcher.clone(),
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
    ) -> AssociatedSingleRouteBuilder<'b, AssociatedRouteMatcher<M>, C, P, PE, QSE> {
        let AssociatedRouteBuilder {
            ref mut node_builder,
            ref matcher,
            ref pipeline_chain,
            ref pipelines,
            phantom,
        } = *self;

        SingleRouteBuilder {
            node_builder: *node_builder,
            matcher: AndRouteMatcher::new(MethodOnlyRouteMatcher::new(methods), matcher.clone()),
            pipeline_chain: *pipeline_chain,
            pipelines: pipelines.clone(),
            phantom,
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
    pub fn head<'b>(
        &'b mut self,
    ) -> AssociatedSingleRouteBuilder<'b, AssociatedRouteMatcher<M>, C, P, PE, QSE> {
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
    pub fn get_or_head<'b>(
        &'b mut self,
    ) -> AssociatedSingleRouteBuilder<'b, AssociatedRouteMatcher<M>, C, P, PE, QSE> {
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
    pub fn get<'b>(
        &'b mut self,
    ) -> AssociatedSingleRouteBuilder<'b, AssociatedRouteMatcher<M>, C, P, PE, QSE> {
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
    pub fn post<'b>(
        &'b mut self,
    ) -> AssociatedSingleRouteBuilder<'b, AssociatedRouteMatcher<M>, C, P, PE, QSE> {
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
    pub fn put<'b>(
        &'b mut self,
    ) -> AssociatedSingleRouteBuilder<'b, AssociatedRouteMatcher<M>, C, P, PE, QSE> {
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
    pub fn patch<'b>(
        &'b mut self,
    ) -> AssociatedSingleRouteBuilder<'b, AssociatedRouteMatcher<M>, C, P, PE, QSE> {
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
    pub fn delete<'b>(
        &'b mut self,
    ) -> AssociatedSingleRouteBuilder<'b, AssociatedRouteMatcher<M>, C, P, PE, QSE> {
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
    pub fn options<'b>(
        &'b mut self,
    ) -> AssociatedSingleRouteBuilder<'b, AssociatedRouteMatcher<M>, C, P, PE, QSE> {
        self.request(vec![Method::Options])
    }
}
