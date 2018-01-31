//! Defines types for a middleware pipeline

pub mod single;

use std::io;
use std::panic::RefUnwindSafe;

use handler::HandlerFuture;
use middleware::{Middleware, NewMiddleware};
use state::{request_id, State};

/// When using middleware, one or more `Middleware` are combined to form a `Pipeline`.
/// `Middleware` are invoked strictly in the order they're added to the `Pipeline`.
///
/// At `Request` dispatch time, the `Middleware` are created from the `NewMiddleware` values given
/// to the `PipelineBuilder`, and combined with a `Handler` created from the `NewHandler` provided
/// to `Pipeline::call`.  These `Middleware` and `Handler` values are used for a single `Request`.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate gotham_derive;
/// # extern crate hyper;
/// # extern crate mime;
/// #
/// # use std::io;
/// # use gotham::http::response::create_response;
/// # use gotham::state::State;
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::{Middleware, NewMiddleware};
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::router::Router;
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
/// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
/// # use gotham::test::TestServer;
/// # use gotham::extractor::{NoopPathExtractor, NoopQueryStringExtractor};
/// # use gotham::router::response::finalizer::ResponseFinalizerBuilder;
/// # use hyper::{Response, StatusCode, Method};
/// #
/// #[derive(StateData)]
/// struct MiddlewareData {
///     vec: Vec<i32>
/// }
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
/// #     fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
/// #         where Chain: FnOnce(State) -> Box<HandlerFuture> + 'static
/// #     {
/// #         state.put(MiddlewareData { vec: vec![1] });
/// #         chain(state)
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
/// #     fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
/// #         where Chain: FnOnce(State) -> Box<HandlerFuture> + 'static
/// #     {
/// #         state.borrow_mut::<MiddlewareData>().vec.push(2);
/// #         chain(state)
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
/// #     fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
/// #         where Chain: FnOnce(State) -> Box<HandlerFuture> + 'static
/// #     {
/// #         state.borrow_mut::<MiddlewareData>().vec.push(3);
/// #         chain(state)
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
/// fn handler(state: State) -> (State, Response) {
///     let body = {
///        let data = state.borrow::<MiddlewareData>();
///        format!("{:?}", data.vec)
///     };
///
///     let res = create_response(&state,
///                               StatusCode::Ok,
///                               Some((body.into_bytes(), mime::TEXT_PLAIN)));
///
///     (state, res)
/// }
///
/// fn main() {
///     let editable_pipeline_set = new_pipeline_set();
///     let (editable_pipeline_set, pipeline) = editable_pipeline_set.add(new_pipeline()
///         .add(MiddlewareOne)
///         .add(MiddlewareTwo)
///         .add(MiddlewareThree)
///         .build());
///
///     let pipeline_set = finalize_pipeline_set(editable_pipeline_set);
///
///     // Router / TestServer definitions elided
/// #   let mut tree_builder = TreeBuilder::new();
/// #
/// #   let matcher = MethodOnlyRouteMatcher::new(vec![Method::Get]);
/// #   let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (pipeline, ()), pipeline_set));
/// #   let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
/// #   let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
/// #   tree_builder.add_route(Box::new(route));
/// #   let tree = tree_builder.finalize();
/// #
/// #   let response_finalizer = ResponseFinalizerBuilder::new().finalize();
/// #   let router = Router::new(tree, response_finalizer);
/// #
/// #   let test_server = TestServer::new(router).unwrap();
///     let response = test_server.client().get("http://example.com/").perform().unwrap();
///     assert_eq!(response.status(), StatusCode::Ok);
///     assert_eq!(response.read_body().unwrap(), "[1, 2, 3]".as_bytes());
/// }
/// ```
pub struct Pipeline<T>
where
    T: NewMiddlewareChain,
{
    chain: T,
}

/// Represents an instance of a `Pipeline`. Returned from `Pipeline::construct()`.
pub struct PipelineInstance<T>
where
    T: MiddlewareChain,
{
    chain: T,
}

impl<T> Pipeline<T>
where
    T: NewMiddlewareChain,
{
    /// Constructs an instance of this `Pipeline` by creating all `Middleware` instances required
    /// to serve a request. If any middleware fails creation, its error will be returned.
    pub fn construct(&self) -> io::Result<PipelineInstance<T::Instance>> {
        Ok(PipelineInstance {
            chain: self.chain.construct()?,
        })
    }
}

