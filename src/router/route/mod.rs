//! Defines types that support individual application routes.
//!
//! The Gotham `Router` having identified one or more potential `Route` instances to service a
//! request via route `Tree` traversal will attempt to identify a matching `Route` and
//! dispatch to it when it does so.

pub mod matcher;
pub mod dispatch;

use std::marker::PhantomData;

use hyper::server::{Request, Response};
use hyper::StatusCode;

use router::route::dispatch::Dispatcher;
use handler::HandlerFuture;
use router::request::query_string::QueryStringExtractor;
use router::route::matcher::RouteMatcher;
use router::tree::SegmentMapping;
use router::request::path::PathExtractor;
use state::State;

#[derive(Clone, Copy, PartialEq)]
/// Indicates how this Route behaves in relation to external `Router` instances.
pub enum Delegation {
    /// Invokes a handler that is considered 'Internal' to the current `Router`+`Route` instance,
    /// this is generally true of all application implemented handlers.
    Internal,

    /// Invokes an external `Router` as the handler for requests handled by this `Route`. This is
    /// useful when supporting "Umbrella Applications". The external `Router` will not have access to
    /// any `Request` path segment processed in order to arrive at the current `Route`.
    External,
}

/// A type that determines if its associated logic can be exposed by the `Router`
/// in response to an external request.
///
/// Capable of delegating requests to secondary `Router` instances in order to support "Umbrella
/// Applications".
pub trait Route {
    /// Determines if this `Route` can be invoked, based on the `Request`.
    fn is_match(&self, state: &State, req: &Request) -> Result<(), StatusCode>;

    /// Determines if this `Route` intends to delegate requests to a secondary `Router` instance.
    fn delegation(&self) -> Delegation;

    /// Extracts the `Request` path and stores it in `State`
    fn extract_request_path(&self,
                            state: &mut State,
                            segment_mapping: SegmentMapping)
                            -> Result<(), String>;

    /// Extends the `Response` object when path extraction fails
    fn extend_response_on_path_error(&self, state: &mut State, res: &mut Response);

    /// Extracts the `Request` query string and stores it in `State`
    fn extract_query_string(&self, state: &mut State, query: Option<&str>) -> Result<(), String>;

    /// Extends the `Response` object when query string extraction fails
    fn extend_response_on_query_string_error(&self, state: &mut State, res: &mut Response);

    /// Final call made by the `Router` to the matched `Route` allowing
    /// application specific logic to respond to the request.
    fn dispatch(&self, state: State, req: Request) -> Box<HandlerFuture>;
}

/// Default implementation for `Route`.
///
/// # Examples
///
/// ## Standard `Route` which calls application code
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Request, Response, Method, StatusCode};
/// #
/// # use gotham::http::response::create_response;
/// # use gotham::router::request::path::NoopPathExtractor;
/// # use gotham::router::request::query_string::NoopQueryStringExtractor;
/// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
/// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
/// # use gotham::state::State;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// #
/// # fn main() {
///   fn handler(state: State, _req: Request) -> (State, Response) {
///     let res = create_response(&state, StatusCode::Ok, None);
///     (state, res)
///   }
///
///   let pipeline_set = finalize_pipeline_set(new_pipeline_set());
///   let methods = vec![Method::Get];
///   let matcher = MethodOnlyRouteMatcher::new(methods);
///   let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
///   let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
///   RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
/// # }
/// ```
///
/// ## A `Route` which delegates remaining `Request` details to a secondary `Router` instance
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Request, Response, StatusCode, Method};
/// #
/// # use gotham::http::response::create_response;
/// # use gotham::router::request::path::NoopPathExtractor;
/// # use gotham::router::request::query_string::NoopQueryStringExtractor;
/// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
/// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::response::finalizer::ResponseFinalizerBuilder;
/// #
/// # fn main() {
///   fn handler(state: State, _req: Request) -> (State, Response) {
///     let res = create_response(&state, StatusCode::Ok, None);
///     (state, res)
///   }
///
///   let secondary_router = {
///        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
///        let mut tree_builder = TreeBuilder::new();
///
///        let route = {
///            let methods = vec![Method::Get];
///            let matcher = MethodOnlyRouteMatcher::new(methods);
///            let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
///            let extractors: Extractors<NoopPathExtractor,
///                                       NoopQueryStringExtractor> = Extractors::new();
///            let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
///            Box::new(route)
///        };
///        tree_builder.add_route(route);
///
///        let tree = tree_builder.finalize();
///        Router::new(tree, ResponseFinalizerBuilder::new().finalize())
///   };
///
///   let pipeline_set = finalize_pipeline_set(new_pipeline_set());
///   let methods = vec![Method::Get];
///   let matcher = MethodOnlyRouteMatcher::new(methods);
///   let dispatcher = Box::new(DispatcherImpl::new(secondary_router, (), pipeline_set));
///   let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
///   RouteImpl::new(matcher, dispatcher, extractors, Delegation::External);
/// # }
/// ```
pub struct RouteImpl<RM, RE, QSE>
    where RM: RouteMatcher,
          RE: PathExtractor,
          QSE: QueryStringExtractor
{
    matcher: RM,
    dispatcher: Box<Dispatcher + Send + Sync>,
    _extractors: Extractors<RE, QSE>,
    delegation: Delegation,
}

/// Extractors used by `RouteImpl` to acquire request data and change into a type safe form
/// for use by custom `Middleware` and `Handler` implementations.
pub struct Extractors<RE, QSE>
    where RE: PathExtractor,
          QSE: QueryStringExtractor
{
    rpe_phantom: PhantomData<RE>,
    qse_phantom: PhantomData<QSE>,
}

impl<RM, RE, QSE> RouteImpl<RM, RE, QSE>
    where RM: RouteMatcher,
          RE: PathExtractor,
          QSE: QueryStringExtractor
{
    /// Creates a new `RouteImpl`
    pub fn new(matcher: RM,
               dispatcher: Box<Dispatcher + Send + Sync>,
               _extractors: Extractors<RE, QSE>,
               delegation: Delegation)
               -> Self {
        RouteImpl {
            matcher,
            dispatcher,
            _extractors,
            delegation,
        }
    }
}

impl<RE, QSE> Extractors<RE, QSE>
    where RE: PathExtractor,
          QSE: QueryStringExtractor
{
    /// Creates a new set of Extractors for use with a `RouteImpl`
    pub fn new() -> Self {
        Extractors {
            rpe_phantom: PhantomData,
            qse_phantom: PhantomData,
        }
    }
}

impl<RM, RE, QSE> Route for RouteImpl<RM, RE, QSE>
    where RM: RouteMatcher,
          RE: PathExtractor,
          QSE: QueryStringExtractor
{
    fn is_match(&self, state: &State, req: &Request) -> Result<(), StatusCode> {
        self.matcher.is_match(state, req)
    }

    fn delegation(&self) -> Delegation {
        self.delegation
    }

    fn dispatch(&self, state: State, req: Request) -> Box<HandlerFuture> {
        self.dispatcher.dispatch(state, req)
    }

    fn extract_request_path(&self,
                            state: &mut State,
                            segment_mapping: SegmentMapping)
                            -> Result<(), String> {
        RE::extract(state, segment_mapping)
    }

    fn extend_response_on_path_error(&self, state: &mut State, res: &mut Response) {
        RE::extend(state, res)
    }

    fn extract_query_string(&self, state: &mut State, query: Option<&str>) -> Result<(), String> {
        QSE::extract(state, query)
    }

    fn extend_response_on_query_string_error(&self, state: &mut State, res: &mut Response) {
        QSE::extend(state, res)
    }
}
