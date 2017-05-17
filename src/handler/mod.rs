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
use std::io;

/// A type alias for the trait objects returned by `HandlerService`
pub type HandlerFuture =
    Future<Item = (State, server::Response), Error = (State, hyper::Error)> + Send;

/// Wraps a `NewHandler` to provide a `hyper::server::NewService` implementation for Gotham
/// handlers.
pub struct NewHandlerService<T>
    where T: NewHandler + 'static
{
    t: T,
}

impl<T> NewHandlerService<T>
    where T: NewHandler + 'static
{
    /// Creates a `NewHandlerService` for the given `NewHandler`.
    ///
    /// # Examples
    ///
    /// Using a closure which is a `NewHandler`:
    ///
    /// ```rust,no_run
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate borrow_bag;
    /// #
    /// # use gotham::handler::{NewHandlerService, NewHandler, Handler};
    /// # use gotham::state::State;
    /// # use hyper::server::{Request, Response};
    /// # use hyper::StatusCode;
    /// #
    /// # fn main() {
    /// fn handler(state: State, request: Request) -> (State, Response) {
    ///     (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// NewHandlerService::new(|| Ok(handler));
    /// # }
    /// ```
    ///
    /// Using a `Router`:
    ///
    /// ```rust,no_run
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate borrow_bag;
    /// #
    /// # use gotham::handler::{NewHandlerService, NewHandler, Handler};
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::tree::Tree;
    /// # use gotham::router::route::RouteImpl;
    /// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
    /// # use gotham::dispatch::Dispatcher;
    /// # use hyper::server::{Request, Response};
    /// # use hyper::{StatusCode, Method};
    /// #
    /// # fn main() {
    /// fn handler(state: State, request: Request) -> (State, Response) {
    ///     (state, Response::new().with_status(StatusCode::Accepted))
    /// }
    ///
    /// let mut tree = Tree::new();
    /// let pipelines = borrow_bag::new_borrow_bag();
    /// let not_found = || Ok(handler);
    /// let internal_server_error = || Ok(handler);
    /// let matcher = MethodOnlyRequestMatcher::new(vec![Method::Get]);
    ///
    /// let dispatcher = Dispatcher::new(|| Ok(handler), ());
    /// let route = Box::new(RouteImpl::new(matcher, dispatcher));
    ///
    ///  tree.add_route(route);
    ///  let router = Router::new(tree, pipelines, not_found, internal_server_error);
    ///
    ///  NewHandlerService::new(router);
    /// # }
    /// ```
    pub fn new(t: T) -> NewHandlerService<T> {
        NewHandlerService { t: t }
    }
}

impl<T> server::NewService for NewHandlerService<T>
    where T: NewHandler + 'static
{
    type Request = server::Request;
    type Response = server::Response;
    type Error = hyper::Error;
    type Instance = HandlerService<T::Instance>;

    fn new_service(&self) -> io::Result<Self::Instance> {
        self.t.new_handler().map(HandlerService::new)
    }
}

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

// TODO: Ensure this is actually true in the new implementation of `Router`
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
pub trait Handler: Send + Sync {
    /// Handles the request, returning a boxed future which resolves to a response.
    fn handle(&self, State, Request) -> Box<HandlerFuture>;
}

/// Creates new `Handler` values.
pub trait NewHandler: Send + Sync {
    /// The type of `Handler` created by the implementor.
    type Instance: Handler;

    /// Create and return a new `Handler` value.
    fn new_handler(&self) -> io::Result<Self::Instance>;
}

impl<F, H> NewHandler for F
    where F: Fn() -> io::Result<H> + Send + Sync,
          H: Handler
{
    type Instance = H;

    fn new_handler(&self) -> io::Result<H> {
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
/// # extern crate borrow_bag;
/// #
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::route::RouteImpl;
/// # use gotham::router::tree::Tree;
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::dispatch::Dispatcher;
/// # use gotham::handler::{NewHandler, IntoResponse};
/// # use futures::{future, Future};
/// # use hyper::Method;
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
/// # fn main() {
/// #   let mut tree = Tree::new();
/// #   let pipelines = borrow_bag::new_borrow_bag();
/// #   let not_found = || Ok(handler);
/// #   let internal_server_error = || Ok(handler);
/// #   let matcher = MethodOnlyRequestMatcher::new(vec![Method::Get]);
/// #
///     let dispatcher = Dispatcher::new(|| Ok(handler), ());
///     let route = Box::new(RouteImpl::new(matcher, dispatcher));
///
///     tree.add_route(route);
///     Router::new(tree, pipelines, not_found, internal_server_error);
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
