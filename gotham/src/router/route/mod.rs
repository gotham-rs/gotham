//! Defines types that support individual application routes.
//!
//! The `Router` will identify one or more `Route` instances that match the path of a request, and
//! iterate to find the first matching `Route` (indicated by `Route::is_match`). The request will
//! be dispatched to the first `Route` which matches.

pub mod dispatch;
pub mod matcher;

use std::marker::PhantomData;
use std::panic::RefUnwindSafe;

use hyper::{Body, Response, Uri};

use extractor::{self, PathExtractor, QueryStringExtractor};
use handler::HandlerFuture;
use helpers::http::request::query_string;
use router::non_match::RouteNonMatch;
use router::route::dispatch::Dispatcher;
use router::route::matcher::RouteMatcher;
use router::tree::segment::SegmentMapping;
use state::{request_id, State};

#[derive(Clone, Copy, PartialEq)]
/// Indicates whether this `Route` will dispatch the request to an inner `Router` instance. To
/// support inner `Router` instances which handle a subtree, the `Dispatcher` stores additional
/// context information.
pub enum Delegation {
    /// This `Route` is dispatching a request to a normal `NewHandler` / `Handler` and does not
    /// need to store any additional context information.
    Internal,

    /// This `Route` is dispatching a request to another `Router` which handles a subtree. The path
    /// segments already consumed by the current `Router` will not be processed again.
    External,
}

/// Values of the `Route` type are used by the `Router` to conditionally dispatch a request after
/// matching the path segments successfully. The steps taken in dispatching to a `Route` are:
///
/// 1. Given a list of routes that match the request path, determine the first `Route` which
///    indicates a match via `Route::is_match`;
/// 2. Determine whether the route's `Delegation` is `Internal` or `External`. If `External`, halt
///    processing and dispatch to the inner `Router`;
/// 3. Run `PathExtractor` and `QueryStringExtractor` logic to popuate `State` with the necessary
///    request data. If either of these extractors fail, the request is halted here;
/// 4. Dispatch the request via `Route::dispatch`.
///
/// `Route` exists as a trait to allow abstraction over the generic types in `RouteImpl`. This
/// trait should not be implemented outside of Gotham.
pub trait Route: RefUnwindSafe {
    /// The type of the response body. The requirements of Hyper are that this implements `Payload`.
    /// Almost always, it will want to be `hyper::Body`.
    type ResBody;
    /// Determines if this `Route` should be invoked, based on the request data in `State.
    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch>;

    /// Determines if this `Route` intends to delegate requests to a secondary `Router` instance.
    fn delegation(&self) -> Delegation;

    /// Extracts dynamic components of the `Request` path and stores the `PathExtractor` in `State`.
    fn extract_request_path<'a>(
        &self,
        state: &mut State,
        params: SegmentMapping<'a>,
    ) -> Result<(), ExtractorFailed>;

    /// Extends the `Response` object when the `PathExtractor` fails.
    fn extend_response_on_path_error(&self, state: &mut State, res: &mut Response<Self::ResBody>);

    /// Extracts the query string parameters and stores the `QueryStringExtractor` in `State`.
    fn extract_query_string(&self, state: &mut State) -> Result<(), ExtractorFailed>;

    /// Extends the `Response` object when query string extraction fails.
    fn extend_response_on_query_string_error(
        &self,
        state: &mut State,
        res: &mut Response<Self::ResBody>,
    );

    /// Dispatches the request to this `Route`, which will execute the pipelines and the handler
    /// assigned to the `Route.
    fn dispatch(&self, state: State) -> Box<HandlerFuture>;
}

/// Returned in the `Err` variant from `extract_query_string` or `extract_request_path`, this
/// signals that the extractor has failed and the request should not proceed.
pub struct ExtractorFailed;

/// Concrete type for a route in a Gotham web application. Values of this type are created by the
/// `gotham::router::builder` API and held internally in the `Router` for dispatching requests.
pub struct RouteImpl<RM, PE, QSE>
where
    RM: RouteMatcher,
    PE: PathExtractor<Body>,
    QSE: QueryStringExtractor<Body>,
{
    matcher: RM,
    dispatcher: Box<Dispatcher + Send + Sync>,
    _extractors: Extractors<PE, QSE>,
    delegation: Delegation,
}

