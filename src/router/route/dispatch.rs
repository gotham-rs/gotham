//! Defines Gotham's `Dispatcher` and supporting types.

use std::sync::Arc;
use std::panic::RefUnwindSafe;
use borrow_bag::{new_borrow_bag, BorrowBag, Handle, Lookup};
use futures::future;

use handler::{Handler, NewHandler, HandlerFuture, IntoHandlerError};
use middleware::pipeline::{NewMiddlewareChain, Pipeline};
use state::{State, request_id};

/// Represents the set of all `Pipeline` instances that are available for use with `Routes`.
pub type PipelineSet<P> = Arc<BorrowBag<P>>;

/// A set of `Pipeline` instances that may continue to grow
pub type EditablePipelineSet<P> = BorrowBag<P>;

/// Create an empty set of `Pipeline` instances.
///
/// See BorrowBag#add to insert new `Pipeline` instances.
pub fn new_pipeline_set() -> EditablePipelineSet<()> {
    new_borrow_bag()
}

/// Wraps the current set of `Pipeline` instances into a thread-safe reference counting pointer for
/// use with `DispatcherImpl` instances.
pub fn finalize_pipeline_set<P>(eps: EditablePipelineSet<P>) -> PipelineSet<P> {
    Arc::new(eps)
}

/// Used by `Router` to dispatch requests via `Pipeline`(s), through `Middleware`(s)
/// and finally into the configured `Handler`.
pub trait Dispatcher: RefUnwindSafe {
    /// Dispatches a request via pipelines and `Handler` represented by this `Dispatcher`.
    fn dispatch(&self, state: State) -> Box<HandlerFuture>;
}

/// Default implementation of the `Dispatcher` trait.
pub struct DispatcherImpl<H, C, P>
where
    H: NewHandler,
    C: PipelineHandleChain<P>,
    P: RefUnwindSafe,
{
    new_handler: H,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
}

impl<H, C, P> DispatcherImpl<H, C, P>
where
    H: NewHandler,
    H::Instance: 'static,
    C: PipelineHandleChain<P>,
    P: RefUnwindSafe,
{
    /// Creates a new `DispatcherImpl`.
    ///
    /// * `new_handler` - The `Handler` that will be called once the `pipeline_chain` is complete.
    /// * `pipeline_chain` - A chain of `Pipeline` instance handles that indicate which `Pipelines` will be invoked.
    /// * `pipelines` - All `Pipeline` instances, accessible by the handles provided in `pipeline_chain`.
    ///
    pub fn new(new_handler: H, pipeline_chain: C, pipelines: PipelineSet<P>) -> Self {
        DispatcherImpl {
            new_handler,
            pipeline_chain,
            pipelines,
        }
    }
}

impl<H, C, P> Dispatcher for DispatcherImpl<H, C, P>
where
    H: NewHandler,
    H::Instance: 'static,
    C: PipelineHandleChain<P>,
    P: RefUnwindSafe,
{
    fn dispatch(&self, state: State) -> Box<HandlerFuture> {
        match self.new_handler.new_handler() {
            Ok(h) => {
                trace!("[{}] cloning handler", request_id(&state));
                self.pipeline_chain.call(
                    &self.pipelines,
                    state,
                    move |state| h.handle(state),
                )
            }
            Err(e) => {
                trace!("[{}] error cloning handler", request_id(&state));
                Box::new(future::err((state, e.into_handler_error())))
            }
        }
    }
}

/// A heterogeneous list of `Handle<P, _>` values, where `P` is a pipeline type. The pipelines are
/// borrowed and invoked in order to serve a request.
///
/// Implemented using nested tuples, with `()` marking the end of the list. The list is in the
/// reverse order of what is described via the routing API.
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
        F: FnOnce(State) -> Box<HandlerFuture> + 'static;
}

