//! Defines types that support individual application routes.
//!
//! The Gotham `Router` having identified one or more potential `Route` instances to service a
//! request via route `Tree` traversal will attempt to identify a matching `Route` and
//! dispatch to it when it does so.

pub mod request_matcher;

use std::marker::PhantomData;

use hyper::server::Request;
use hyper::StatusCode;

use dispatch::Dispatcher;
use handler::HandlerFuture;
use router::request::query_string::QueryStringExtractor;
use router::route::request_matcher::RequestMatcher;
use router::tree::SegmentMapping;
use router::request::path::RequestPathExtractor;
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

    /// Extracts the `Request` path into a Struct and stores it in `State`  for use
    /// by Middleware and Handlers
    fn extract_request_path(&self,
                            state: &mut State,
                            segment_mapping: SegmentMapping)
                            -> Result<(), String>;

    /// Extracts the `Request` query string into a Struct and stores it in `State`  for use
    /// by Middleware and Handlers
    fn extract_query_string(&self, state: &mut State, query: Option<&str>) -> Result<(), String>;

    /// Final call made by the `Router` to the matched `Route` allowing
    /// application specific logic to respond to the request.
    fn dispatch(&self, state: State, req: Request) -> Box<HandlerFuture>;
}

/// Default implementation for `Route`.
///
/// Delegates `is_match` to `RequestMatcher` and `dispatch` to `Dispatcher`
/// without any additional involvement.
///
/// # Examples
///
/// ## Standard `Route` which calls application code
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::server::{Request, Response};
/// # use hyper::Method;
/// #
/// # use gotham::router::request::path::NoopRequestPathExtractor;
/// # use gotham::router::request::query_string::NoopQueryStringExtractor;
/// # use gotham::router::route::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
/// # use gotham::state::State;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// #
/// # fn main() {
///   fn handler(state: State, _req: Request) -> (State, Response) {
///     (state, Response::new())
///   }
///
///   let pipeline_set = finalize_pipeline_set(new_pipeline_set());
///   let methods = vec![Method::Get];
///   let matcher = MethodOnlyRequestMatcher::new(methods);
///   let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
///   let extractors: Extractors<NoopRequestPathExtractor, NoopQueryStringExtractor> = Extractors::new();
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
/// # use hyper::server::{Request, Response};
/// # use hyper::Method;
/// #
/// # use gotham::router::request::path::NoopRequestPathExtractor;
/// # use gotham::router::request::query_string::NoopQueryStringExtractor;
/// # use gotham::router::route::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::response::finalizer::ResponseFinalizerBuilder;
/// #
/// # fn main() {
///   fn handler(state: State, _req: Request) -> (State, Response) {
///     (state, Response::new())
///   }
///
///   let secondary_router = {
///        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
///        let mut tree_builder = TreeBuilder::new();
///
///        let route = {
///            let methods = vec![Method::Get];
///            let matcher = MethodOnlyRequestMatcher::new(methods);
///            let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
///            let extractors: Extractors<NoopRequestPathExtractor,
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
///   let matcher = MethodOnlyRequestMatcher::new(methods);
///   let dispatcher = Box::new(DispatcherImpl::new(secondary_router, (), pipeline_set));
///   let extractors: Extractors<NoopRequestPathExtractor, NoopQueryStringExtractor> = Extractors::new();
///   RouteImpl::new(matcher, dispatcher, extractors, Delegation::External);
/// # }
/// ```
pub struct RouteImpl<RM, RE, QE>
    where RM: RequestMatcher,
          RE: RequestPathExtractor,
          QE: QueryStringExtractor
{
    matcher: RM,
    dispatcher: Box<Dispatcher + Send + Sync>,
    _extractors: Extractors<RE, QE>,
    delegation: Delegation,
}

/// Extractors used by `RouteImpl` to acquire request data and change into a type safe form
/// for use by custom `Middleware` and `Handler` implementations.
pub struct Extractors<RE, QE>
    where RE: RequestPathExtractor,
          QE: QueryStringExtractor
{
    rpe_phantom: PhantomData<RE>,
    qse_phantom: PhantomData<QE>,
}

impl<RM, RE, QE> RouteImpl<RM, RE, QE>
    where RM: RequestMatcher,
          RE: RequestPathExtractor,
          QE: QueryStringExtractor
{
    /// Creates a new `RouteImpl`
    pub fn new(matcher: RM,
               dispatcher: Box<Dispatcher + Send + Sync>,
               _extractors: Extractors<RE, QE>,
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

impl<RE, QE> Extractors<RE, QE>
    where RE: RequestPathExtractor,
          QE: QueryStringExtractor
{
    /// Creates a new set of Extractors for use with a `RouteImpl`
    pub fn new() -> Self {
        Extractors {
            rpe_phantom: PhantomData,
            qse_phantom: PhantomData,
        }
    }
}

impl<RM, RE, QE> Route for RouteImpl<RM, RE, QE>
    where RM: RequestMatcher,
          RE: RequestPathExtractor,
          QE: QueryStringExtractor
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

    fn extract_query_string(&self, state: &mut State, query: Option<&str>) -> Result<(), String> {
        QE::extract(state, query)
    }
}
