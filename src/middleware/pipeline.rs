//! Defines types for a middleware pipeline

use std::io;
use middleware::{Middleware, NewMiddleware};
use handler::HandlerFuture;
use state::State;
use hyper::server::Request;

// TODO: Refactor this example when the `Router` API properly integrates with pipelines.
/// When using middleware, one or more [`Middleware`][Middleware] are combined to form a
/// `Pipeline`. `Middleware` are invoked strictly in the order they're added to the `Pipeline`.
///
/// At request dispatch time, the `Middleware` are created from the
/// [`NewMiddleware`][NewMiddleware] values given to the `PipelineBuilder`, and combined with a
/// [`Handler`][Handler] created from the [`NewHandler`][NewHandler] provided to `Pipeline::call`.
/// These `Middleware` and `Handler` values are used for a single request.
///
/// [Middleware]: ../trait.Middleware.html
/// [NewMiddleware]: ../trait.NewMiddleware.html
/// [Handler]: ../../handler/trait.Handler.html
/// [NewHandler]: ../../handler/trait.NewHandler.html
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
/// # use gotham::handler::{HandlerFuture, NewHandlerService};
/// # use gotham::middleware::{Middleware, NewMiddleware};
/// // # use gotham::middleware::pipeline::{new_pipeline, Pipeline, PipelineBuilder};
/// # use gotham::router::Router;
/// # use gotham::router::tree::Tree;
/// # use gotham::router::route::{Route, RouteImpl};
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::dispatch::Dispatcher;
/// # use gotham::test::TestServer;
/// # use hyper::server::{Request, Response};
/// # use hyper::StatusCode;
/// # use hyper::Method;
/// // # use futures::{future, Future};
/// #
/// struct MiddlewareData {
///     vec: Vec<i32>
/// }
///
/// impl StateData for MiddlewareData {}
///
/// # #[derive(Clone)]
/// struct MiddlewareOne;
/// # #[derive(Clone)]
/// struct MiddlewareTwo;
/// # #[derive(Clone)]
/// struct MiddlewareThree;
///
/// impl Middleware for MiddlewareOne {
///     // Implementation elided.
///     // Appends `1` to `MiddlewareData.vec`
/// #     fn call<Chain>(self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
/// #         where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
/// #     {
/// #         state.put(MiddlewareData { vec: vec![1] });
/// #         chain(state, req)
/// #     }
/// }
/// #
/// # impl NewMiddleware for MiddlewareOne {
/// #     type Instance = MiddlewareOne;
/// #     fn new_middleware(&self) -> io::Result<MiddlewareOne> {
/// #         Ok(self.clone())
/// #     }
/// # }
///
/// impl Middleware for MiddlewareTwo {
///     // Implementation elided.
///     // Appends `2` to `MiddlewareData.vec`
/// #     fn call<Chain>(self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
/// #         where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
/// #     {
/// #         state.borrow_mut::<MiddlewareData>().unwrap().vec.push(2);
/// #         chain(state, req)
/// #     }
/// }
/// #
/// # impl NewMiddleware for MiddlewareTwo {
/// #     type Instance = MiddlewareTwo;
/// #     fn new_middleware(&self) -> io::Result<MiddlewareTwo> {
/// #         Ok(self.clone())
/// #     }
/// # }
///
/// impl Middleware for MiddlewareThree {
///     // Implementation elided.
///     // Appends `3` to `MiddlewareData.vec`
/// #     fn call<Chain>(self, mut state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
/// #         where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
/// #     {
/// #         state.borrow_mut::<MiddlewareData>().unwrap().vec.push(3);
/// #         chain(state, req)
/// #     }
/// }
/// #
/// # impl NewMiddleware for MiddlewareThree {
/// #     type Instance = MiddlewareThree;
/// #     fn new_middleware(&self) -> io::Result<MiddlewareThree> {
/// #         Ok(self.clone())
/// #     }
/// # }
///
/// fn handler(state: State, _req: Request) -> (State, Response) {
///     // TODO: Dump the contents of the `Vec<i32>` into the response body.
///     // let body = {
///     //    let data = state.borrow::<MiddlewareData>().unwrap();
///     //    format!("{:?}", data.vec)
///     // };
///
///     // (state, Response::new().with_status(StatusCode::Ok).with_body(body))
///     (state, Response::new())
/// }
///
/// fn basic_route() -> Box<Route + Send + Sync>
/// {
///     let matcher = MethodOnlyRequestMatcher::new(vec![Method::Get]);
///     let dispatcher = Dispatcher::new(|| Ok(handler), ());
///     Box::new(RouteImpl::new(matcher, dispatcher))
/// }
///
/// fn not_found(state: State, _req: Request) -> (State, Response) {
///     (state, Response::new().with_status(StatusCode::NotFound))
/// }
///
/// fn internal_server_error(state: State, _req: Request) -> (State, Response) {
///     (state, Response::new().with_status(StatusCode::InternalServerError))
/// }
///
/// fn main() {
///     // TODO: Pipeline issues currently outstanding
///     // let pipeline = new_pipeline()
///     //    .add(MiddlewareOne)
///     //    .add(MiddlewareTwo)
///     //    .add(MiddlewareThree)
///     //    .build();
///
///     let mut tree = Tree::new();
///     tree.add_route(basic_route());
///
///     let router = Router::new(tree, || Ok(not_found), || Ok(internal_server_error));
///
///     let new_service = NewHandlerService::new(router);
///     let mut test_server = TestServer::new(new_service).unwrap();
///     let client = test_server.client("127.0.0.1:10000".parse().unwrap()).unwrap();
///     let uri = "http://example.com/".parse().unwrap();
///     let response = test_server.run_request(client.get(uri)).unwrap();
///     assert_eq!(response.status(), StatusCode::Ok);
///
///     // TODO: Pipeline issues currently outstanding
///     // assert_eq!(test_server.read_body(response).unwrap(), "[1, 2, 3]".as_bytes());
/// }
/// ```
pub struct Pipeline<T>
    where T: NewMiddlewareChain
{
    chain: T,
}

