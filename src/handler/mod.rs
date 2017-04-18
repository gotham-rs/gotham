//! Defines types for Gotham handlers
//!
//! A function can be used directly as a handler using one of the [default implementations of
//! `Handler`][handler-impl], but the trait can also be implemented directly for greater control.
//!
//! [handler-impl]: trait.Handler.html#implementors

use hyper;
use hyper::server;
use hyper::server::Request;
use futures::{future, Future};
use state::State;

/// A type alias for the trait objects returned by `HandlerService`
pub type HandlerFuture =
    Future<Item = (State, server::Response), Error = (State, hyper::Error)> + Send;

/// `HandlerService` wraps a Gotham `Handler` and exposes a hyper `Service`.
///
/// The request is served by invoking [`Handler::handle(hyper::server::Request)`][Handler::handle].
///
/// [Handler::handle]: trait.Handler.html#tymethod.handle
pub struct HandlerService<T>
    where T: Handler
{
    handler: T,
}

impl<T> HandlerService<T>
    where T: Handler
{
    /// Creates a new `HandlerService` for the given `Handler`.
    pub fn new(t: T) -> HandlerService<T> {
        HandlerService { handler: t }
    }
}

impl<T> server::Service for HandlerService<T>
    where T: Handler
{
    type Request = server::Request;
    type Response = server::Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = server::Response, Error = hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        self.handler
            .handle(State::new(), req)
            .and_then(|(_, response)| future::ok(response))
            .or_else(|(_, error)| future::err(error))
            .boxed()
    }
}

// TODO: Update this doc comment if necessary when the `Middleware` type is created.
/// A `Handler` receives some subset of requests to the application, and returns a future which
/// resolves to a response. This represents the common entry point for the parts of a Gotham
/// application, implemented by `Router` and `Middleware`.
///
/// A `Handler` is basically an asynchronous function. To anybody familiar with tokio's
/// documentation, this explanation will sound familiar as it's exactly [the description of a tokio
/// `Service`][tokio-simple-server]
///
/// [tokio-simple-server]: https://tokio.rs/docs/getting-started/simple-server/
pub trait Handler: Send + Sync {
    /// Handles the request, returning a boxed future which resolves to a response.
    fn handle(&self, State, Request) -> Box<HandlerFuture>;
}

pub trait NewHandler: Send + Sync {
    type Handler: Handler;

    fn new_handler(&self) -> Self::Handler;
}

impl<F, H> NewHandler for F
    where F: Fn() -> H + Send + Sync,
          H: Handler
{
    type Handler = H;

    fn new_handler(&self) -> H {
        self()
    }
}

/// Represents a type which can be converted into the future type returned by a
/// [`Handler`][Handler].
///
/// [Handler]: trait.Handler.html
pub trait IntoHandlerFuture {
    /// Converts this value into a boxed future resolving to a state and response.
    fn into_handler_future(self) -> Box<HandlerFuture>;
}

impl<T> IntoHandlerFuture for (State, T)
    where T: IntoResponse
{
    fn into_handler_future(self) -> Box<HandlerFuture> {
        let (state, t) = self;
        future::ok((state, t.into_response())).boxed()
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
/// The only default implementation is the noop which converts a `hyper::server::Response` by
/// returning the value unmodified.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use gotham::state::State;
/// # use gotham::router::{Router, RouterBuilder};
/// # use gotham::handler::IntoResponse;
/// # use futures::{future, Future};
/// # use hyper::Method::Get;
/// # use hyper::StatusCode;
/// # use hyper::server::{Http, Request, Response};
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
///     fn into_response(self) -> Response {
///         Response::new()
///             .with_status(StatusCode::Ok)
///             .with_body(self.value)
///     }
/// }
///
/// fn handler(state: State, req: Request) -> (State, MyStruct) {
///     (state, MyStruct::new())
/// }
///
/// fn router() -> Router {
///     Router::build(|routes| {
///        routes.direct(Get, "/").to(handler);
///     })
/// }
/// #
/// # fn main() {
/// #   router();
/// # }
/// ```
///
/// # Default implementations
///
/// * `hyper::server::Response` &ndash; The response is wrapped in a completed future and boxed
/// * `Box<HandlerFuture>` &ndash; The boxed future is returned directly
pub trait IntoResponse {
    /// Converts this value into a `hyper::server::Response`
    fn into_response(self) -> server::Response;
}

impl IntoResponse for server::Response {
    fn into_response(self) -> server::Response {
        self
    }
}

impl<F, R> Handler for F
    where F: Fn(State, Request) -> R + Send + Sync,
          R: IntoHandlerFuture
{
    fn handle(&self, state: State, req: Request) -> Box<HandlerFuture> {
        self(state, req).into_handler_future()
    }
}
