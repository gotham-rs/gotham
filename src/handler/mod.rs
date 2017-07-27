//! Defines types for Gotham handlers
//!
//! A function can be used directly as a handler using one of the [default implementations of
//! `Handler`][handler-impl], but the trait can also be implemented directly for greater control.
//!
//! [handler-impl]: trait.Handler.html#implementors

use std::io;
use std::sync::Arc;
use std::error::Error;

use chrono;
use hyper;
use hyper::server;
use hyper::{Request, Response};
use futures::{future, Future};
use futures_cpupool::{CpuPool, CpuFuture};

use state::{State, set_request_id, request_id};
use http::request::path::RequestPathSegments;
use http::header::XRuntimeMicroseconds;

/// A type alias for the trait objects returned by `HandlerService`
pub type HandlerFuture = Future<Item = (State, Response), Error = (State, hyper::Error)> + Send;

/// Wraps a `NewHandler` to provide a `hyper::server::NewService` implementation for Gotham
/// handlers.
pub struct NewHandlerService<T>
    where T: NewHandler + 'static
{
    t: Arc<T>,
    pool: Arc<CpuPool>,
}

impl<T> Clone for NewHandlerService<T>
    where T: NewHandler + 'static
{
    fn clone(&self) -> Self {
        NewHandlerService {
            t: self.t.clone(),
            pool: self.pool.clone(),
        }
    }
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
    /// # use gotham::http::response::create_response;
    /// # use gotham::handler::NewHandlerService;
    /// # use gotham::state::State;
    /// # use hyper::{Request, Response};
    /// # use hyper::StatusCode;
    /// #
    /// # fn main() {
    /// fn handler(state: State, _req: Request) -> (State, Response) {
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
    /// # use hyper::server::{Request, Response};
    /// # use hyper::{StatusCode, Method};
    /// #
    /// # fn main() {
    /// fn handler(state: State, _req: Request) -> (State, Response) {
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
        NewHandlerService {
            t: Arc::new(t),
            pool: Arc::new(CpuPool::new_num_cpus()),
        }
    }
}

impl<T> server::NewService for NewHandlerService<T>
    where T: NewHandler + 'static
{
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = Self;

    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl<T> server::Service for NewHandlerService<T>
    where T: NewHandler
{
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = CpuFuture<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let s = chrono::UTC::now();
        let mut state = State::new();
        set_request_id(&mut state, &req);

        trace!("[{}] populating immutable request data into state",
               request_id(&state));
        state.put(req.method().clone());
        state.put(req.uri().clone());
        state.put(req.version().clone());
        state.put(req.headers().clone());
        state.put(RequestPathSegments::new(req.uri().path().clone()));

        info!("[REQUEST][{}][{}][{}]",
              request_id(&state),
              req.method(),
              req.path());

        // Hyper doesn't allow us to present an affine-typed `Handler` interface directly. We have
        // to emulate the promise given by hyper's documentation, by creating a `Handler` value and
        // immediately consuming it.
        match self.t.new_handler() {
            Ok(handler) => {
                self.pool
                    .spawn_fn(move || {
                        handler
                            .handle(state, req)
                            .and_then(move |(state, res)| {
                                let f = chrono::UTC::now();
                                match f.signed_duration_since(s).num_microseconds() {
                                    Some(dur) => {
                                        info!("[RESPONSE][{}][{}][{}][{}µs]",
                                              request_id(&state),
                                              res.version(),
                                              res.status(),
                                              dur);

                                        future::ok(res.with_header(XRuntimeMicroseconds(dur)))
                                    }
                                    None => {
                                        // Valid response is still sent to client in this case but
                                        // timing has failed and should be looked into.
                                        error!("[RESPONSE][{}][{}][{}][invalid]",
                                               request_id(&state),
                                               res.version(),
                                               res.status());
                                        future::ok(res)
                                    }
                                }
                            })
                            .or_else(move |(state, err)| {
                                let f = chrono::UTC::now();
                                match f.signed_duration_since(s).num_microseconds() {
                                    Some(dur) => {
                                        error!("[ERROR][{}][Error: {}][{}]",
                                               request_id(&state),
                                               err.description(),
                                               dur);
                                    }
                                    None => {
                                        error!("[ERROR][{}][Error: {}][invalid]",
                                               request_id(&state),
                                               err.description());
                                    }
                                }
                                future::err(err)
                            })
                            .boxed()
                    })
            }
            Err(e) => self.pool.spawn(future::err(e.into())),
        }
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
pub trait Handler: Send + Sync {
    /// Handles the request, returning a boxed future which resolves to a response.
    fn handle(self, State, Request) -> Box<HandlerFuture>;
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
/// The only default implementation is the noop which converts a `hyper::Response` by
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
/// # use hyper::server::{Request, Response};
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
/// fn handler(state: State, _req: Request) -> (State, MyStruct) {
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
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl<F, R> Handler for F
    where F: FnOnce(State, Request) -> R + Send + Sync,
          R: IntoHandlerFuture
{
    fn handle(self, state: State, req: Request) -> Box<HandlerFuture> {
        self(state, req).into_handler_future()
    }
}