/// Represents an instance of a `Pipeline`. Returned from
/// [`Pipeline::construct`][Pipeline::construct]
///
/// [Pipeline::construct]: struct.Pipeline.html#method.construct
pub struct PipelineInstance<T>
    where T: MiddlewareChain
{
    chain: T,
}

impl<T> Pipeline<T>
    where T: NewMiddlewareChain
{
    /// Constructs an instance of this `Pipeline` by creating all `Middleware` instances required
    /// to serve a request. If any middleware fails creation, its error will be returned.
    pub fn construct(&self) -> io::Result<PipelineInstance<T::Instance>> {
        Ok(PipelineInstance { chain: self.chain.construct()? })
    }
}

impl<T> PipelineInstance<T>
    where T: MiddlewareChain
{
    /// Serves a request using this `PipelineInstance`. Requests that pass through all `Middleware`
    /// will be served with the `f` function.
    pub fn call<F>(self, state: State, req: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
    {
        self.chain.call(state, req, f)
    }
}

/// Begins defining a new pipeline.
///
/// See [`PipelineBuilder`][PipelineBuilder] for information on using `new_pipeline()`
///
/// [PipelineBuilder]: struct.PipelineBuilder.html
pub fn new_pipeline() -> PipelineBuilder<()> {
    // See: `impl NewMiddlewareChain for ()`
    PipelineBuilder { t: () }
}

/// Allows a pipeline to be defined by adding `NewMiddleware` values, and building a `Pipeline`.
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
/// #   fn call<Chain>(self, state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
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
/// #   fn call<Chain>(self, state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
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
/// #   fn call<Chain>(self, state: State, req: Request, chain: Chain) -> Box<HandlerFuture>
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
/// let pipeline: Pipeline<_> = new_pipeline()
///     .add(MiddlewareOne)
///     .add(MiddlewareTwo)
///     .add(MiddlewareThree)
///     .build();
/// # }
/// ```
///
/// The pipeline defined here is invoked in this order:
///
/// `(&mut state, request)` &rarr; `MiddlewareOne` &rarr; `MiddlewareTwo` &rarr; `MiddlewareThree`
/// &rarr; `handler` (provided later)
pub struct PipelineBuilder<T>
    where T: NewMiddlewareChain
{
    t: T,
}

impl<T> PipelineBuilder<T>
    where T: NewMiddlewareChain
{
    /// Builds a `Pipeline`, which contains all middleware in the order provided via `add` and is
    /// ready to process requests via a `NewHandler` provided to [`Pipeline::call`][Pipeline::call]
    ///
    /// [Pipeline::call]: struct.Pipeline.html#method.call
    pub fn build(self) -> Pipeline<T>
        where T: NewMiddlewareChain
    {
        Pipeline { chain: self.t }
    }

    /// Adds a `NewMiddleware` which will create a `Middleware` during request dispatch.
    pub fn add<M>(self, m: M) -> PipelineBuilder<(M, T)>
        where M: NewMiddleware,
              M::Instance: Send + 'static,
              Self: Sized
    {
        // "cons" the most recently added `NewMiddleware` onto the front of the list. This is
        // essentially building an HList-style tuple in reverse order. So for a call like:
        //
        //     new_pipeline().add(MiddlewareOne).add(MiddlewareTwo).add(MiddlewareThree)
        //
        // The resulting `PipelineBuilder` will be:
        //
        //     PipelineBuilder { t: (MiddlewareThree, (MiddlewareTwo, (MiddlewareOne, ()))) }
        //
        // An empty `PipelineBuilder` is represented as:
        //
        //     PipelineBuilder { t: () }
        PipelineBuilder { t: (m, self.t) }
    }
}

/// A recursive type representing a pipeline, which is used to spawn a `MiddlewareChain`.
///
/// This type should never be implemented outside of Gotham, does not form part of the public API,
/// and is subject to change without notice.
#[doc(hidden)]
pub unsafe trait NewMiddlewareChain: Sized {
    type Instance: MiddlewareChain;

    /// Create and return a new `MiddlewareChain` value.
    fn construct(&self) -> io::Result<Self::Instance>;
}

unsafe impl<T, U> NewMiddlewareChain for (T, U)
    where T: NewMiddleware,
          T::Instance: Send + 'static,
          U: NewMiddlewareChain
{
    type Instance = (T::Instance, U::Instance);

    fn construct(&self) -> io::Result<Self::Instance> {
        // This works as a recursive `map` over the "list" of `NewMiddleware`, and is used in
        // creating the `Middleware` instances for serving a single request.
        //
        // The reversed order is preserved in the return value.
        let (ref nm, ref tail) = *self;
        Ok((nm.new_middleware()?, tail.construct()?))
    }
}

unsafe impl NewMiddlewareChain for () {
    type Instance = ();

    fn construct(&self) -> io::Result<Self::Instance> {
        // () marks the end of the list, so is returned as-is.
        Ok(())
    }
}

/// A recursive type representing an instance of a pipeline, which is used to process a single
/// request.
///
/// This type should never be implemented outside of Gotham, does not form part of the public API,
/// and is subject to change without notice.
#[doc(hidden)]
pub unsafe trait MiddlewareChain: Sized {
    // TODO: Update this after implementing the `dispatch` module.
    /// Recursive function for processing middleware and chaining to the given function.
    fn call<F>(self, state: State, request: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static;
}

unsafe impl MiddlewareChain for () {
    fn call<F>(self, state: State, request: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
    {
        // At the last item in the `MiddlewareChain`, the function is invoked to serve the
        // request. `f` is the nested function of all `Middleware` and the `Handler`.
        //
        // In the case of 0 middleware, `f` is the function created in `MiddlewareChain::call`
        // which invokes the `Handler` directly.
        f(state, request)
    }
}

unsafe impl<T, U> MiddlewareChain for (T, U)
    where T: Middleware + Send + 'static,
          U: MiddlewareChain
{
    fn call<F>(self, state: State, request: Request, f: F) -> Box<HandlerFuture>
        where F: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static
    {
        let (m, p) = self;
        // Construct the function from the inside, out. Starting with a function which calls the
        // `Handler`, and then creating a new function which calls the `Middleware` with the
        // previous function as the `chain` argument, we end up with a structure somewhat like
        // this (using `m0`, `m1`, `m2` as middleware names, where `m2` is the last middleware
        // before the `Handler`):
        //
        //  move |state, req| {
        //      m0.call(state, req, move |state, req| {
        //          m1.call(state, req, move |state, req| {
        //              m2.call(state, req, move |state, req| handler.call(state, req))
        //          })
        //      })
        //  }
        //
        // The resulting function is called by `<() as MiddlewareChain>::call`
        p.call(state, request, move |state, req| m.call(state, req, f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::TestServer;
    use handler::{Handler, NewHandlerService};
    use state::StateData;
    use hyper::server::Response;
    use hyper::StatusCode;
    use futures::{future, Future};

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
    fn pipeline_ordering_test() {
        let new_service = NewHandlerService::new(|| {
            let pipeline = new_pipeline()
                .add(Number { value: 0 }) // 0
                .add(Addition { value: 1 }) // 1
                .add(Multiplication { value: 2 }) // 2
                .add(Addition { value: 1 }) // 3
                .add(Multiplication { value: 2 }) // 6
                .add(Addition { value: 2 }) // 8
                .add(Multiplication { value: 3 }) // 24
                .build();

            Ok(move |state, req| match pipeline.construct() {
                   Ok(p) => p.call(state, req, |state, req| handler.handle(state, req)),
                   Err(e) => future::err((state, e.into())).boxed(),
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
