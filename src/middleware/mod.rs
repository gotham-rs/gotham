//! Defines types for Gotham middleware

use handler::HandlerFuture;
use state::State;
use hyper::server::Request;

pub mod pipeline;

/// `Middleware` has the opportunity to provide additional behaviour to the request / response
/// interaction. Middleware-specific state data can be recorded in the [`State`][State] struct for
/// use elsewhere.
///
/// [State]: ../state/struct.State.html
///
/// # Examples
///
/// Taking no action, and immediately passing the request through to the rest of the application:
///
/// ```rust,no_run
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use gotham::handler::{Handler, HandlerFuture};
/// # use gotham::middleware::Middleware;
/// # use gotham::state::State;
/// # use hyper::server::{Request, Response};
/// #
/// struct NoopMiddleware;
///
/// impl Middleware for NoopMiddleware {
///     fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
///     {
///         chain(state, req)
///     }
/// }
/// #
/// # fn main() {}
/// ```
///
/// Recording a piece of state data before passing the request through:
///
/// ```rust,no_run
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use gotham::handler::{Handler, HandlerFuture};
/// # use gotham::middleware::Middleware;
/// # use gotham::state::{State, StateData};
/// # use hyper::server::{Request, Response};
/// #
/// struct MiddlewareWithStateData;
///
/// struct MiddlewareStateData {
///     i: i32,
/// }
///
/// impl StateData for MiddlewareStateData {}
///
/// impl Middleware for MiddlewareWithStateData {
///     fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
///     {
///         state.put(MiddlewareStateData { i: 10 });
///         chain(state, req)
///     }
/// }
/// #
/// # fn main() {}
/// ```
///
/// Terminating the request early based on some arbitrary condition:
///
/// ```rust,no_run
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate futures;
/// #
/// # use gotham::handler::{Handler, HandlerFuture};
/// # use gotham::middleware::Middleware;
/// # use gotham::state::{State, StateData};
/// # use hyper::server::{Request, Response};
/// # use hyper::{Method, StatusCode};
/// # use futures::{future, Future};
/// #
/// struct ConditionalMiddleware;
///
/// impl Middleware for ConditionalMiddleware {
///     fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
///     {
///         if *req.method() == Method::Get {
///             chain(state, req)
///         } else {
///             let response = Response::new().with_status(StatusCode::MethodNotAllowed);
///             future::ok(response).boxed()
///         }
///     }
/// }
/// #
/// # fn main() {}
/// ```
///
/// # Notes
///
/// **Note:** Data which is captured in functions passed to future combinators **must** be moved
/// into the function, or else the function won't have the correct type inferred. Importantly, this
/// means that the `State` reference (and by extension, any reference returned from
/// `state.borrow::<T>()` or `state.borrow_mut::<T>()`) cannot be used in such a function.
///
/// Two recommended approaches are:
///
/// **1\.** Retain the data only in the middleware function scope:
///
/// ```rust,no_run
/// # extern crate gotham;
/// # extern crate futures;
/// # extern crate hyper;
/// #
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::state::State;
/// # use hyper::server::{Request, Response};
/// # use futures::{future, Future};
/// use std::time::Instant;
///
/// struct ElapsedTimeMiddleware;
///
/// impl Middleware for ElapsedTimeMiddleware {
///     fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
///     {
///         let start_instant = Instant::now();
///         chain(state, req).and_then(move |response| {
///             let duration = start_instant.elapsed();
///             println!("Request was handled in {}s", duration.as_secs());
///             future::ok(response)
///         }).boxed()
///     }
/// }
/// #
/// # fn main() {}
/// ```
///
/// **2\.** Move the data out of `State`:
///
/// ```rust,no_run
/// # extern crate gotham;
/// # extern crate futures;
/// # extern crate hyper;
/// #
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::state::{State, StateData};
/// # use hyper::server::{Request, Response};
/// # use futures::{future, Future};
/// use std::time::Instant;
///
/// struct ElapsedTimeMiddleware;
///
/// struct ElapsedTimeData(Instant);
///
/// impl StateData for ElapsedTimeData {}
///
/// impl Middleware for ElapsedTimeMiddleware {
///     fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
///     {
///         state.put(ElapsedTimeData(Instant::now()));
///         let result = chain(state, req);
///         let start_instant = state.take::<ElapsedTimeData>().unwrap().0;
///
///         result.and_then(move |response| {
///             let duration = start_instant.elapsed();
///             println!("Request was handled in {}s", duration.as_secs());
///             future::ok(response)
///         }).boxed()
///     }
/// }
/// #
/// # fn main() {}
/// ```
///
/// The following will **not** work, because the `&mut State` is borrowed by the closure:
///
/// ```rust,no_run
/// # extern crate gotham;
/// # extern crate futures;
/// # extern crate hyper;
/// #
/// # use gotham::handler::HandlerFuture;
/// # use gotham::middleware::Middleware;
/// # use gotham::state::{State, StateData};
/// # use hyper::server::{Request, Response};
/// # use futures::{future, Future};
/// # use std::time::Instant;
/// #
/// # struct ElapsedTimeMiddleware;
/// #
/// # struct ElapsedTimeData(Instant);
/// #
/// # impl StateData for ElapsedTimeData {}
/// #
/// impl Middleware for ElapsedTimeMiddleware {
///     fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
///         where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
///     {
///         state.put(ElapsedTimeData(Instant::now()));
///         chain(state, req).and_then(move |response| {
/// # /*
///             let duration = state.take::<ElapsedTimeData>().unwrap().0.elapsed();
///             println!("Request was handled in {}s", duration.as_secs());
/// # */
///             future::ok(response)
///         }).boxed()
///         // ^^^^^ the trait `std::marker::Send` is not implemented for `std::any::Any + 'static`
///     }
/// }
/// #
/// # fn main() {}
/// ```
pub trait Middleware {
    /// Entry point to the middleware. To pass the request on to the application, the middleware
    /// invokes the `chain` function with the provided `state` and `request`.
    ///
    /// By convention, the middleware should:
    ///
    /// * Avoid modifying the `Request`, unless it is already determined that the response will be
    ///   generated by the middleware (i.e. without calling `chain`);
    /// * Ensure to pass the same `&mut State` to `chain`, rather than creating a new `State`.
    fn call<Chain>(&self, state: &mut State, request: Request, chain: Chain) -> Box<HandlerFuture>
        where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>,
              Self: Sized;
}
