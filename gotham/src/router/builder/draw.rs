use std::marker::PhantomData;
use std::panic::RefUnwindSafe;

use hyper::Method;

use router::route::dispatch::{PipelineHandleChain, PipelineSet};
use router::route::matcher::MethodOnlyRouteMatcher;
use router::request::path::NoopPathExtractor;
use router::request::query_string::NoopQueryStringExtractor;
use router::builder::{AssociatedRouteBuilder, DelegateRouteBuilder, RouterBuilder, ScopeBuilder,
                      SingleRouteBuilder};
use router::tree::node::{NodeBuilder, SegmentType};
use router::tree::regex::ConstrainedSegmentRegex;

/// The default type returned when building a single route. See
/// `router::builder::DefineSingleRoute` for an overview of the ways that a route can be specified.
pub type DefaultSingleRouteBuilder<'a, C, P> = SingleRouteBuilder<
    'a,
    MethodOnlyRouteMatcher,
    C,
    P,
    NoopPathExtractor,
    NoopQueryStringExtractor,
>;

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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # fn my_handler(_: State) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.get_or_head("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # fn my_handler(_: State) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.get("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # fn my_handler(_: State) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.head("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # fn my_handler(_: State) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.post("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # fn my_handler(_: State) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.put("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # fn my_handler(_: State) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.patch("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # fn my_handler(_: State) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.delete("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # fn my_handler(_: State) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.options("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use hyper::Method::*;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # fn my_handler(_: State) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.request(vec![Get, Head], "/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
    /// ```
    fn request<'b>(
        &'b mut self,
        methods: Vec<Method>,
        path: &str,
    ) -> DefaultSingleRouteBuilder<'b, C, P> {
        let (node_builder, pipeline_chain, pipelines) = self.component_refs();
        let node_builder = descend(node_builder, path);

        let matcher = MethodOnlyRouteMatcher::new(methods);

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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # mod api {
    /// #   use super::*;
    /// #   pub fn list(_: State) -> (State, Response) {
    /// #       unreachable!()
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
    /// # fn main() { router(); }
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

    /// Begins delegating a subpath of the tree.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// #
    /// fn admin_router() -> Router {
    ///     // Implementation elided
    /// #   build_simple_router(|_route| {})
    /// }
    ///
    /// # fn router() -> Router {
    /// build_simple_router(|route| {
    ///     route.delegate("/admin").to_router(admin_router());
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::single::single_pipeline;
    /// # use gotham::state::State;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// #
    /// # #[derive(Default, Serialize, Deserialize)]
    /// # struct Session;
    /// #
    /// // API routes which don't require sessions.
    /// fn api_router() -> Router {
    ///     // Implementation elided
    /// #   build_simple_router(|_route| {})
    /// }
    /// # fn handler(_state: State) -> (State, Response) {
    /// #   unimplemented!()
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
    /// # fn main() { router(); }
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

    fn associate<'b, F>(&'b mut self, path: &str, f: F)
    where
        F: FnOnce(&mut AssociatedRouteBuilder<'b, C, P>),
    {
        let (node_builder, pipeline_chain, pipelines) = self.component_refs();
        let node_builder = descend(node_builder, path);

        let mut builder = AssociatedRouteBuilder {
            node_builder,
            pipeline_chain: *pipeline_chain,
            pipelines: pipelines.clone(),
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
