//! Defines types for a middleware pipeline

use std::io;
use middleware::{Middleware, NewMiddleware};
use handler::{NewHandler, Handler, HandlerFuture};
use state::State;
use hyper::server::Request;
use futures::{future, Future};

// TODO: Refactor this example when the `Router` API properly integrates with pipelines.
/// When using middleware, one or more [`Middleware`][Middleware] are combined with a
/// [`Handler`][Handler] to form a `Pipeline`. `Middleware` are invoked strictly in the order
/// they're added to the `Pipeline`.
///
/// [Middleware]: ../trait.Middleware.html
/// [Handler]: ../../handler/trait.Handler.html
///
/// The `PipelineBuilder` used to define a pipeline expects to receive values of type
/// `NewMiddleware` and a `NewHandler`, which are used to spawn a new set of `Middleware` and a
/// `Handler` for each request.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use std::io;
/// # use gotham::state::{State, StateData};
/// # use gotham::handler::{Handler, HandlerFuture, HandlerService, NewHandlerService};
/// # use gotham::middleware::{Middleware, NewMiddleware};
/// # use gotham::middleware::pipeline::{new_pipeline, Pipeline, PipelineBuilder};
/// # use gotham::router::Router;
/// # use gotham::test::TestServer;
/// # use hyper::server::{Request, Response};
/// # use hyper::StatusCode;
/// # use hyper::Method::*;
/// #
/// struct MiddlewareData {
///     vec: Vec<i32>
/// }
///
/// impl StateData for MiddlewareData {}
///
/// #[derive(Clone)]
/// struct MiddlewareOne;
/// #[derive(Clone)]
/// struct MiddlewareTwo;
/// #[derive(Clone)]
/// struct MiddlewareThree;
///
/// impl Middleware for MiddlewareOne {
///     fn call<Chain>(&self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
///     {
///         state.put(MiddlewareData { vec: vec![1] });
///         chain(state, req)
///     }
/// }
///
/// impl NewMiddleware for MiddlewareOne {
///     type Instance = MiddlewareOne;
///     fn new_middleware(&self) -> io::Result<MiddlewareOne> {
///         Ok(self.clone())
///     }
/// }
///
/// impl Middleware for MiddlewareTwo {
///     fn call<Chain>(&self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
///     {
///         state.borrow_mut::<MiddlewareData>().unwrap().vec.push(2);
///         chain(state, req)
///     }
/// }
///
/// impl NewMiddleware for MiddlewareTwo {
///     type Instance = MiddlewareTwo;
///     fn new_middleware(&self) -> io::Result<MiddlewareTwo> {
///         Ok(self.clone())
///     }
/// }
///
/// impl Middleware for MiddlewareThree {
///     fn call<Chain>(&self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
///     {
///         state.borrow_mut::<MiddlewareData>().unwrap().vec.push(3);
///         chain(state, req)
///     }
/// }
///
/// impl NewMiddleware for MiddlewareThree {
///     type Instance = MiddlewareThree;
///     fn new_middleware(&self) -> io::Result<MiddlewareThree> {
///         Ok(self.clone())
///     }
/// }
///
/// fn handler(mut state: State, req: Request) -> (State, Response) {
///     let body = {
///         let data = state.borrow::<MiddlewareData>().unwrap();
///         format!("{:?}", data.vec)
///     };
///
///     (state, Response::new().with_status(StatusCode::Ok).with_body(body))
/// }
///
/// fn main() {
///     let new_service = NewHandlerService::new(|| {
///         let router = Router::build(|routes| {
///             routes.direct(Get, "/").to(handler);
///         });
///
///         Ok(new_pipeline()
///             .add(MiddlewareOne)
///             .add(MiddlewareTwo)
///             .add(MiddlewareThree)
///             .build(router))
///     });
///
///     let mut test_server = TestServer::new(new_service).unwrap();
///     let client = test_server.client("127.0.0.1:10000".parse().unwrap()).unwrap();
///     let uri = "http://example.com/".parse().unwrap();
///     let response = test_server.run_request(client.get(uri)).unwrap();
///     assert_eq!(response.status(), StatusCode::Ok);
///     assert_eq!(test_server.read_body(response).unwrap(), "[1, 2, 3]".as_bytes());
/// }
/// ```
pub struct Pipeline<T, H>
    where T: NewPipelineInstance,
          H: NewHandler
{
    builder: PipelineBuilder<T>,
    new_handler: H,
}

