use std::marker::PhantomData;

use hyper::Method;

use router::route::{Delegation, Extractors, RouteImpl};
use router::route::dispatch::{PipelineHandleChain, PipelineSet, DispatcherImpl};
use router::route::matcher::{RouteMatcher, MethodOnlyRouteMatcher};
use router::request::path::{PathExtractor, NoopPathExtractor};
use router::request::query_string::{QueryStringExtractor, NoopQueryStringExtractor};
use router::builder::{SingleRouteBuilder, ScopeBuilder};
use router::tree::node::{SegmentType, NodeBuilder};

pub type DefaultSingleRouteBuilder<'a, C, P> = SingleRouteBuilder<
    'a,
    MethodOnlyRouteMatcher,
    C,
    P,
    NoopPathExtractor,
    NoopQueryStringExtractor,
>;

/// Defines functions available on builders that are able to define routes.
pub trait DrawRoutes<C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    /// Creates a route which matches `GET` and `HEAD` requests to the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # use hyper::{Request, Response};
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::middleware::pipeline::new_pipeline;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set};
    /// # fn my_handler(_: State, _: Request) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// #   let pipelines = new_pipeline_set();
    /// #   let (pipelines, default) =
    /// #       pipelines.add(new_pipeline().add(NewSessionMiddleware::default()).build());
    /// #
    /// #   let pipelines = finalize_pipeline_set(pipelines);
    /// #
    /// #   let default_pipeline_chain = (default, ());
    /// #
    /// build_router(default_pipeline_chain, pipelines, |route| {
    ///     route.get("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
    /// ```
    fn get<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Get, Method::Head], path)
    }

    /// Creates a route which matches `POST` requests to the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # use hyper::{Request, Response};
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::middleware::pipeline::new_pipeline;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set};
    /// # fn my_handler(_: State, _: Request) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// #   let pipelines = new_pipeline_set();
    /// #   let (pipelines, default) =
    /// #       pipelines.add(new_pipeline().add(NewSessionMiddleware::default()).build());
    /// #
    /// #   let pipelines = finalize_pipeline_set(pipelines);
    /// #
    /// #   let default_pipeline_chain = (default, ());
    /// #
    /// build_router(default_pipeline_chain, pipelines, |route| {
    ///     route.post("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
    /// ```
    fn post<'b>(&'b mut self, path: &str) -> DefaultSingleRouteBuilder<'b, C, P> {
        self.request(vec![Method::Post], path)
    }

    // TODO: Glob paths
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
    /// # use hyper::{Request, Response};
    /// # use hyper::Method::*;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::middleware::pipeline::new_pipeline;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set};
    /// # fn my_handler(_: State, _: Request) -> (State, Response) {
    /// #   unreachable!()
    /// # }
    /// #
    /// # fn router() -> Router {
    /// #   let pipelines = new_pipeline_set();
    /// #   let (pipelines, default) =
    /// #       pipelines.add(new_pipeline().add(NewSessionMiddleware::default()).build());
    /// #
    /// #   let pipelines = finalize_pipeline_set(pipelines);
    /// #
    /// #   let default_pipeline_chain = (default, ());
    /// #
    /// build_router(default_pipeline_chain, pipelines, |route| {
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
            delegation: Delegation::Internal,
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
    /// # use hyper::{Request, Response};
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::middleware::pipeline::new_pipeline;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set};
    /// # mod api {
    /// #   use super::*;
    /// #   pub fn list(_: State, _: Request) -> (State, Response) {
    /// #       unreachable!()
    /// #   }
    /// # }
    /// #
    /// # fn router() -> Router {
    /// #   let pipelines = new_pipeline_set();
    /// #   let (pipelines, default) =
    /// #       pipelines.add(new_pipeline().add(NewSessionMiddleware::default()).build());
    /// #
    /// #   let pipelines = finalize_pipeline_set(pipelines);
    /// #
    /// #   let default_pipeline_chain = (default, ());
    /// #
    /// build_router(default_pipeline_chain, pipelines, |route| {
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

    /// Return the components that comprise this builder. For internal use only.
    #[doc(hidden)]
    fn component_refs(&mut self) -> (&mut NodeBuilder, &mut C, &PipelineSet<P>);
}

fn descend<'n>(node_builder: &'n mut NodeBuilder, path: &str) -> &'n mut NodeBuilder {
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
            println!("router::builder::build_subtree descending into {}", segment);
            let (segment, segment_type) = if segment.starts_with(":") {
                (&segment[1..], SegmentType::Dynamic)
            } else {
                (segment, SegmentType::Static)
            };

            if !node.has_child(segment, segment_type.clone()) {
                let node_builder = NodeBuilder::new(segment, segment_type.clone());
                node.add_child(node_builder);
            }

            let child = node.borrow_mut_child(segment, segment_type).unwrap();
            build_subtree(child, i)
        }
        None => {
            println!("router::builder::build_subtree reached node");
            node
        }
    }
}
