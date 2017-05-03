//! Defines Gotham's `Dispatcher` and supporting types.
//!
//! These types are intended to be used internally by Gotham's `Router` and supporting code. Gotham
//! applications should not need to consume these types directly.

use handler::{Handler, NewHandler, HandlerFuture};
use middleware::pipeline::{NewMiddlewareChain, Pipeline};
use state::State;

use hyper::server::Request;
use futures::{future, Future};

/// Internal type used by `Router` to dispatch requests via the configured `Pipeline`(s) and to the
/// correct `Handler`.
pub struct Dispatcher<H, C>
    where H: NewHandler,
          C: PipelineChain + Send + Sync
{
    /// The `NewHandler` which will create `Handler` values used for serving requests via this
    /// `Dispatcher`
    pub new_handler: H,

    /// The chain of `Pipeline`(s) used when dispatching via this `Dispatcher`
    pub pipeline_chain: C,
}

// Implementing `Handler`, rather than just `impl Dispatcher` ensures we continue meeting the same
// guarantees over time if the trait is refactored. `Dispatcher` isn't anticipated to ever act as
// a normal `Handler`.
impl<H, C> Handler for Dispatcher<H, C>
    where H: NewHandler,
          H::Instance: 'static,
          C: PipelineChain + Send + Sync
{
    fn handle(&self, state: State, req: Request) -> Box<HandlerFuture> {
        match self.new_handler.new_handler() {
            Ok(h) => self.pipeline_chain.call(state, req, move |state, req| h.handle(state, req)),
            Err(e) => future::err((state, e.into())).boxed(),
        }
    }
}

/// A heterogeneous list of `Pipeline` values, which are invoked in order to serve a request.
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
pub trait PipelineChain {
    /// Invokes this part of the `PipelineChain`, with requests being passed through to `f` once
    /// all `Middleware` in the `Pipeline` have passed the request through.
    fn call<F>(&self, state: State, req: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static;
}

/// Part of a `PipelineChain` which references a `Pipeline` and continues with a tail element.
impl<'a, T, U> PipelineChain for (&'a Pipeline<T>, U)
    where T: NewMiddlewareChain,
          T::Instance: Send + 'static,
          U: PipelineChain
{
    fn call<F>(&self, state: State, req: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
    {
        let (ref pipeline, ref chain) = *self;
        match pipeline.construct() {
            Ok(p) => chain.call(state, req, move |state, req| p.call(state, req, f)),
            Err(e) => future::err((state, e.into())).boxed(),
        }
    }
}

/// The marker for the end of a `PipelineChain`.
impl PipelineChain for () {
    fn call<F>(&self, state: State, req: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
    {
        f(state, req)
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
    use hyper::server::Response;
    use hyper::StatusCode;

    fn handler(state: State, _req: Request) -> (State, Response) {
        let number = state.borrow::<Number>().unwrap().value;
        (state, Response::new().with_status(StatusCode::Ok).with_body(format!("{}", number)))
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
        fn call<Chain>(self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static,
                  Self: Sized
        {
            state.put(self.clone());
            chain(state, req)
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
        fn call<Chain>(self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static,
                  Self: Sized
        {
            state.borrow_mut::<Number>().unwrap().value += self.value;
            chain(state, req)
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
        fn call<Chain>(self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static,
                  Self: Sized
        {
            state.borrow_mut::<Number>().unwrap().value *= self.value;
            chain(state, req)
        }
    }

    #[test]
    fn pipeline_chain_ordering_test() {
        let new_service = NewHandlerService::new(|| {
            Ok(move |state, req| {
                let p1 = new_pipeline()
                    .add(Number { value: 0 }) // 0
                    .add(Addition { value: 1 }) // 1
                    .add(Multiplication { value: 2 }) // 2
                    .build();

                let p2 = new_pipeline()
                    .add(Addition { value: 1 }) // 3
                    .add(Multiplication { value: 2 }) // 6
                    .build();

                let p3 = new_pipeline()
                    .add(Addition { value: 2 }) // 8
                    .add(Multiplication { value: 3 }) // 24
                    .build();

                let new_handler = || Ok(handler);

                let pipeline_chain = (&p3, (&p2, (&p1, ())));
                let dispatcher = Dispatcher {
                    new_handler,
                    pipeline_chain,
                };
                dispatcher.handle(state, req)
            })
        });

        let uri = "http://localhost/".parse().unwrap();

        let mut test_server = TestServer::new(new_service).unwrap();
        let response = test_server.client("127.0.0.1:0".parse().unwrap()).unwrap().get(uri);
        let response = test_server.run_request(response).unwrap();

        let buf = test_server.read_body(response).unwrap();
        assert_eq!(buf.as_slice(), "24".as_bytes());
    }
}
