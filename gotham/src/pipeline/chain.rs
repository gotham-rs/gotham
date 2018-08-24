//! Defines the types for connecting multiple pipeline handles into a "chain" when constructing the
//! dispatcher for a route.

use borrow_bag::{Handle, Lookup};
use futures::future;
use std::panic::RefUnwindSafe;

use handler::{HandlerFuture, IntoHandlerError};
use middleware::chain::NewMiddlewareChain;
use pipeline::set::PipelineSet;
use pipeline::Pipeline;
use state::{request_id, State};

/// A heterogeneous list of `Handle<P, _>` values, where `P` is a pipeline type. The pipelines are
/// borrowed and invoked in order to serve a request.
///
/// Implemented using nested tuples, with `()` marking the end of the list. The list is in the
/// reverse order of their invocation when a request is dispatched.
///
/// That is:
///
/// `(p3, (p2, (p1, ())))`
///
/// will be invoked as:
///
/// `(state, request)` &rarr; `p1` &rarr; `p2` &rarr; `p3` &rarr; `handler`
pub trait PipelineHandleChain<P>: RefUnwindSafe {
    /// Invokes this part of the `PipelineHandleChain`, with requests being passed through to `f`
    /// once all `Middleware` in the `Pipeline` have passed the request through.
    fn call<F>(&self, pipelines: &PipelineSet<P>, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + Send + 'static;
}

/// Part of a `PipelineHandleChain` which references a `Pipeline` and continues with a tail element.
impl<'a, P, T, N, U> PipelineHandleChain<P> for (Handle<Pipeline<T>, N>, U)
where
    T: NewMiddlewareChain,
    T::Instance: Send + 'static,
    U: PipelineHandleChain<P>,
    P: Lookup<Pipeline<T>, N>,
    N: RefUnwindSafe,
{
    fn call<F>(&self, pipelines: &PipelineSet<P>, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
    {
        let (handle, ref chain) = *self;
        match pipelines.borrow(handle).construct() {
            Ok(p) => chain.call(pipelines, state, move |state| p.call(state, f)),
            Err(e) => {
                trace!("[{}] error borrowing pipeline", request_id(&state));
                Box::new(future::err((state, e.into_handler_error())))
            }
        }
    }
}

/// The marker for the end of a `PipelineHandleChain`.
impl<P> PipelineHandleChain<P> for () {
    fn call<F>(&self, _: &PipelineSet<P>, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
    {
        trace!("[{}] start pipeline", request_id(&state));
        f(state)
    }
}
