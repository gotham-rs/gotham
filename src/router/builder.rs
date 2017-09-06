#![allow(warnings)]

use std::marker::PhantomData;

use hyper::Method;

use handler::{Handler, NewHandler};
use router::Router;
use router::tree::TreeBuilder;
use router::response::finalizer::ResponseFinalizerBuilder;
use router::route::{Delegation, Extractors, RouteImpl};
use router::route::matcher::{RouteMatcher, MethodOnlyRouteMatcher};
use router::route::dispatch::{PipelineHandleChain, PipelineSet, DispatcherImpl};
use router::request::path::{PathExtractor, NoopPathExtractor};
use router::request::query_string::{QueryStringExtractor, NoopQueryStringExtractor};
use router::tree::node::{SegmentType, NodeBuilder};

/// Builds a `Router` using the provided closure. Routes are defined using the `RouterBuilder`
/// value passed to the closure, and the `Router` is constructed before returning.
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
/// fn router() -> Router {
///     let pipelines = new_pipeline_set();
///     let (pipelines, default) =
///         pipelines.add(new_pipeline().add(NewSessionMiddleware::default()).build());
///
///     let pipelines = finalize_pipeline_set(pipelines);
///
///     let default_pipeline_chain = (default, ());
///
///     build_router(default_pipeline_chain, pipelines, |route| {
///         route.get("/request/path").to(my_handler);
///     })
/// }
/// # fn main() { router(); }
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

    Router::new(tree_builder.finalize(), response_finalizer)
}

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

pub struct ScopeBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    node_builder: &'a mut NodeBuilder,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
}

type DefaultSingleRouteBuilder<'a, C, P> = SingleRouteBuilder<
    'a,
    MethodOnlyRouteMatcher,
    C,
    P,
    NoopPathExtractor,
    NoopQueryStringExtractor,
>;

impl<'a, C, P> DrawRoutes<C, P> for RouterBuilder<'a, C, P>
where
    C: PipelineHandleChain<P>
        + Copy
        + Send
        + Sync
        + 'static,
    P: Send + Sync + 'static,
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
    C: PipelineHandleChain<P>
        + Copy
        + Send
        + Sync
        + 'static,
    P: Send + Sync + 'static,
{
    fn component_refs(&mut self) -> (&mut NodeBuilder, &mut C, &PipelineSet<P>) {
        (
            &mut self.node_builder,
            &mut self.pipeline_chain,
            &self.pipelines,
        )
    }
}

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
    delegation: Delegation,
    phantom: PhantomData<(PE, QSE)>,
}

impl<'a, M, C, P, PE, QSE> SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher
        + Send
        + Sync
        + 'static,
    C: PipelineHandleChain<P>
        + Send
        + Sync
        + 'static,
    P: Send + Sync + 'static,
    PE: PathExtractor
        + Send
        + Sync
        + 'static,
    QSE: QueryStringExtractor
        + Send
        + Sync
        + 'static,
{
    pub fn to<H>(self, handler: H)
    where
        H: Handler + Copy + Send + Sync + 'static,
    {
        self.to_new_handler(move || Ok(handler))
    }

    pub fn to_new_handler<NH>(self, new_handler: NH)
    where
        NH: NewHandler + 'static,
    {
        let dispatcher = DispatcherImpl::new(new_handler, self.pipeline_chain, self.pipelines);
        let route: RouteImpl<M, PE, QSE> = RouteImpl::new(
            self.matcher,
            Box::new(dispatcher),
            Extractors::new(),
            self.delegation,
        );
        self.node_builder.add_route(Box::new(route));
    }

    pub fn with_path_params<NPE>(self) -> SingleRouteBuilder<'a, M, C, P, NPE, QSE>
    where
        NPE: PathExtractor + Send + Sync + 'static,
    {
        self.coerce()
    }

    pub fn with_query_params<NQSE>(self) -> SingleRouteBuilder<'a, M, C, P, PE, NQSE>
    where
        NQSE: QueryStringExtractor + Send + Sync + 'static,
    {
        self.coerce()
    }

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
            delegation: self.delegation,
            phantom: PhantomData,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::{Request, Response, StatusCode, Method};
    use hyper::server::{NewService, Service};
    use futures::Future;

    use middleware::pipeline::new_pipeline;
    use middleware::session::NewSessionMiddleware;
    use state::State;
    use handler::{Handler, NewHandlerService};
    use router::route::dispatch::{new_pipeline_set, finalize_pipeline_set};

    mod welcome {
        use super::*;
        pub fn index(state: State, req: Request) -> (State, Response) {
            (state, Response::new().with_status(StatusCode::Ok))
        }
    }

    mod api {
        use super::*;
        pub fn submit(state: State, req: Request) -> (State, Response) {
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

        let router = build_router(default_pipeline_chain, pipelines, |route| {
            route.get("/").to(welcome::index);
            route.scope("/api", |route| { route.post("/submit").to(api::submit); });
        });

        let new_service = NewHandlerService::new(router);

        let service = new_service.new_service().unwrap();

        let response = service
            .call(Request::new(Method::Get, "/".parse().unwrap()))
            .wait()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let service = new_service.new_service().unwrap();

        let response = service
            .call(Request::new(Method::Post, "/api/submit".parse().unwrap()))
            .wait()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Accepted);
    }
}
