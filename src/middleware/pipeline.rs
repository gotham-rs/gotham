//! Defines types for a middleware pipeline

use middleware::Middleware;
use handler::{Handler, HandlerFuture};
use state::State;
use hyper::server::Request;

// TODO: Refactor this example when the `Router` API properly integrates with pipelines.
/// When using middleware, one or more [`Middleware`][Middleware] are combined with a
/// [`Handler`][Handler] to form a `Pipeline`. `Middleware` are invoked strictly in the order
/// they're added to the `Pipeline`.
///
/// [Middleware]: ../trait.Middleware.html
/// [Handler]: ../../handler/trait.Handler.html
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use gotham::state::{State, StateData};
/// # use gotham::handler::{Handler, HandlerFuture, HandlerService};
/// # use gotham::middleware::Middleware;
/// # use gotham::middleware::pipeline::{Pipeline, PipelineBuilder};
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
/// struct MiddlewareOne;
/// struct MiddlewareTwo;
/// struct MiddlewareThree;
///
/// impl Middleware for MiddlewareOne {
///     fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
///     {
///         state.put(MiddlewareData { vec: vec![1] });
///         chain(state, req)
///     }
/// }
///
/// impl Middleware for MiddlewareTwo {
///     fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
///     {
///         state.borrow_mut::<MiddlewareData>().unwrap().vec.push(2);
///         chain(state, req)
///     }
/// }
///
/// impl Middleware for MiddlewareThree {
///     fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
///     {
///         state.borrow_mut::<MiddlewareData>().unwrap().vec.push(3);
///         chain(state, req)
///     }
/// }
///
/// fn handler(state: &mut State, req: Request) -> Response {
///     let data = state.borrow::<MiddlewareData>().unwrap();
///     let body = format!("{:?}", data.vec);
///
///     Response::new()
///         .with_status(StatusCode::Ok)
///         .with_body(body)
/// }
///
/// fn router() -> Pipeline {
///     let router = Router::build(|routes| {
///         routes.direct(Get, "/").to(handler);
///     });
///
///     Pipeline::new()
///         .add(MiddlewareOne)
///         .add(MiddlewareTwo)
///         .add(MiddlewareThree)
///         .build(router)
/// }
///
/// fn main() {
///     let mut test_server = TestServer::new(|| Ok(HandlerService::new(router()))).unwrap();
///     let client = test_server.client("127.0.0.1:10000".parse().unwrap()).unwrap();
///     let uri = "http://example.com/".parse().unwrap();
///     let response = test_server.run_request(client.get(uri)).unwrap();
///     assert_eq!(response.status(), StatusCode::Ok);
///     assert_eq!(test_server.read_body(response).unwrap(), "[1, 2, 3]".as_bytes());
/// }
/// ```
pub struct Pipeline {
    f: Box<Fn(&mut State, Request) -> Box<HandlerFuture> + Send + Sync>,
}

impl Handler for Pipeline {
    fn handle(&self, state: &mut State, req: Request) -> Box<HandlerFuture> {
        (self.f)(state, req)
    }
}

impl Pipeline {
    /// Begins defining a new pipeline. The returned [`PipeEnd`][PipeEnd] implements the
    /// [`PipelineBuilder`][PipelineBuilder] trait, which is used to define a pipeline using the
    /// builder pattern.
    ///
    /// See [`PipelineBuilder`][PipelineBuilder] for information on using `Pipeline::new()`
    ///
    /// [PipelineBuilder]: trait.PipelineBuilder.html
    /// [PipeEnd]: struct.PipeEnd.html
    pub fn new() -> PipeEnd {
        PipeEnd { _nothing: () }
    }
}

///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use gotham::state::State;
/// # use gotham::handler::{Handler, HandlerFuture};
/// # use gotham::middleware::Middleware;
/// # use gotham::middleware::pipeline::{Pipeline, PipelineBuilder};
/// # use hyper::server::{Request, Response};
/// # use hyper::StatusCode;
/// #
/// # struct MiddlewareOne;
/// # struct MiddlewareTwo;
/// # struct MiddlewareThree;
/// #
/// # impl Middleware for MiddlewareOne {
/// #   fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
/// #   {
/// #       chain(state, req)
/// #   }
/// # }
/// #
/// # impl Middleware for MiddlewareTwo {
/// #   fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
/// #   {
/// #       chain(state, req)
/// #   }
/// # }
/// #
/// # impl Middleware for MiddlewareThree {
/// #   fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
/// #   {
/// #       chain(state, req)
/// #   }
/// # }
/// #
/// # fn handler(_: &mut State, _: Request) -> Response {
/// #   Response::new().with_status(StatusCode::Accepted)
/// # }
/// #
/// # fn main() {
/// let pipeline: Pipeline = Pipeline::new()
///     .add(MiddlewareOne)
///     .add(MiddlewareTwo)
///     .add(MiddlewareThree)
///     .build(handler);
/// # }
/// ```
///
/// The pipeline defined here is invoked in this order:
///
/// `(&mut state, request)` &rarr; `MiddlewareOne` &rarr; `MiddlewareTwo` &rarr; `MiddlewareThree`
/// &rarr; `handler`
pub unsafe trait PipelineBuilder: Sized {
    /// Builds a `Pipeline`, which has all middleware in the order provided via
    /// `PipelineBuilder::add`, with the `Handler` set to receive requests that pass through the
    /// pipeline.
    fn build<H>(self, handler: H) -> Pipeline
        where H: Handler + 'static
    {
        self.build_recurse(move |state: &mut State, req: Request| handler.handle(state, req))
    }