impl<T, H> Handler for Pipeline<T, H>
    where T: NewPipelineInstance + Send + Sync,
          H: NewHandler,
          H::Instance: 'static
{
    fn handle(&self, state: State, req: Request) -> Box<HandlerFuture> {
        match self.new_handler.new_handler() {
            Ok(handler) => {
                match self.builder.t.new_pipeline_instance() {
                    Ok(p) => p.call(state, req, move |state, req| handler.handle(state, req)),
                    Err(e) => future::err((state, e.into())).boxed(),
                }
            }
            Err(e) => future::err((state, e.into())).boxed(),
        }
    }
}

/// Begins defining a new pipeline. The returned [`PipeEnd`][PipeEnd] implements the
/// [`PipelineBuilder`][PipelineBuilder] trait, which is used to define a pipeline using the
/// builder pattern.
///
/// See [`PipelineBuilder`][PipelineBuilder] for information on using `Pipeline::new()`
///
/// [PipelineBuilder]: trait.PipelineBuilder.html
/// [PipeEnd]: struct.PipeEnd.html
pub fn new_pipeline() -> PipelineBuilder<()> {
    PipelineBuilder { t: () }
}

///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use std::io;
/// # use gotham::state::State;
/// # use gotham::handler::{Handler, HandlerFuture};
/// # use gotham::middleware::{Middleware, NewMiddleware};
/// # use gotham::middleware::pipeline::{new_pipeline, Pipeline, PipelineBuilder};
/// # use hyper::server::{Request, Response};
/// # use hyper::StatusCode;
/// #
/// # #[derive(Clone)]
/// # struct MiddlewareOne;
/// # #[derive(Clone)]
/// # struct MiddlewareTwo;
/// # #[derive(Clone)]
/// # struct MiddlewareThree;
/// #
/// # impl Middleware for MiddlewareOne {
/// #   fn call<Chain>(&self, state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
/// #   {
/// #       chain(state, req)
/// #   }
/// # }
/// #
/// # impl NewMiddleware for MiddlewareOne {
/// #   type Instance = MiddlewareOne;
/// #   fn new_middleware(&self) -> io::Result<MiddlewareOne> {
/// #       Ok(self.clone())
/// #   }
/// # }
/// #
/// # impl Middleware for MiddlewareTwo {
/// #   fn call<Chain>(&self, state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
/// #   {
/// #       chain(state, req)
/// #   }
/// # }
/// #
/// # impl NewMiddleware for MiddlewareTwo {
/// #   type Instance = MiddlewareTwo;
/// #   fn new_middleware(&self) -> io::Result<MiddlewareTwo> {
/// #       Ok(self.clone())
/// #   }
/// # }
/// #
/// # impl Middleware for MiddlewareThree {
/// #   fn call<Chain>(&self, state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
/// #   {
/// #       chain(state, req)
/// #   }
/// # }
/// #
/// # impl NewMiddleware for MiddlewareThree {
/// #   type Instance = MiddlewareThree;
/// #   fn new_middleware(&self) -> io::Result<MiddlewareThree> {
/// #       Ok(self.clone())
/// #   }
/// # }
/// #
/// # fn handler(state: State, _: Request) -> (State, Response) {
/// #   (state, Response::new().with_status(StatusCode::Accepted))
/// # }
/// #
/// # fn main() {
/// let pipeline: Pipeline<_, _> = new_pipeline()
///     .add(MiddlewareOne)
///     .add(MiddlewareTwo)
///     .add(MiddlewareThree)
///     .build(|| Ok(handler));
/// # }
/// ```
///
/// The pipeline defined here is invoked in this order:
///
/// `(&mut state, request)` &rarr; `MiddlewareOne` &rarr; `MiddlewareTwo` &rarr; `MiddlewareThree`
/// &rarr; `handler`
pub struct PipelineBuilder<T>
    where T: NewPipelineInstance
{
    t: T,
}

