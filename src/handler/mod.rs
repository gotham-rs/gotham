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

/// A type alias for the trait objects returned by `HandlerService`
pub type HandlerFuture = Future<Item = server::Response, Error = hyper::Error>;

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
    type Future = Box<HandlerFuture>;

    fn call(&self, req: Self::Request) -> Self::Future {
        self.handler.handle(req)
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
    fn handle(&self, req: Request) -> Box<HandlerFuture>;
}

/// Represents an object which can be converted to a response, perhaps asynchronously. This trait
/// is used in converting the return type of a function into a response.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use gotham::router::{Router, RouterBuilder};
/// # use gotham::handler::{HandlerFuture, IntoHandlerFuture};
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
/// impl IntoHandlerFuture for MyStruct {
///     fn into_handler_future(self) -> Box<HandlerFuture> {
///         let response = Response::new()
///             .with_status(StatusCode::Ok)
///             .with_body(self.value);
///
///         future::ok(response).boxed()
///     }
/// }
///
/// fn handler(req: Request) -> MyStruct {
///     MyStruct::new()
/// }
///
/// fn router() -> Router {
///     Router::build(|routes| {
///        routes.direct(Get, "/").to(handler);
///     })
/// }
///
/// # fn main() {
/// #   router();
/// # }
/// ```
///
/// # Default implementations
///
/// * `hyper::server::Response` &ndash; The response is wrapped in a completed future and boxed
/// * `Box<HandlerFuture>` &ndash; The boxed future is returned directly
pub trait IntoHandlerFuture {
    /// Converts this object into a boxed future resolving to a response.
    fn into_handler_future(self) -> Box<HandlerFuture>;
}

impl IntoHandlerFuture for server::Response {
    fn into_handler_future(self) -> Box<HandlerFuture> {
        future::ok(self).boxed()
    }
}

impl IntoHandlerFuture for Box<HandlerFuture> {
    fn into_handler_future(self) -> Box<HandlerFuture> {
        self
    }
}

impl<F, R> Handler for F
    where F: Fn(Request) -> R + Send + Sync,
          R: IntoHandlerFuture
{
    fn handle(&self, req: Request) -> Box<HandlerFuture> {
        self(req).into_handler_future()
    }
}
