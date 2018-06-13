//! Defines types for handlers, the primary building block of a Gotham application.
//!
//! A function can be used directly as a handler using one of the default implementations of
//! `Handler`, but the traits can also be implemented directly for greater control. See the
//! `Handler` trait for some examples of valid handlers.
use std::io;
use std::panic::RefUnwindSafe;

use futures::{future, Future};
use hyper::{Body, Response};

use state::State;

mod error;

pub use self::error::{HandlerError, IntoHandlerError};

/// A type alias for the trait objects returned by `HandlerService`.
///
/// When the `Future` resolves to an error, the `(State, HandlerError)` value is used to generate
/// an appropriate HTTP error response.
pub type HandlerFuture<T = Body> = Future<Item = (State, T), Error = (State, HandlerError)> + Send;

/// A `Handler` is an asynchronous function, taking a `State` value which represents the request
/// and related runtime state, and returns a future which resolves to a response.
///
/// This represents the common entry point for the parts of a Gotham application, and is used with
/// the `Router` API to describe how a request should be dispatched and handled.
///
/// The `Handler` is created and consumed by each request. In the most common case (a bare function
/// acting as a `Handler`) the `Handler + Copy` traits allow the `Handler` to be copied for each
/// request, and the copy consumed. For a closure or a custom handler, the `NewHandler`
/// implementation creates a `Handler` value for each request.
///
/// # Examples
///
/// The simplest kind of handler is a bare function which returns a synchronous response. This is
/// useful when we don't need to do any I/O before generating a response.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::Response;
/// # use gotham::handler::Handler;
/// # use gotham::state::State;
/// #
/// # fn main() {
/// fn my_handler(_state: State) -> (State, Response) {
///     // Implementation elided.
/// #   unimplemented!()
/// }
/// #
/// # fn assert_type<H>(_h: H) where H: Handler + Copy {}
/// # assert_type(my_handler);
/// # }
/// ```
///
/// An asynchronous handler returns a `HandlerFuture` that will resolve to the response. For
/// example, this allows I/O work to begin, and for the Gotham app to continue generating a
/// response once the work completes.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use gotham::handler::{Handler, HandlerFuture};
/// # use gotham::state::State;
/// #
/// # fn main() {
/// fn async_handler(_state: State) -> Box<HandlerFuture> {
///     // Implementation elided.
/// #   unimplemented!()
/// }
/// #
/// # fn assert_type<H>(_h: H) where H: Handler + Copy {}
/// # assert_type(async_handler);
/// # }
/// ```
///
/// A closure can implement `Handler` automatically, in the same way as a bare function. When
/// constructing a `Handler` in this way, a wrapping closure must also be used to implement the
/// `NewHandler` trait.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use gotham::handler::{HandlerFuture, NewHandler};
/// # use gotham::state::State;
/// # use futures::future;
/// #
/// # fn main() {
/// let new_handler = || {
///     let handler = |_state: State| {
///         // Implementation elided.
/// #       Box::new(future::empty()) as Box<HandlerFuture>
///     };
///     Ok(handler)
/// };
///
/// // Pass `new_handler` to the router, using the `to_new_handler` API.
/// #
/// # fn assert_type<H>(_h: H) where H: NewHandler {}
/// # assert_type(new_handler);
/// # }
/// ```
///
/// A custom handler, which implements the `NewHandler` and `Handler` traits directly for greater
/// control. See the `NewHandler` trait for more examples of custom handlers.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use std::io;
/// # use gotham::handler::{Handler, HandlerFuture, NewHandler};
/// # use gotham::state::State;
/// #
/// # fn main() {
/// #[derive(Copy, Clone)]
/// struct MyCustomHandler;
///
/// impl NewHandler for MyCustomHandler {
///     type Instance = Self;
///
///     fn new_handler(&self) -> io::Result<Self::Instance> {
///         Ok(*self)
///     }
/// }
///
/// impl Handler for MyCustomHandler {
///     fn handle(self, _state: State) -> Box<HandlerFuture> {
///         // Implementation elided.
/// #       unimplemented!()
///     }
/// }
/// #
/// # fn assert_type<H>(_h: H) where H: NewHandler {}
/// # assert_type(MyCustomHandler);
/// # }
/// ```
pub trait Handler: Send {
    /// Handles the request, returning a boxed future which resolves to a response.
    fn handle(self, state: State) -> Box<HandlerFuture>;
}