    /// Adds a `Middleware` which will be in the `Pipeline` returned from `PipelineBuilder::build`.
    fn add<M>(self, m: M) -> PipeSegment<M, Self>
        where M: Middleware + Send + Sync
    {
        PipeSegment {
            middleware: m,
            tail: self,
        }
    }

    /// Internal function for recursively building a `Pipeline`.
    #[doc(hidden)]
    fn build_recurse<F>(self, f: F) -> Pipeline
        where F: Fn(&mut State, Request) -> Box<HandlerFuture> + Send + Sync + 'static;
}

/// A segment of a [`PipelineBuilder`][PipelineBuilder] which represents a
/// [`Middleware`][Middleware] that has been added to an existing
/// [`PipelineBuilder`][PipelineBuilder].
///
/// [Middleware]: ../trait.Middleware.html
/// [PipelineBuilder]: trait.PipelineBuilder.html
pub struct PipeSegment<M, Tail>
    where M: Middleware + Send + Sync,
          Tail: PipelineBuilder
{
    middleware: M,
    tail: Tail,
}

/// An empty [`PipelineBuilder`][PipelineBuilder].
///
/// [PipelineBuilder]: trait.PipelineBuilder.html
pub struct PipeEnd {
    _nothing: (),
}

unsafe impl<M, Tail> PipelineBuilder for PipeSegment<M, Tail>
    where M: Middleware + Send + Sync + 'static,
          Tail: PipelineBuilder
{
    fn build_recurse<F>(self, f: F) -> Pipeline
        where F: Fn(&mut State, Request) -> Box<HandlerFuture> + Send + Sync + 'static
    {
        let middleware = self.middleware;
        self.tail.build_recurse(move |state: &mut State, req: Request| {
                                    middleware.call(state, req, &f)
                                })
    }
}

unsafe impl PipelineBuilder for PipeEnd {
    fn build_recurse<F>(self, f: F) -> Pipeline
        where F: Fn(&mut State, Request) -> Box<HandlerFuture> + Send + Sync + 'static
    {
        Pipeline { f: Box::new(f) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::TestServer;
    use handler::HandlerService;
    use state::StateData;
    use hyper::server::Response;
    use hyper::StatusCode;

    fn handler(state: &mut State, _req: Request) -> Response {
        let number = state.borrow::<Number>().unwrap();
        Response::new().with_status(StatusCode::Ok).with_body(format!("{}", number.value))
    }

    #[derive(Clone)]
    struct Number {
        value: i32,
    }

    impl Middleware for Number {
        fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>,
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

    impl Middleware for Addition {
        fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>,
                  Self: Sized
        {
            state.borrow_mut::<Number>().unwrap().value += self.value;
            chain(state, req)
        }
    }

    struct Multiplication {
        value: i32,
    }

    impl Middleware for Multiplication {
        fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>,
                  Self: Sized
        {
            state.borrow_mut::<Number>().unwrap().value *= self.value;
            chain(state, req)
        }
    }

    #[test]
    fn pipeline_ordering_test() {
        let new_service = || {
            let pipeline = Pipeline::new()
                .add(Number { value: 0 }) // 0
                .add(Addition { value: 1 }) // 1
                .add(Multiplication { value: 2 }) // 2
                .add(Addition { value: 1 }) // 3
                .add(Multiplication { value: 2 }) // 6
                .add(Addition { value: 2 }) // 8
                .add(Multiplication { value: 3 }) // 24
                .build(handler);
            Ok(HandlerService::new(pipeline))
        };

        let uri = "http://localhost/".parse().unwrap();

        let mut test_server = TestServer::new(new_service).unwrap();
        let response = test_server.client("127.0.0.1:0".parse().unwrap()).unwrap().get(uri);
        let response = test_server.run_request(response).unwrap();

        let buf = test_server.read_body(response).unwrap();
        assert_eq!(buf.as_slice(), "24".as_bytes());
    }
}
