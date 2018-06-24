//! Defines the types for connecting multiple middleware into a "chain" when forming a pipeline.

use std::io;
use std::panic::RefUnwindSafe;

use handler::HandlerFuture;
use middleware::{Middleware, NewMiddleware};
use state::{request_id, State};

/// A recursive type representing a pipeline, which is used to spawn a `MiddlewareChain`.
///
/// This type should never be implemented outside of Gotham, does not form part of the public API,
/// and is subject to change without notice.
#[doc(hidden)]
pub unsafe trait NewMiddlewareChain: RefUnwindSafe + Sized {
    type Instance: MiddlewareChain;

    /// Create and return a new `MiddlewareChain` value.
    fn construct(&self) -> io::Result<Self::Instance>;
}

unsafe impl<T, U> NewMiddlewareChain for (T, U)
where
    T: NewMiddleware,
    T::Instance: Send + 'static,
    U: NewMiddlewareChain,
{
    type Instance = (T::Instance, U::Instance);

    fn construct(&self) -> io::Result<Self::Instance> {
        // This works as a recursive `map` over the "list" of `NewMiddleware`, and is used in
        // creating the `Middleware` instances for serving a single request.
        //
        // The reversed order is preserved in the return value.
        trace!(" adding middleware instance to pipeline");
        let (ref nm, ref tail) = *self;
        Ok((nm.new_middleware()?, tail.construct()?))
    }
}

unsafe impl NewMiddlewareChain for () {
    type Instance = ();

    fn construct(&self) -> io::Result<Self::Instance> {
        // () marks the end of the list, so is returned as-is.
        trace!(" completed middleware pipeline construction");
        Ok(())
    }
}

/// A recursive type representing an instance of a pipeline, which is used to process a single
/// request.
///
/// This type should never be implemented outside of Gotham, does not form part of the public API,
/// and is subject to change without notice.
#[doc(hidden)]
pub unsafe trait MiddlewareChain: Sized {
    /// Recursive function for processing middleware and chaining to the given function.
    fn call<F>(self, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + Send + 'static;
}

unsafe impl MiddlewareChain for () {
    fn call<F>(self, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
    {
        // At the last item in the `MiddlewareChain`, the function is invoked to serve the
        // request. `f` is the nested function of all `Middleware` and the `Handler`.
        //
        // In the case of 0 middleware, `f` is the function created in `MiddlewareChain::call`
        // which invokes the `Handler` directly.
        trace!("pipeline complete, invoking handler");
        f(state)
    }
}

unsafe impl<T, U> MiddlewareChain for (T, U)
where
    T: Middleware + Send + 'static,
    U: MiddlewareChain,
{
    fn call<F>(self, state: State, f: F) -> Box<HandlerFuture>
    where
        F: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
    {
        let (m, p) = self;
        // Construct the function from the inside, out. Starting with a function which calls the
        // `Handler`, and then creating a new function which calls the `Middleware` with the
        // previous function as the `chain` argument, we end up with a structure somewhat like
        // this (using `m0`, `m1`, `m2` as middleware names, where `m2` is the last middleware
        // before the `Handler`):
        //
        //  move |state| {
        //      m0.call(state, move |state| {
        //          m1.call(state, move |state| {
        //              m2.call(state, move |state| handler.call(state))
        //          })
        //      })
        //  }
        //
        // The resulting function is called by `<() as MiddlewareChain>::call`
        trace!("[{}] executing middleware", request_id(&state));
        p.call(state, move |state| m.call(state, f))
    }
}
