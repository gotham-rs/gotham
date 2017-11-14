//! Defines types for Gotham handlers
//!
//! A function can be used directly as a handler using one of the [default implementations of
//! `Handler`][handler-impl], but the trait can also be implemented directly for greater control.
//!
//! [handler-impl]: trait.Handler.html#implementors

use std::io;
use std::sync::Arc;
use std::panic::{AssertUnwindSafe, RefUnwindSafe};

use hyper;
use hyper::server::{NewService, Service};
use hyper::{Request, Response};
use futures::{future, Future};

use state::State;
use http::request::path::RequestPathSegments;

mod error;
mod timing;
mod trap;

pub use self::error::{HandlerError, IntoHandlerError};

/// A type alias for the trait objects returned by `HandlerService`.
///
/// When the `Future` resolves to an error, the `(State, HandlerError)` value is used to generate
/// an appropriate HTTP error response.
pub type HandlerFuture = Future<Item = (State, Response), Error = (State, HandlerError)>;

/// Wraps a `NewHandler` to provide a `hyper::server::NewService` implementation for Gotham
/// handlers.
pub struct NewHandlerService<T>
where
    T: NewHandler + 'static,
{
    t: Arc<T>,
}

impl<T> Clone for NewHandlerService<T>
where
    T: NewHandler + 'static,
{
    fn clone(&self) -> Self {
        NewHandlerService { t: self.t.clone() }
    }
}

impl<T> NewHandlerService<T>
where
    T: NewHandler + 'static,
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
    /// #
    /// # use gotham::http::response::create_response;
    /// # use gotham::handler::NewHandlerService;
    /// # use gotham::state::State;
    /// # use hyper::Response;
    /// # use hyper::StatusCode;
    /// #
    /// # fn main() {
    /// fn handler(state: State) -> (State, Response) {
    ///     let res = create_response(&state, StatusCode::Accepted, None);
    ///     (state, res)
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
    /// #
    /// # use gotham::http::response::create_response;
    /// # use gotham::handler::NewHandlerService;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::tree::TreeBuilder;
    /// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
    /// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
    /// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
    /// # use gotham::router::request::path::NoopPathExtractor;
    /// # use gotham::router::request::query_string::NoopQueryStringExtractor;
    /// # use gotham::router::response::finalizer::ResponseFinalizerBuilder;
    /// # use hyper::Response;
    /// # use hyper::{StatusCode, Method};
    /// #
    /// # fn main() {
    /// fn handler(state: State) -> (State, Response) {
    ///     let res = create_response(&state, StatusCode::Accepted, None);
    ///     (state, res)
    /// }
    ///
    /// let mut tree_builder = TreeBuilder::new();
    /// let pipeline_set = finalize_pipeline_set(new_pipeline_set());
    /// let finalizer = ResponseFinalizerBuilder::new().finalize();
    ///
    /// let matcher = MethodOnlyRouteMatcher::new(vec![Method::Get]);
    /// let dispatcher = DispatcherImpl::new(|| Ok(handler), (), pipeline_set);
    /// let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
    /// let route = RouteImpl::new(matcher, Box::new(dispatcher), extractors, Delegation::Internal);
    ///
    /// tree_builder.add_route(Box::new(route));
    /// let tree = tree_builder.finalize();
    /// let router = Router::new(tree, finalizer);
    ///
    /// NewHandlerService::new(router);
    /// # }
    /// ```
    pub fn new(t: T) -> NewHandlerService<T> {
        NewHandlerService { t: Arc::new(t) }
    }
}

impl<T> NewService for NewHandlerService<T>
where
    T: NewHandler + 'static,
{
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = Self;

    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl<T> Service for NewHandlerService<T>
where
    T: NewHandler,
{
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let (method, uri, version, headers, body) = req.deconstruct();

        let mut state = State::new();
        state.put(RequestPathSegments::new(uri.path()));
        state.put(method);
        state.put(uri);
        state.put(version);
        state.put(headers);
        state.put(body);
        state.set_request_id();

        trap::call_handler(self.t.as_ref(), AssertUnwindSafe(state))
    }
}

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
/// # use gotham::router::Router;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
/// # use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
/// # use gotham::handler::IntoResponse;
/// # use gotham::router::request::path::NoopPathExtractor;
/// # use gotham::router::request::query_string::NoopQueryStringExtractor;
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
///
/// # Default implementations
///
/// * `hyper::Response` &ndash; The response is wrapped in a completed future and boxed
/// * `Box<HandlerFuture>` &ndash; The boxed future is returned directly
pub trait IntoResponse {
    /// Converts this value into a `hyper::Response`
    fn into_response(self, state: &State) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self, _state: &State) -> Response {
        self
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
