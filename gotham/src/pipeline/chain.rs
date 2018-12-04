//! Defines the types for connecting multiple pipeline handles into a "chain" when constructing the
//! dispatcher for a route.

use borrow_bag::{Handle, Lookup};
use futures::future;
use std::panic::RefUnwindSafe;

use error::Result;
use handler::{Handler, HandlerFuture, IntoHandlerError, NewHandler};
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

/// Creates a `NewHandler` implementation, who's generated handlers will first process the pipeline
/// and finally call the passed handler.
pub fn new_handler_with_pipeline<C, P, NH>(
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
    new_handler: NH,
) -> NewHandlerWithPipeline<C, P, NH>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + RefUnwindSafe + 'static,
    NH: NewHandler,
    NH::Instance: 'static,
{
    NewHandlerWithPipeline {
        pipeline_chain,
        pipelines,
        new_handler,
    }
}

/// `NewHandler` implementation returned by `new_handler_with_pipeline`.
pub struct NewHandlerWithPipeline<C, P, NH> {
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
    new_handler: NH,
}

/// `Handler` implementation instantiated by `NewHandlerWithPipeline`.
pub struct HandlerWithPipeline<C, P, H> {
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
    handler: H,
}

impl<C, P, NH> NewHandler for NewHandlerWithPipeline<C, P, NH>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + RefUnwindSafe + 'static,
    NH: NewHandler,
    NH::Instance: 'static,
{
    type Instance = HandlerWithPipeline<C, P, NH::Instance>;

    fn new_handler(&self) -> Result<Self::Instance> {
        Ok(HandlerWithPipeline {
            pipeline_chain: self.pipeline_chain,
            pipelines: self.pipelines.clone(),
            handler: self.new_handler.new_handler()?,
        })
    }
}

impl<C, P, H> Handler for HandlerWithPipeline<C, P, H>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + RefUnwindSafe + 'static,
    H: Handler + 'static,
{
    fn handle(self, state: State) -> Box<HandlerFuture> {
        let handler = self.handler;
        self.pipeline_chain
            .call(&self.pipelines, state, |state| handler.handle(state))
    }
}
