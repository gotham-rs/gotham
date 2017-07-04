//! Defines a `Route` type and a default implementation.
//!
//! The Gotham `Router` having identified one or more potential `Route` instances to service a
//! request via route `Tree` traversal will attempt to identify a matching `Route` and
//! dispatch to it when it does so.

use std::marker::PhantomData;

use hyper::server::Request;
use hyper::StatusCode;

use dispatch::Dispatcher;
use handler::HandlerFuture;
use router::request_matcher::RequestMatcher;
use router::tree::SegmentMapping;
use http::request_path::RequestPathExtractor;
use http::query_string::{QueryStringExtractor, QueryStringMapping};
use state::State;

/// A type that determines if its associated logic can be exposed by the `Router`
/// in response to an external request.
pub trait Route {
    /// Determines if this `Route` can be invoked, based on the `Request`.
    fn is_match(&self, state: &State, req: &Request) -> Result<(), StatusCode>;

    /// Determines if this `Route` intends to delegate to secondary `Router` instance.
    fn is_delegating(&self) -> bool;

    /// Extracts the `Request` path into a Struct and stores it in `State`  for use
    /// by Middleware and Handlers
    fn extract_request_path(&self,
                            state: &mut State,
                            segment_mapping: SegmentMapping)
                            -> Result<(), String>;

    /// Extracts the `Request` query string into a Struct and stores it in `State`  for use
    /// by Middleware and Handlers
    fn extract_query_string(&self,
                            state: &mut State,
                            query_string_mapping: QueryStringMapping)
                            -> Result<(), String>;

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
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::server::{Request, Response};
/// # use hyper::Method;
/// #
/// # use gotham::http::request_path::NoopRequestPathExtractor;
/// # use gotham::http::query_string::NoopQueryStringExtractor;
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
/// # use gotham::state::State;
/// # use gotham::router::route::{RouteImpl, Extractors};
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
///   RouteImpl::new(matcher, dispatcher, extractors, false);
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
    delegating: bool,
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
               delegating: bool)
               -> Self {
        RouteImpl {
            matcher,
            dispatcher,
            _extractors,
            delegating,
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

    fn is_delegating(&self) -> bool {
        self.delegating
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

    fn extract_query_string(&self,
                            state: &mut State,
                            query_string_mapping: QueryStringMapping)
                            -> Result<(), String> {
        QE::extract(state, query_string_mapping)
    }
}
