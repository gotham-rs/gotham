//! Defines types that support individual application routes.
//!
//! The Gotham `Router` having identified `1..n` potential `Route` instances to service a
//! request via route `Tree` traversal will attempt to identify a matching `Route` instance and
//! dispatch to it when it does so.

pub mod matcher;
pub mod dispatch;

use std::marker::PhantomData;
use std::panic::RefUnwindSafe;

use hyper::{Response, Uri};

use handler::HandlerFuture;
use http::request::query_string;
use extractor::{self, PathExtractor, QueryStringExtractor};
use router::non_match::RouteNonMatch;
use router::route::dispatch::Dispatcher;
use router::route::matcher::RouteMatcher;
use router::tree::SegmentMapping;
use state::{request_id, State};

#[derive(Clone, Copy, PartialEq)]
/// Indicates how this Route behaves in relation to external `Router` instances.
pub enum Delegation {
    /// Invokes a `Handler` that is considered 'internal' to the current `Router`+`Route` instance,
    /// this is generally true of all application implemented handlers.
    Internal,

    /// Invokes an external `Router` as the `Handler` for `Requests` matched by this `Route`. This is
    /// useful when supporting "Modular Applications". The external `Router` will not have access to
    /// any `Request` path segments processed in order to arrive at the current `Route`.
    External,
}

/// A type that determines if its associated logic can be exposed by the `Router`
/// in response to an external request. If it determines that it can the `Route` runs extractors on
/// the `Request`, potentially extending `State` before dispatching to the `Dispatcher` assigned
/// to this `Route`.
///
/// Capable of delegating requests to secondary `Router` instances in order to support "Modular
/// Applications".
pub trait Route: RefUnwindSafe {
    /// Determines if this `Route` can be invoked, based on the `Request`.
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch>;

    /// Determines if this `Route` intends to delegate requests to a secondary `Router` instance.
    fn delegation(&self) -> Delegation;

    /// Extracts the `Request` path and stores it in `State`
    fn extract_request_path(
        &self,
        state: &mut State,
        segment_mapping: SegmentMapping,
    ) -> Result<(), ExtractorFailed>;

    /// Extends the `Response` object when path extraction fails
    fn extend_response_on_path_error(&self, state: &mut State, res: &mut Response);

    /// Extracts the `Request` query string and stores it in `State`
    fn extract_query_string(&self, state: &mut State) -> Result<(), ExtractorFailed>;

    /// Extends the `Response` object when query string extraction fails
    fn extend_response_on_query_string_error(&self, state: &mut State, res: &mut Response);

    /// Final call made by the `Router` to the matched `Route` allowing
    /// application specific logic to respond to the request.
    fn dispatch(&self, state: State) -> Box<HandlerFuture>;
}

/// Returned in the `Err` variant from `extract_query_string` or `extract_request_path`, this
/// signals that the extractor has failed and the request should not proceed.
pub struct ExtractorFailed;

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
/// # use hyper::{Response, Method, StatusCode};
/// #
/// # use gotham::http::response::create_response;
/// # use gotham::extractor::{NoopPathExtractor, NoopQueryStringExtractor};
/// # use gotham::pipeline::set::*;
/// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
/// # use gotham::router::route::dispatch::DispatcherImpl;
/// # use gotham::state::State;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// #
/// # fn main() {
///   fn handler(state: State) -> (State, Response) {
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
/// # use hyper::{Response, StatusCode, Method};
/// #
/// # use gotham::http::response::create_response;
/// # use gotham::extractor::{NoopPathExtractor, NoopQueryStringExtractor};
/// # use gotham::pipeline::set::*;
/// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
/// # use gotham::router::route::dispatch::DispatcherImpl;
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::response::finalizer::ResponseFinalizerBuilder;
/// #
/// # fn main() {
///   fn handler(state: State) -> (State, Response) {
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
pub struct RouteImpl<RM, PE, QSE>
where
    RM: RouteMatcher,
    PE: PathExtractor,
    QSE: QueryStringExtractor,
{
    matcher: RM,
    dispatcher: Box<Dispatcher + Send + Sync>,
    _extractors: Extractors<PE, QSE>,
    delegation: Delegation,
}

/// Extractors used by `RouteImpl` to acquire request data and change into a type safe form
/// for use by custom `Middleware` and `Handler` implementations.
pub struct Extractors<PE, QSE>
where
    PE: PathExtractor,
    QSE: QueryStringExtractor,
{
    rpe_phantom: PhantomData<PE>,
    qse_phantom: PhantomData<QSE>,
}

impl<RM, PE, QSE> RouteImpl<RM, PE, QSE>
where
    RM: RouteMatcher,
    PE: PathExtractor,
    QSE: QueryStringExtractor,
{
    /// Creates a new `RouteImpl`
    pub fn new(
        matcher: RM,
        dispatcher: Box<Dispatcher + Send + Sync>,
        _extractors: Extractors<PE, QSE>,
        delegation: Delegation,
    ) -> Self {
        RouteImpl {
            matcher,
            dispatcher,
            _extractors,
            delegation,
        }
    }
}

impl<PE, QSE> Extractors<PE, QSE>
where
    PE: PathExtractor,
    QSE: QueryStringExtractor,
{
    /// Creates a new set of Extractors for use with a `RouteImpl`
    pub fn new() -> Self {
        Extractors {
            rpe_phantom: PhantomData,
            qse_phantom: PhantomData,
        }
    }
}

impl<RM, PE, QSE> Route for RouteImpl<RM, PE, QSE>
where
    RM: RouteMatcher,
    PE: PathExtractor,
    QSE: QueryStringExtractor,
{
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        self.matcher.is_match(state)
    }

    fn delegation(&self) -> Delegation {
        self.delegation
    }

    fn dispatch(&self, state: State) -> Box<HandlerFuture> {
        self.dispatcher.dispatch(state)
    }

    fn extract_request_path(
        &self,
        state: &mut State,
        segment_mapping: SegmentMapping,
    ) -> Result<(), ExtractorFailed> {
        match extractor::internal::from_segment_mapping::<PE>(segment_mapping) {
            Ok(val) => Ok(state.put(val)),
            Err(e) => {
                debug!("[{}] path extractor failed: {}", request_id(&state), e);
                Err(ExtractorFailed)
            }
        }
    }

    fn extend_response_on_path_error(&self, state: &mut State, res: &mut Response) {
        PE::extend(state, res)
    }

    fn extract_query_string(&self, state: &mut State) -> Result<(), ExtractorFailed> {
        let result: Result<QSE, _> = {
            let uri = state.borrow::<Uri>();
            let query_string_mapping = query_string::split(uri.query());
            extractor::internal::from_query_string_mapping(&query_string_mapping)
        };

        match result {
            Ok(val) => Ok(state.put(val)),
            Err(e) => {
                debug!(
                    "[{}] query string extractor failed: {}",
                    request_id(&state),
                    e
                );
                Err(ExtractorFailed)
            }
        }
    }

    fn extend_response_on_query_string_error(&self, state: &mut State, res: &mut Response) {
        QSE::extend(state, res)
    }
}