impl<T> PipelineBuilder<T>
    where T: NewPipelineInstance
{
    /// Builds a `Pipeline`, which has all middleware in the order provided via
    /// `PipelineBuilder::add`, with the `Handler` set to receive requests that pass through the
    /// pipeline.
    pub fn build<H>(self, h: H) -> Pipeline<T, H>
        where T: NewPipelineInstance,
              H: NewHandler,
              Self: Sized + 'static
    {
        Pipeline {
            builder: self,
            new_handler: h,
        }
    }

    /// Adds a `NewMiddleware` which will have its `Middleware` added to the `Pipeline` returned
    /// from `PipelineBuilder::build`.
    pub fn add<M>(self, m: M) -> PipelineBuilder<(M, T)>
        where M: NewMiddleware,
              M::Instance: Send + 'static,
              Self: Sized
    {
        PipelineBuilder { t: (m, self.t) }
    }
}

/// A recursive type representing a pipeline, which is used to spawn a `PipelineInstance`.
///
/// This type should never be implemented outside of Gotham, does not form part of the public API,
/// and is subject to change without notice.
#[doc(hidden)]
pub unsafe trait NewPipelineInstance: Sized {
    type Instance: PipelineInstance;

    /// Create and return a new `PipelineInstance` value.
    fn new_pipeline_instance(&self) -> io::Result<Self::Instance>;
}

unsafe impl<T, U> NewPipelineInstance for (T, U)
    where T: NewMiddleware,
          T::Instance: Send + 'static,
          U: NewPipelineInstance
{
    type Instance = (T::Instance, U::Instance);

    fn new_pipeline_instance(&self) -> io::Result<Self::Instance> {
        let (ref nm, ref tail) = *self;
        Ok((nm.new_middleware()?, tail.new_pipeline_instance()?))
    }
}

unsafe impl NewPipelineInstance for () {
    type Instance = ();

    fn new_pipeline_instance(&self) -> io::Result<Self::Instance> {
        Ok(())
    }
}

/// A recursive type representing an instance of a pipeline, which is used to process a single
/// request.
///
/// This type should never be implemented outside of Gotham, does not form part of the public API,
/// and is subject to change without notice.
#[doc(hidden)]
pub unsafe trait PipelineInstance: Sized {
    /// Dispatches a request to the given `Handler` after processing all `Middleware` in the
    /// pipeline.
    fn call<H>(self, state: State, request: Request, handler: H) -> Box<HandlerFuture>
        where H: Handler + 'static
    {
        self.call_recurse(state, request, move |state, req| handler.handle(state, req))
    }

    /// Recursive function for processing middleware and chaining to the given function.
    fn call_recurse<F>(self, state: State, request: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static;
}

unsafe impl PipelineInstance for () {
    fn call_recurse<F>(self, state: State, request: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
    {
        f(state, request)
    }
}

unsafe impl<T, U> PipelineInstance for (T, U)
    where T: Middleware + Send + 'static,
          U: PipelineInstance
{
    fn call_recurse<F>(self, state: State, request: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
    {
        let (m, p) = self;
        p.call_recurse(state, request, move |state, req| m.call(state, req, f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::TestServer;
    use handler::NewHandlerService;
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
        fn call<Chain>(&self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
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
        fn call<Chain>(&self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
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
        fn call<Chain>(&self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static,
                  Self: Sized
        {
            state.borrow_mut::<Number>().unwrap().value *= self.value;
            chain(state, req)
        }
    }

    #[test]
    fn pipeline_ordering_test() {
        let new_service = NewHandlerService::new(|| {
            Ok(new_pipeline()
                .add(Number { value: 0 }) // 0
                .add(Addition { value: 1 }) // 1
                .add(Multiplication { value: 2 }) // 2
                .add(Addition { value: 1 }) // 3
                .add(Multiplication { value: 2 }) // 6
                .add(Addition { value: 2 }) // 8
                .add(Multiplication { value: 3 }) // 24
                .build(|| Ok(handler)))
        });

        let uri = "http://localhost/".parse().unwrap();

        let mut test_server = TestServer::new(new_service).unwrap();
        let response = test_server.client("127.0.0.1:0".parse().unwrap()).unwrap().get(uri);
        let response = test_server.run_request(response).unwrap();

        let buf = test_server.read_body(response).unwrap();
        assert_eq!(buf.as_slice(), "24".as_bytes());
    }
}
