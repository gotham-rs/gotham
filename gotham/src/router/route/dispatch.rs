//! Defines the route `Dispatcher` and supporting types.

use futures::future;
use std::panic::RefUnwindSafe;

use handler::{Handler, HandlerFuture, IntoHandlerError, NewHandler};
use pipeline::chain::PipelineHandleChain;
use pipeline::set::PipelineSet;
use state::{request_id, State};

/// Used by `Router` to dispatch requests via pipelines and finally into the configured `Handler`.
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
    /// * `pipeline_chain` - A chain of `Pipeline` instance handles that indicate which `Pipelines`
    ///   will be invoked.
    /// * `pipelines` - All `Pipeline` instances, accessible by the handles provided in
    ///   `pipeline_chain`.
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
    H::Instance: Send + 'static,
    C: PipelineHandleChain<P>,
    P: RefUnwindSafe,
{
    fn dispatch(&self, state: State) -> Box<HandlerFuture> {
        match self.new_handler.new_handler() {
            Ok(h) => {
                trace!("[{}] cloning handler", request_id(&state));
                self.pipeline_chain
                    .call(&self.pipelines, state, move |state| h.handle(state))
            }
            Err(e) => {
                trace!("[{}] error cloning handler", request_id(&state));
                Box::new(future::err((state, e.compat().into_handler_error())))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::Arc;

    use hyper::{Body, Response, StatusCode};

    use middleware::{Middleware, NewMiddleware};
    use pipeline::new_pipeline;
    use pipeline::set::*;
    use state::StateData;
    use test::TestServer;

    fn handler(state: State) -> (State, Response<Body>) {
        let number = state.borrow::<Number>().value;
        (
            state,
            Response::builder()
                .status(StatusCode::OK)
                .body(format!("{}", number).into())
                .unwrap(),
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
            Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
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
            Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
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
            Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
            Self: Sized,
        {
            state.borrow_mut::<Number>().value *= self.value;
            chain(state)
        }
    }

    #[test]
    fn pipeline_chain_ordering_test() {
        let test_server = TestServer::new(|| {
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
        }).unwrap();

        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        let buf = response.read_body().unwrap();
        assert_eq!(buf.as_slice(), "24".as_bytes());
    }
}
