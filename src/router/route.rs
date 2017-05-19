//! Defines a `Route` type and a default implementation.
//!
//! The Gotham `Router` having identified one or more potential `Route` instances to service a
//! request via route `Tree` traversal will attempt to identify a matching `Route` and
//! dispatch to it when it does so.

use hyper::server::Request;
use borrow_bag::BorrowBag;

use dispatch::{PipelineHandleChain, Dispatcher};
use handler::{HandlerFuture, NewHandler};
use http::request_path::RequestPathExtractor;
use router::request_matcher::RequestMatcher;
use router::tree::SegmentMapping;
use state::State;

/// A type that determines if its associated logic can be exposed by the `Router`
/// in response to an external request.
pub trait Route<P> {
    /// Determines if this `Route` can be invoked, based on the `Request`.
    fn is_match(&self, req: &Request) -> bool;

    /// Extracts the `Request` path into a Struct for use by Middleware and Handlers
    fn extract_request_path(&self, state: &mut State, segment_mapping: SegmentMapping);

    /// Final call made by the `Router` to the matched `Route` allowing
    /// application specific logic to respond to the request.
    fn dispatch(&self, pipelines: &BorrowBag<P>, state: State, req: Request) -> Box<HandlerFuture>;
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
/// # fn main() {
/// # use hyper::server::{Request, Response};
/// # use hyper::Method;
/// # use gotham::http::request_path::noop_request_path_extractor as noop;
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::dispatch::Dispatcher;
/// # use gotham::state::State;
/// # use gotham::router::route::RouteImpl;
/// #
///   fn handler(state: State, _req: Request) -> (State, Response) {
///     (state, Response::new())
///   }
///
///   let methods = vec![Method::Get];
///   let matcher = MethodOnlyRequestMatcher::new(methods);
///   let dispatcher: Dispatcher<_, _, ()> = Dispatcher::new(|| Ok(handler), ());
///   RouteImpl::new(matcher, dispatcher, Box::new(noop));
/// # }
/// ```
pub struct RouteImpl<RM, NH, PC, P>
    where RM: RequestMatcher,
          NH: NewHandler,
          PC: PipelineHandleChain<P>
{
    matcher: RM,
    dispatcher: Dispatcher<NH, PC, P>,

    request_path_extractor: RequestPathExtractor,
}

impl<RM, NH, PC, P> RouteImpl<RM, NH, PC, P>
    where RM: RequestMatcher,
          NH: NewHandler,
          PC: PipelineHandleChain<P>
{
    /// Creates a new `RouteImpl`
    pub fn new(matcher: RM,
               dispatcher: Dispatcher<NH, PC, P>,
               request_path_extractor: RequestPathExtractor)
               -> Self {
        RouteImpl {
            matcher,
            dispatcher,
            request_path_extractor,
        }
    }
}

impl<RM, NH, PC, P> Route<P> for RouteImpl<RM, NH, PC, P>
    where RM: RequestMatcher,
          NH: NewHandler,
          NH::Instance: 'static,
          PC: PipelineHandleChain<P>
{
    fn is_match(&self, req: &Request) -> bool {
        self.matcher.is_match(req)
    }

    fn dispatch(&self, pipelines: &BorrowBag<P>, state: State, req: Request) -> Box<HandlerFuture> {
        self.dispatcher.dispatch(pipelines, state, req)
    }

    fn extract_request_path(&self, state: &mut State, segment_mapping: SegmentMapping) {
        (self.request_path_extractor)(state, segment_mapping);
    }
}