/// Extractors used by `RouteImpl` to acquire request data and change into a type safe form
/// for use by `Middleware` and `Handler` implementations.
pub struct Extractors<PE, QSE>
where
    PE: PathExtractor<Body>,
    QSE: QueryStringExtractor<Body>,
{
    rpe_phantom: PhantomData<PE>,
    qse_phantom: PhantomData<QSE>,
}

impl<RM, PE, QSE> RouteImpl<RM, PE, QSE>
where
    RM: RouteMatcher,
    PE: PathExtractor<Body>,
    QSE: QueryStringExtractor<Body>,
{
    /// Creates a new `RouteImpl` from the provided components.
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
    PE: PathExtractor<Body>,
    QSE: QueryStringExtractor<Body>,
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
    PE: PathExtractor<Body>,
    QSE: QueryStringExtractor<Body>,
{
    type ResBody = Body;

    fn is_match(&self, state: &State) -> Result<(), RouteNonMatch> {
        self.matcher.is_match(state)
    }

    fn delegation(&self) -> Delegation {
        self.delegation
    }

    fn dispatch(&self, state: State) -> Box<HandlerFuture> {
        self.dispatcher.dispatch(state)
    }

    fn extract_request_path<'a>(
        &self,
        state: &mut State,
        params: SegmentMapping<'a>,
    ) -> Result<(), ExtractorFailed> {
        match extractor::internal::from_segment_mapping::<PE>(params) {
            Ok(val) => Ok(state.put(val)),
            Err(e) => {
                debug!("[{}] path extractor failed: {}", request_id(&state), e);
                Err(ExtractorFailed)
            }
        }
    }

    fn extend_response_on_path_error(&self, state: &mut State, res: &mut Response<Self::ResBody>) {
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

    fn extend_response_on_query_string_error(
        &self,
        state: &mut State,
        res: &mut Response<Self::ResBody>,
    ) {
        QSE::extend(state, res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::Async;
    use hyper::{HeaderMap, Method, StatusCode, Uri};
    use std::str::FromStr;

    use extractor::{NoopPathExtractor, NoopQueryStringExtractor};
    use helpers::http::request::path::RequestPathSegments;
    use helpers::http::response::create_empty_response;
    use pipeline::set::*;
    use router::builder::*;
    use router::route::dispatch::DispatcherImpl;
    use router::route::matcher::MethodOnlyRouteMatcher;
    use state::set_request_id;

    #[test]
    fn internal_route_tests() {
        fn handler(state: State) -> (State, Response<Body>) {
            let res = create_empty_response(&state, StatusCode::ACCEPTED);
            (state, res)
        }

        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let methods = vec![Method::GET];
        let matcher = MethodOnlyRouteMatcher::new(methods);
        let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
        let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
        let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        match route.dispatch(state).poll() {
            Ok(Async::Ready((_state, response))) => {
                assert_eq!(response.status(), StatusCode::ACCEPTED)
            }
            Ok(Async::NotReady) => panic!("expected future to be completed already"),
            Err((_state, e)) => panic!("error polling future: {}", e),
        }
    }

    #[test]
    fn external_route_tests() {
        fn handler(state: State) -> (State, Response<Body>) {
            let res = create_empty_response(&state, StatusCode::ACCEPTED);
            (state, res)
        }

        let secondary_router = build_simple_router(|route| {
            route.get("/").to(handler);
        });

        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let methods = vec![Method::GET];
        let matcher = MethodOnlyRouteMatcher::new(methods);
        let dispatcher = Box::new(DispatcherImpl::new(secondary_router, (), pipeline_set));
        let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
        let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::External);

        let mut state = State::new();
        state.put(Method::GET);
        state.put(Uri::from_str("https://example.com/").unwrap());
        state.put(HeaderMap::new());
        state.put(RequestPathSegments::new("/"));
        set_request_id(&mut state);

        match route.dispatch(state).poll() {
            Ok(Async::Ready((_state, response))) => {
                assert_eq!(response.status(), StatusCode::ACCEPTED)
            }
            Ok(Async::NotReady) => panic!("expected future to be completed already"),
            Err((_state, e)) => panic!("error polling future: {}", e),
        }
    }
}