/// Part of a `PipelineHandleChain` which references a `Pipeline` and continues with a tail element.
impl<'a, P, T, N, U> PipelineHandleChain<P> for (Handle<Pipeline<T>, N>, U)
where
    T: NewMiddlewareChain,
    T::Instance: 'static,
    U: PipelineHandleChain<P>,
    P: Lookup<Pipeline<T>, N>,
    N: RefUnwindSafe,
{
    fn call<F>(&self, pipelines: &PipelineSet<P>, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + 'static,
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
        F: FnOnce(State) -> Box<HandlerFuture> + 'static,
    {
        trace!("[{}] start pipeline", request_id(&state));
        f(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use test::TestServer;
    use handler::NewHandlerService;
    use middleware::{Middleware, NewMiddleware};
    use middleware::pipeline::new_pipeline;
    use state::StateData;
    use hyper::Response;
    use hyper::StatusCode;

    fn handler(state: State) -> (State, Response) {
        let number = state.borrow::<Number>().value;
        (
            state,
            Response::new().with_status(StatusCode::Ok).with_body(
                format!(
                    "{}",
                    number
                ),
            ),
        )
    }

    #[derive(Clone)]
    struct Number {
        value: i32,
    }

    impl NewMiddleware for Number {
        type Instance = Number;

        fn new_middleware(&self) -> io::Result<Number> {
            Ok(self.clone())
        }
    }

    impl Middleware for Number {
        fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
        where
            Chain: FnOnce(State) -> Box<HandlerFuture> + 'static,
            Self: Sized,
        {
            state.put(self.clone());
            chain(state)
        }
    }

    impl StateData for Number {}

    struct Addition {
        value: i32,
    }

    impl NewMiddleware for Addition {
        type Instance = Addition;

        fn new_middleware(&self) -> io::Result<Addition> {
            Ok(Addition { ..*self })
        }
    }

    impl Middleware for Addition {
        fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
        where
            Chain: FnOnce(State) -> Box<HandlerFuture> + 'static,
            Self: Sized,
        {
            state.borrow_mut::<Number>().value += self.value;
            chain(state)
        }
    }

    struct Multiplication {
        value: i32,
    }

    impl NewMiddleware for Multiplication {
        type Instance = Multiplication;

        fn new_middleware(&self) -> io::Result<Multiplication> {
            Ok(Multiplication { ..*self })
        }
    }

    impl Middleware for Multiplication {
        fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
        where
            Chain: FnOnce(State) -> Box<HandlerFuture> + 'static,
            Self: Sized,
        {
            state.borrow_mut::<Number>().value *= self.value;
            chain(state)
        }
    }

    #[test]
    fn pipeline_chain_ordering_test() {
        let new_service = NewHandlerService::new(|| {
            Ok(move |state| {
                let pipelines = new_pipeline_set();

                let (pipelines, p1) = pipelines.add(
                    new_pipeline()
                    .add(Number { value: 0 }) // 0
                    .add(Addition { value: 1 }) // 1
                    .add(Multiplication { value: 2 }) // 2
                    .build(),
                );

                let (pipelines, p2) = pipelines.add(
                    new_pipeline()
                    .add(Addition { value: 1 }) // 3
                    .add(Multiplication { value: 2 }) // 6
                    .build(),
                );

                let (pipelines, p3) = pipelines.add(
                    new_pipeline()
                    .add(Addition { value: 2 }) // 8
                    .add(Multiplication { value: 3 }) // 24
                    .build(),
                );

                let pipelines = Arc::new(pipelines);

                let new_handler = || Ok(handler);

                let pipeline_chain = (p3, (p2, (p1, ())));
                let dispatcher = DispatcherImpl::new(new_handler, pipeline_chain, pipelines);
                dispatcher.dispatch(state)
            })
        });

        let uri = "http://localhost/".parse().unwrap();

        let mut test_server = TestServer::new(new_service).unwrap();
        let response = test_server
            .client("127.0.0.1:0".parse().unwrap())
            .unwrap()
            .get(uri);
        let response = test_server.run_request(response).unwrap();

        let buf = test_server.read_body(response).unwrap();
        assert_eq!(buf.as_slice(), "24".as_bytes());
    }
}