impl<T> PipelineInstance<T>
where
    T: MiddlewareChain,
{
    /// Serves a request using this `PipelineInstance`. Requests that pass through all `Middleware`
    /// will be served with the `f` function.
    pub fn call<F>(self, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + 'static,
    {
        trace!("[{}] calling middleware", request_id(&state));
        self.chain.call(state, f)
    }
}

/// Begins defining a new pipeline.
///
/// See `PipelineBuilder` for information on using `new_pipeline()`.
pub fn new_pipeline() -> PipelineBuilder<()> {
    trace!(" starting pipeline construction");
    // See: `impl NewMiddlewareChain for ()`
    PipelineBuilder { t: () }
}

/// Allows a pipeline to be defined by adding `NewMiddleware` values, and building a `Pipeline`.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// #
/// # use std::io;
/// # use gotham::state::State;
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::{Middleware, NewMiddleware};
/// # use gotham::pipeline::new_pipeline;
/// #
/// # #[derive(Clone)]
/// # struct MiddlewareOne;
/// # #[derive(Clone)]
/// # struct MiddlewareTwo;
/// # #[derive(Clone)]
/// # struct MiddlewareThree;
/// #
/// # impl Middleware for MiddlewareOne {
/// #   fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State) -> Box<HandlerFuture> + 'static
/// #   {
/// #       chain(state)
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
/// #   fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State) -> Box<HandlerFuture> + 'static
/// #   {
/// #       chain(state)
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
/// #   fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State) -> Box<HandlerFuture> + 'static
/// #   {
/// #       chain(state)
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
/// # fn main() {
/// new_pipeline()
///     .add(MiddlewareOne)
///     .add(MiddlewareTwo)
///     .add(MiddlewareThree)
///     .build();
/// # }
/// ```
///
/// The pipeline defined here is invoked in this order:
///
/// `(&mut stateuest)` &rarr; `MiddlewareOne` &rarr; `MiddlewareTwo` &rarr; `MiddlewareThree`
/// &rarr; `handler` (provided later)
pub struct PipelineBuilder<T>
where
    T: NewMiddlewareChain,
{
    t: T,
}

impl<T> PipelineBuilder<T>
where
    T: NewMiddlewareChain,
{
    /// Builds a `Pipeline`, which contains all middleware in the order provided via `add` and is
    /// ready to process requests via a `NewHandler` provided to [`Pipeline::call`][Pipeline::call]
    ///
    /// [Pipeline::call]: struct.Pipeline.html#method.call
    pub fn build(self) -> Pipeline<T>
    where
        T: NewMiddlewareChain,
    {
        Pipeline { chain: self.t }
    }

    /// Adds a `NewMiddleware` which will create a `Middleware` during request dispatch.
    pub fn add<M>(self, m: M) -> PipelineBuilder<(M, T)>
    where
        M: NewMiddleware,
        M::Instance: 'static,
        Self: Sized,
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
        trace!(" adding middleware to pipeline");
        PipelineBuilder { t: (m, self.t) }
    }
}

/// A recursive type representing a pipeline, which is used to spawn a `MiddlewareChain`.
///
/// This type should never be implemented outside of Gotham, does not form part of the public API,
/// and is subject to change without notice.
#[doc(hidden)]
pub unsafe trait NewMiddlewareChain: RefUnwindSafe + Sized {
    type Instance: MiddlewareChain;

    /// Create and return a new `MiddlewareChain` value.
    fn construct(&self) -> io::Result<Self::Instance>;
}

unsafe impl<T, U> NewMiddlewareChain for (T, U)
where
    T: NewMiddleware,
    T::Instance: 'static,
    U: NewMiddlewareChain,
{
    type Instance = (T::Instance, U::Instance);

    fn construct(&self) -> io::Result<Self::Instance> {
        // This works as a recursive `map` over the "list" of `NewMiddleware`, and is used in
        // creating the `Middleware` instances for serving a single request.
        //
        // The reversed order is preserved in the return value.
        trace!(" adding middleware instance to pipeline");
        let (ref nm, ref tail) = *self;
        Ok((nm.new_middleware()?, tail.construct()?))
    }
}

unsafe impl NewMiddlewareChain for () {
    type Instance = ();

    fn construct(&self) -> io::Result<Self::Instance> {
        // () marks the end of the list, so is returned as-is.
        trace!(" completed middleware pipeline construction");
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
    fn call<F>(self, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + 'static;
}

unsafe impl MiddlewareChain for () {
    fn call<F>(self, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + 'static,
    {
        // At the last item in the `MiddlewareChain`, the function is invoked to serve the
        // request. `f` is the nested function of all `Middleware` and the `Handler`.
        //
        // In the case of 0 middleware, `f` is the function created in `MiddlewareChain::call`
        // which invokes the `Handler` directly.
        trace!("pipeline complete, invoking handler");
        f(state)
    }
}

unsafe impl<T, U> MiddlewareChain for (T, U)
where
    T: Middleware + 'static,
    U: MiddlewareChain,
{
    fn call<F>(self, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + 'static,
    {
        let (m, p) = self;
        // Construct the function from the inside, out. Starting with a function which calls the
        // `Handler`, and then creating a new function which calls the `Middleware` with the
        // previous function as the `chain` argument, we end up with a structure somewhat like
        // this (using `m0`, `m1`, `m2` as middleware names, where `m2` is the last middleware
        // before the `Handler`):
        //
        //  move |state| {
        //      m0.call(state, move |state| {
        //          m1.call(state, move |state| {
        //              m2.call(state, move |state| handler.call(state))
        //          })
        //      })
        //  }
        //
        // The resulting function is called by `<() as MiddlewareChain>::call`
        trace!("[{}] executing middleware", request_id(&state));
        p.call(state, move |state| m.call(state, f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::TestServer;
    use handler::{Handler, IntoHandlerError};
    use state::StateData;
    use hyper::Response;
    use hyper::StatusCode;
    use futures::future;

    fn handler(state: State) -> (State, Response) {
        let number = state.borrow::<Number>().value;
        (
            state,
            Response::new()
                .with_status(StatusCode::Ok)
                .with_body(format!("{}", number)),
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
    fn pipeline_ordering_test() {
        let test_server = TestServer::new(|| {
            let pipeline = new_pipeline()
                .add(Number { value: 0 }) // 0
                .add(Addition { value: 1 }) // 1
                .add(Multiplication { value: 2 }) // 2
                .add(Addition { value: 1 }) // 3
                .add(Multiplication { value: 2 }) // 6
                .add(Addition { value: 2 }) // 8
                .add(Multiplication { value: 3 }) // 24
                .build();

            Ok(move |state| match pipeline.construct() {
                Ok(p) => p.call(state, |state| handler.handle(state)),
                Err(e) => Box::new(future::err((state, e.into_handler_error()))),
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
