//! Defines types for a middleware pipeline

pub mod chain;
pub mod set;
pub mod single;

use std::io;

use handler::HandlerFuture;
use middleware::chain::{MiddlewareChain, NewMiddlewareChain};
use middleware::NewMiddleware;
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
/// # use gotham::helpers::http::response::create_response;
/// # use gotham::state::State;
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::pipeline::single::*;
/// # use gotham::router::builder::*;
/// # use gotham::test::TestServer;
/// # use hyper::{Body, Response, StatusCode};
/// #
/// #[derive(StateData)]
/// struct MiddlewareData {
///     vec: Vec<i32>
/// }
///
/// #[derive(NewMiddleware, Copy, Clone)]
/// struct MiddlewareOne;
///
/// impl Middleware for MiddlewareOne {
///     // Implementation elided.
///     // Appends `1` to `MiddlewareData.vec`
/// #     fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
/// #         where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
/// #     {
/// #         state.put(MiddlewareData { vec: vec![1] });
/// #         chain(state)
/// #     }
/// }
///
/// #[derive(NewMiddleware, Copy, Clone)]
/// struct MiddlewareTwo;
///
/// impl Middleware for MiddlewareTwo {
///     // Implementation elided.
///     // Appends `2` to `MiddlewareData.vec`
/// #     fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
/// #         where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
/// #     {
/// #         state.borrow_mut::<MiddlewareData>().vec.push(2);
/// #         chain(state)
/// #     }
/// }
///
/// #[derive(NewMiddleware, Copy, Clone)]
/// struct MiddlewareThree;
///
/// impl Middleware for MiddlewareThree {
///     // Implementation elided.
///     // Appends `3` to `MiddlewareData.vec`
/// #     fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
/// #         where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
/// #     {
/// #         state.borrow_mut::<MiddlewareData>().vec.push(3);
/// #         chain(state)
/// #     }
/// }
///
/// fn handler(state: State) -> (State, Response<Body>) {
///     let body = {
///        let data = state.borrow::<MiddlewareData>();
///        format!("{:?}", data.vec)
///     };
///
///     let res = create_response(&state,
///                               StatusCode::OK,
///                               mime::TEXT_PLAIN,
///                               body);
///
///     (state, res)
/// }
///
/// fn main() {
///     let (chain, pipelines) = single_pipeline(
///         new_pipeline()
///             .add(MiddlewareOne)
///             .add(MiddlewareTwo)
///             .add(MiddlewareThree)
///             .build()
///     );
///
///     let router = build_router(chain, pipelines, |route| {
///         route.get("/").to(handler);
///     });
///
///     let test_server = TestServer::new(router).unwrap();
///     let response = test_server.client().get("http://example.com/").perform().unwrap();
///     assert_eq!(response.status(), StatusCode::OK);
///     assert_eq!(response.read_utf8_body().unwrap(), "[1, 2, 3]");
/// }
/// ```
pub struct Pipeline<T>
where
    T: NewMiddlewareChain,
{
    chain: T,
}

/// Represents an instance of a `Pipeline`. Returned from `Pipeline::construct()`.
struct PipelineInstance<T>
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
    fn construct(&self) -> io::Result<PipelineInstance<T::Instance>> {
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
    fn call<F>(self, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
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

/// Constructs a pipeline from a single middleware.
pub fn single_middleware<M>(m: M) -> Pipeline<(M, ())>
where
    M: NewMiddleware,
    M::Instance: Send + 'static,
{
    new_pipeline().add(m).build()
}

/// Allows a pipeline to be defined by adding `NewMiddleware` values, and building a `Pipeline`.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate gotham_derive;
/// #
/// # use gotham::state::State;
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::pipeline::new_pipeline;
/// #
/// # #[derive(NewMiddleware, Copy, Clone)]
/// # struct MiddlewareOne;
/// #
/// # #[derive(NewMiddleware, Copy, Clone)]
/// # struct MiddlewareTwo;
/// #
/// # #[derive(NewMiddleware, Copy, Clone)]
/// # struct MiddlewareThree;
/// #
/// # impl Middleware for MiddlewareOne {
/// #   fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
/// #   {
/// #       chain(state)
/// #   }
/// # }
/// #
/// # impl Middleware for MiddlewareTwo {
/// #   fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
/// #   {
/// #       chain(state)
/// #   }
/// # }
/// #
/// # impl Middleware for MiddlewareThree {
/// #   fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
/// #   {
/// #       chain(state)
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
/// `(&mut state)` &rarr; `MiddlewareOne` &rarr; `MiddlewareTwo` &rarr; `MiddlewareThree` &rarr;
/// `handler` (provided later, when building the router)
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
    /// ready to process requests via a `NewHandler` provided to `Pipeline::call`.
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
        M::Instance: Send + 'static,
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

#[cfg(test)]
mod tests {
    use super::*;

    use futures::future;
    use hyper::{Body, Response, StatusCode};

    use handler::{Handler, IntoHandlerError};
    use middleware::Middleware;
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