/// A type which is used to spawn new `Handler` values. When implementing a custom `Handler` type,
/// this is used to define how instances of the `Handler` are created.
///
/// The `Instance` associated type is usually `Self` in the simple case, but can be a different
/// type where greater control is needed over lifetimes.
///
/// # Examples
///
/// A custom handler which implements `NewHandler` by copying itself.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use std::io;
/// # use gotham::handler::{Handler, HandlerFuture, NewHandler};
/// # use gotham::state::State;
/// #
/// # fn main() {
/// #[derive(Copy, Clone)]
/// struct MyCustomHandler;
///
/// impl NewHandler for MyCustomHandler {
///     type Instance = Self;
///
///     fn new_handler(&self) -> io::Result<Self::Instance> {
///         Ok(*self)
///     }
/// }
///
/// impl Handler for MyCustomHandler {
///     fn handle(self, _state: State) -> Box<HandlerFuture> {
///         // Implementation elided.
/// #       unimplemented!()
///     }
/// }
/// #
/// # fn assert_type<H>(_h: H) where H: NewHandler {}
/// # assert_type(MyCustomHandler);
/// # }
/// ```
///
/// A custom handler which implements `NewHandler` using a specific `Instance` type.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use std::io;
/// # use gotham::handler::{Handler, HandlerFuture, NewHandler};
/// # use gotham::state::State;
/// #
/// # fn main() {
/// #[derive(Copy, Clone)]
/// struct MyValueInstantiatingHandler;
///
/// impl NewHandler for MyValueInstantiatingHandler {
///     type Instance = MyHandler;
///
///     fn new_handler(&self) -> io::Result<Self::Instance> {
///         Ok(MyHandler)
///     }
/// }
///
/// struct MyHandler;
///
/// impl Handler for MyHandler {
///     fn handle(self, _state: State) -> Box<HandlerFuture> {
///         // Implementation elided.
/// #       unimplemented!()
///     }
/// }
/// #
/// # fn assert_type<H>(_h: H) where H: NewHandler {}
/// # assert_type(MyValueInstantiatingHandler);
/// # }
/// ```
pub trait NewHandler: Send + Sync + RefUnwindSafe {
    /// The type of `Handler` created by the `NewHandler`.
    type Instance: Handler + Send;

    /// Create and return a new `Handler` value.
    fn new_handler(&self) -> io::Result<Self::Instance>;
}

impl<F, H> NewHandler for F
where
    F: Fn() -> io::Result<H> + Send + Sync + RefUnwindSafe,
    H: Handler + Send,
{
    type Instance = H;

    fn new_handler(&self) -> io::Result<H> {
        self()
    }
}

/// Represents a type which can be converted into the future type returned by a `Handler`.
///
/// This is used to allow functions with different return types to satisfy the `Handler` trait
/// bound via the generic function implementation.
pub trait IntoHandlerFuture<B> {
    /// Converts this value into a boxed future resolving to a state and response.
    fn into_handler_future(self) -> Box<HandlerFuture<B>>;
}

impl<B, T> IntoHandlerFuture<B> for (State, T)
where
    T: IntoResponse<B>,
{
    fn into_handler_future(self) -> Box<HandlerFuture<B>> {
        let (state, t) = self;
        let response = t.into_response(&state);
        Box::new(future::ok((state, response)))
    }
}

impl<B> IntoHandlerFuture<B> for Box<HandlerFuture<B>> {
    fn into_handler_future(self) -> Box<HandlerFuture<B>> {
        self
    }
}

/// Represents a type which can be converted to a response. This trait is used in converting the
/// return type of a function into a response.
///
/// # Examples
///
/// ```rust
/// # #![allow(deprecated)] // TODO: Refactor this.
/// #
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use gotham::state::State;
/// # use gotham::pipeline::set::*;
/// # use gotham::router::Router;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
/// # use gotham::router::route::dispatch::DispatcherImpl;
/// # use gotham::handler::IntoResponse;
/// # use gotham::extractor::{NoopPathExtractor, NoopQueryStringExtractor};
/// # use gotham::router::response::finalizer::ResponseFinalizerBuilder;
/// # use hyper::Method;
/// # use hyper::StatusCode;
/// # use hyper::Response;
/// #
/// struct MyStruct {
///     value: String
/// }
///
/// impl MyStruct {
///     fn new() -> MyStruct {
///         // ...
/// #       MyStruct { value: "".to_owned() }
///     }
/// }
///
/// impl IntoResponse for MyStruct {
///     fn into_response(self, _state: &State) -> Response {
///         Response::new()
///             .with_status(StatusCode::OK)
///             .with_body(self.value)
///     }
/// }
///
/// fn handler(state: State) -> (State, MyStruct) {
///     (state, MyStruct::new())
/// }
///
/// # fn main() {
/// #   let mut tree_builder = TreeBuilder::new();
/// #   let pipeline_set = finalize_pipeline_set(new_pipeline_set());
/// #   let finalizer = ResponseFinalizerBuilder::new().finalize();
/// #   let matcher = MethodOnlyRouteMatcher::new(vec![Method::GET]);
/// #   let dispatcher = DispatcherImpl::new(|| Ok(handler), (), pipeline_set);
/// #   let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
/// #   let route = RouteImpl::new(matcher, Box::new(dispatcher), extractors, Delegation::Internal);
///     tree_builder.add_route(Box::new(route));
///     let tree = tree_builder.finalize();
///     Router::new(tree, finalizer);
/// # }
/// ```

pub trait IntoResponse<B> {
    /// Converts this value into a `hyper::Response`
    fn into_response(self, state: &State) -> Response<B>;
}

impl<B> IntoResponse<B> for Response<B> {
    fn into_response(self, _state: &State) -> Response<B> {
        self
    }
}

impl<B, T, E> IntoResponse<B> for ::std::result::Result<T, E>
where
    T: IntoResponse<B>,
    E: IntoResponse<B>,
{
    fn into_response(self, state: &State) -> Response<B> {
        match self {
            Ok(res) => res.into_response(state),
            Err(e) => e.into_response(state),
        }
    }
}

impl<B, F, R> Handler for F
where
    F: FnOnce(State) -> R + Send,
    R: IntoHandlerFuture<B>,
{
    fn handle(self, state: State) -> Box<HandlerFuture> {
        self(state).into_handler_future()
    }
}
