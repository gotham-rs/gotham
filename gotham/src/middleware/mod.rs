//! Defines types for `Middleware`, a reusable unit of logic that can apply to a group of requests
//! by being added to the `Pipeline` in a `Router`.

use std::io;
use std::panic::RefUnwindSafe;

use handler::HandlerFuture;
use state::State;

pub mod chain;
pub mod logger;
pub mod security;
pub mod session;
pub mod state;
pub mod timer;

/// `Middleware` has the opportunity to provide additional behaviour to the `Request` / `Response`
/// interaction. For example:
///
/// * The request can be halted due to some unmet precondition;
/// * Processing the request can be delayed until some other action has completed;
/// * Middleware-specific state data can be recorded in the `State` struct for use elsewhere;
/// * The returned future can be manipulated via continuations to provide additional behaviour
///   after the request completes.
///
/// # Examples
///
/// Taking no action, and immediately passing the `Request` through to the rest of the application:
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate gotham_derive;
/// # extern crate hyper;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::pipeline::*;
/// # use gotham::pipeline::single::*;
/// # use gotham::router::builder::*;
/// # use gotham::state::State;
/// # use gotham::test::TestServer;
/// #
/// #[derive(NewMiddleware, Copy, Clone)]
/// struct NoopMiddleware;
///
/// impl Middleware for NoopMiddleware {
///     fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
///     {
///         chain(state)
///     }
/// }
/// #
/// # fn main() {
/// #   let (chain, pipelines) = single_pipeline(
/// #       new_pipeline()
/// #           .add(NoopMiddleware)
/// #           .build()
/// #   );
/// #
/// #   let router = build_router(chain, pipelines, |route| {
/// #       route
/// #           .get("/")
/// #           .to_new_handler(|| {
/// #               Ok(|state| (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap()))
/// #           });
/// #   });
/// #
/// #   let test_server = TestServer::new(router).unwrap();
/// #   let response = test_server.client().get("https://example.com/").perform().unwrap();
/// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
/// # }
/// ```
///
/// Recording a piece of state data before passing the request through:
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate gotham_derive;
/// # extern crate hyper;
/// #
/// # use hyper::{Response, StatusCode};
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::pipeline::*;
/// # use gotham::pipeline::single::*;
/// # use gotham::router::builder::*;
/// # use gotham::state::State;
/// # use gotham::test::TestServer;
/// #
/// #[derive(NewMiddleware, Copy, Clone)]
/// struct MiddlewareWithStateData;
///
/// #[derive(StateData)]
/// struct MiddlewareStateData {
///     i: i32,
/// }
///
/// impl Middleware for MiddlewareWithStateData {
///     fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
///     {
///         state.put(MiddlewareStateData { i: 10 });
///         chain(state)
///     }
/// }
/// #
/// # fn main() {
/// #   let (chain, pipelines) = single_pipeline(
/// #       new_pipeline()
/// #           .add(MiddlewareWithStateData)
/// #           .build()
/// #   );
/// #
/// #   let router = build_router(chain, pipelines, |route| {
/// #       route
/// #           .get("/")
/// #           .to_new_handler(|| {
/// #               Ok(|mut state: State| {
/// #                   let data = state.take::<MiddlewareStateData>();
/// #                   let body = format!("{}", data.i).into_bytes();
/// #                   (state, Response::builder().status(StatusCode::OK).body(body.into()).unwrap())
/// #               })
/// #           });
/// #   });
/// #
/// #   let test_server = TestServer::new(router).unwrap();
/// #   let response = test_server.client().get("https://example.com/").perform().unwrap();
/// #   assert_eq!(response.status(), StatusCode::OK);
/// #   let body = response.read_utf8_body().unwrap();
/// #   assert_eq!(&body, "10");
/// # }
/// ```
///
/// Decorating the response after the request has completed:
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate gotham_derive;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use futures::Future;
/// # use hyper::{Body, Response, StatusCode};
/// # use hyper::header::WARNING;
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::pipeline::*;
/// # use gotham::pipeline::single::*;
/// # use gotham::router::builder::*;
/// # use gotham::state::State;
/// # use gotham::test::TestServer;
/// #
/// #[derive(NewMiddleware, Copy, Clone)]
/// struct MiddlewareAddingResponseHeader;
///
/// impl Middleware for MiddlewareAddingResponseHeader {
///     fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
///     {
///         let f = chain(state)
///             .map(|(state, mut response)| {
///                 response.headers_mut().insert(WARNING, "299 example.com Deprecated".parse().unwrap());
///                 (state, response)
///             });
///
///         Box::new(f)
///     }
/// }
/// #
/// # fn main() {
/// #   let (chain, pipelines) = single_pipeline(
/// #       new_pipeline()
/// #           .add(MiddlewareAddingResponseHeader)
/// #           .build()
/// #   );
/// #
/// #   let router = build_router(chain, pipelines, |route| {
/// #       route
/// #           .get("/")
/// #           .to_new_handler(|| {
/// #               Ok(|state| (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap()))
/// #           });
/// #   });
/// #
/// #   let test_server = TestServer::new(router).unwrap();
/// #   let response = test_server.client().get("https://example.com/").perform().unwrap();
/// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
/// #
/// #   {
/// #       let warning = response.headers().get(WARNING).unwrap();
/// #       assert_eq!(warning, "299 example.com Deprecated");
/// #   }
/// # }
/// ```
///
/// Terminating the request early based on some arbitrary condition:
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate gotham_derive;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use hyper::{Body, Response, Method, StatusCode};
/// # use futures::future;
/// # use gotham::helpers::http::response::create_empty_response;
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::pipeline::*;
/// # use gotham::pipeline::single::*;
/// # use gotham::router::builder::*;
/// # use gotham::state::{State, FromState};
/// # use gotham::test::TestServer;
/// #
/// #[derive(NewMiddleware, Copy, Clone)]
/// struct ConditionalMiddleware;
///
/// impl Middleware for ConditionalMiddleware {
///     fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
///     {
///         if *Method::borrow_from(&state) == Method::GET {
///             chain(state)
///         } else {
///             let response = create_empty_response(&state, StatusCode::METHOD_NOT_ALLOWED);
///             Box::new(future::ok((state, response)))
///         }
///     }
/// }
/// #
/// # fn main() {
/// #   let (chain, pipelines) = single_pipeline(
/// #       new_pipeline()
/// #           .add(ConditionalMiddleware)
/// #           .build()
/// #   );
/// #
/// #   let router = build_router(chain, pipelines, |route| {
/// #       route
/// #           .get_or_head("/")
/// #           .to_new_handler(|| {
/// #               Ok(|state| (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap()))
/// #           });
/// #   });
/// #
/// #   let test_server = TestServer::new(router).unwrap();
/// #
/// #   let response = test_server.client().get("https://example.com/").perform().unwrap();
/// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
/// #
/// #   let response = test_server.client().head("https://example.com/").perform().unwrap();
/// #   assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
/// # }
/// ```
///
/// Asynchronous middleware, which continues the request after some action completes:
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate gotham_derive;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use futures::{future, Future};
/// # use hyper::{Body, Response, StatusCode};
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::pipeline::*;
/// # use gotham::pipeline::single::*;
/// # use gotham::router::builder::*;
/// # use gotham::state::State;
/// # use gotham::test::TestServer;
/// #
/// #[derive(NewMiddleware, Copy, Clone)]
/// struct AsyncMiddleware;
///
/// impl Middleware for AsyncMiddleware {
///     fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static
///     {
///         // This could be any asynchronous action. `future::lazy(_)` defers a function
///         // until the next cycle of tokio's event loop.
///         let f = future::lazy(|| future::ok(()));
///         Box::new(f.and_then(move |_| chain(state)))
///     }
/// }
/// #
/// # fn main() {
/// #   let (chain, pipelines) = single_pipeline(
/// #       new_pipeline()
/// #           .add(AsyncMiddleware)
/// #           .build()
/// #   );
/// #
/// #   let router = build_router(chain, pipelines, |route| {
/// #       route
/// #           .get("/")
/// #           .to_new_handler(|| {
/// #               Ok(|state| (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap()))
/// #           });
/// #   });
/// #
/// #   let test_server = TestServer::new(router).unwrap();
/// #   let response = test_server.client().get("https://example.com/").perform().unwrap();
/// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
/// # }
/// ```
pub trait Middleware {
    /// Entry point to the middleware. To pass the request on to the application, the middleware
    /// invokes the `chain` function with the provided `state`.
    ///
    /// By convention, the middleware should:
    ///
    /// * Not modify any request components added to `State` by Gotham.
    /// * Avoid modifying parts of the `State` that don't strictly need to be modified to perform
    ///   its function.
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
        Self: Sized;
}

