//! Defines types for Gotham handlers
//!
//! A function can be used directly as a handler using one of the [default implementations of
//! `Handler`][handler-impl], but the trait can also be implemented directly for greater control.
//!
//! [handler-impl]: trait.Handler.html#implementors
use std::io;
use std::panic::RefUnwindSafe;

use hyper::Response;
use futures::{future, Future};

use state::State;

mod error;

pub use self::error::{HandlerError, IntoHandlerError};

/// A type alias for the trait objects returned by `HandlerService`.
///
/// When the `Future` resolves to an error, the `(State, HandlerError)` value is used to generate
/// an appropriate HTTP error response.
pub type HandlerFuture = Future<Item = (State, Response), Error = (State, HandlerError)>;

/// A `Handler` receives some subset of requests to the application, and returns a future which
/// resolves to a response. This represents the common entry point for the parts of a Gotham
/// application, implemented by `Router` and `Pipeline`.
///
/// The `Handler` is created by its `NewHandler` implementation, and is used for a single request.
///
/// A `Handler` is basically an asynchronous function. To anybody familiar with tokio's
/// documentation, this explanation will sound familiar as it's exactly [the description of a tokio
/// `Service`][tokio-simple-server]
///
/// [tokio-simple-server]: https://tokio.rs/docs/getting-started/simple-server/
pub trait Handler {
    /// Handles the request, returning a boxed future which resolves to a response.
    fn handle(self, state: State) -> Box<HandlerFuture>;
}

/// Creates new `Handler` values.
pub trait NewHandler: Send + Sync + RefUnwindSafe {
    /// The type of `Handler` created by the implementor.
    type Instance: Handler;

    /// Create and return a new `Handler` value.
    fn new_handler(&self) -> io::Result<Self::Instance>;
}

impl<F, H> NewHandler for F
where
    F: Fn() -> io::Result<H> + Send + Sync + RefUnwindSafe,
    H: Handler,
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
pub trait IntoHandlerFuture {
    /// Converts this value into a boxed future resolving to a state and response.
    fn into_handler_future(self) -> Box<HandlerFuture>;
}

impl<T> IntoHandlerFuture for (State, T)
where
    T: IntoResponse,
{
    fn into_handler_future(self) -> Box<HandlerFuture> {
        let (state, t) = self;
        let response = t.into_response(&state);
        Box::new(future::ok((state, response)))
    }
}

impl IntoHandlerFuture for Box<HandlerFuture> {
    fn into_handler_future(self) -> Box<HandlerFuture> {
        self
    }
}

/// Represents a type which can be converted to a response. This trait is used in converting the
/// return type of a function into a response.
///
/// The only default implementation is the noop which converts a `hyper::Response` by
/// returning the value unmodified.
///
/// # Examples
///
/// ```rust
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
///             .with_status(StatusCode::Ok)
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
/// #   let matcher = MethodOnlyRouteMatcher::new(vec![Method::Get]);
/// #   let dispatcher = DispatcherImpl::new(|| Ok(handler), (), pipeline_set);
/// #   let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
/// #   let route = RouteImpl::new(matcher, Box::new(dispatcher), extractors, Delegation::Internal);
///     tree_builder.add_route(Box::new(route));
///     let tree = tree_builder.finalize();
///     Router::new(tree, finalizer);
/// # }
/// ```

pub trait IntoResponse {
    /// Converts this value into a `hyper::Response`
    fn into_response(self, state: &State) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self, _state: &State) -> Response {
        self
    }
}

impl<T, E> IntoResponse for ::std::result::Result<T, E>
where
    T: IntoResponse,
    E: IntoResponse,
{
    fn into_response(self, state: &State) -> Response {
        match self {
            Ok(res) => res.into_response(state),
            Err(e) => e.into_response(state),
        }
    }
}

impl<F, R> Handler for F
where
    F: FnOnce(State) -> R,
    R: IntoHandlerFuture,
{
    fn handle(self, state: State) -> Box<HandlerFuture> {
        self(state).into_handler_future()
    }
}
