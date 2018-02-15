use std::panic::RefUnwindSafe;

use extractor::{PathExtractor, QueryStringExtractor};
use pipeline::chain::PipelineHandleChain;
use router::builder::SingleRouteBuilder;
use router::builder::replace::{ReplacePathExtractor, ReplaceQueryStringExtractor};
use router::route::{Delegation, Extractors, RouteImpl};
use router::route::matcher::RouteMatcher;
use router::route::dispatch::DispatcherImpl;
use handler::{Handler, NewHandler};

/// Describes the API for defining a single route, after determining which request paths will be
/// dispatched here. The API here uses chained function calls to build and add the route into the
/// `RouterBuilder` which created it.
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
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::middleware::session::NewSessionMiddleware;
/// # use gotham::pipeline::set::*;
/// #
/// fn my_handler(_: State) -> (State, Response) {
///     // Handler implementation elided.
/// #   unimplemented!()
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
///     route.get("/request/path") // <- This value implements `DefineSingleRoute`
///          .to(my_handler);
/// })
/// # }
/// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::set::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// #
    /// fn my_handler(_: State) -> (State, Response) {
    ///     // Handler implementation elided.
    /// #   unimplemented!()
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
    ///     route.get("/request/path").to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use std::io;
    /// # use gotham::handler::{Handler, HandlerFuture, NewHandler};
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::set::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
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
    ///     fn handle(self, _state: State) -> Box<HandlerFuture> {
    ///         // Handler implementation elided.
    /// #       unimplemented!()
    ///     }
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
    ///     route.get("/request/path").to_new_handler(MyNewHandler);
    /// })
    /// # }
    /// # fn main() { router(); }
    /// ```
    fn to_new_handler<NH>(self, new_handler: NH)
    where
        NH: NewHandler + 'static;

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
    /// # use hyper::Response;
    /// # use gotham::state::{State, FromState};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::set::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// #
    /// #[derive(Deserialize, StateData, StaticResponseExtender)]
    /// struct MyPathParams {
    /// #   #[allow(dead_code)]
    ///     name: String,
    /// }
    ///
    /// fn my_handler(state: State) -> (State, Response) {
    /// #   #[allow(unused_variables)]
    ///     let params = MyPathParams::borrow_from(&state);
    ///
    ///     // Handler implementation elided.
    /// #   unimplemented!()
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
    ///          .with_path_extractor::<MyPathParams>()
    ///          .to(my_handler);
    /// })
    /// # }
    /// # fn main() { router(); }
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
    /// # use hyper::Response;
    /// # use gotham::state::{State, FromState};
    /// # use gotham::router::Router;
    /// # use gotham::router::builder::*;
    /// # use gotham::pipeline::new_pipeline;
    /// # use gotham::pipeline::set::*;
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// #
    /// #[derive(StateData, Deserialize, StaticResponseExtender)]
    /// struct MyQueryParams {
    /// #   #[allow(dead_code)]
    ///     id: u64,
    /// }
    ///
    /// fn my_handler(state: State) -> (State, Response) {
    /// #   #[allow(unused_variables)]
    ///     let id = MyQueryParams::borrow_from(&state).id;
    ///
    ///     // Handler implementation elided.
    /// #   unimplemented!()
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
    /// # fn main() { router(); }
    /// ```
    fn with_query_string_extractor<NQSE>(
        self,
    ) -> <Self as ReplaceQueryStringExtractor<NQSE>>::Output
    where
        NQSE: QueryStringExtractor + Send + Sync + 'static,
        Self: ReplaceQueryStringExtractor<NQSE>,
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
}