/// A type which is used to spawn new `Middleware` values. When implementing a `Middleware`, this
/// defines how instances of the `Middleware` are created.
///
/// This can be derived by `Middleware` that implement `Clone`, and will result in the following
/// implementation:
///
/// ```rust
/// # extern crate gotham;
/// #
/// # use std::io;
/// # use gotham::middleware::{NewMiddleware, Middleware};
/// # use gotham::handler::HandlerFuture;
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::state::State;
/// #
/// # #[allow(unused)]
/// # #[derive(Copy, Clone)]
/// struct MyMiddleware;
///
/// impl NewMiddleware for MyMiddleware {
///     type Instance = Self;
///
///     fn new_middleware(&self) -> io::Result<Self::Instance> {
///         Ok(self.clone())
///     }
/// }
/// #
/// # impl Middleware for MyMiddleware {
/// #   fn call<Chain>(self, _state: State, _chain: Chain) -> Box<HandlerFuture>
/// #       where Chain: FnOnce(State) -> Box<HandlerFuture> + 'static
/// #   {
/// #       unimplemented!()
/// #   }
/// # }
/// #
/// # fn main() {
/// #   // Just for the implied type assertion.
/// #   new_pipeline().add(MyMiddleware).build();
/// # }
pub trait NewMiddleware: Sync + RefUnwindSafe {
    /// The type of `Middleware` created by the `NewMiddleware`.
    type Instance: Middleware;

    /// Create and return a new `Middleware` value.
    fn new_middleware(&self) -> io::Result<Self::Instance>;
}
