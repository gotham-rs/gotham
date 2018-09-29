//! Defines functionality for processing a request and trapping errors and panics in response
//! generation.

use std::any::Any;
use std::error::Error;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::{io, mem};

use failure;
use futures::future::{self, Future, FutureResult, IntoFuture};
use futures::Async;
use hyper::{Body, Response, StatusCode};

use handler::{Handler, HandlerError, IntoResponse, NewHandler};
use state::{request_id, State};

type CompatError = failure::Compat<failure::Error>;

/// Instantiates a `Handler` from the given `NewHandler`, and invokes it with the request. If a
/// panic occurs from `NewHandler::new_handler` or `Handler::handle`, it is trapped and will result
/// in a `500 Internal Server Error` response.
///
/// Timing information is recorded and logged, except in the case of a panic where the timer is
/// moved and cannot be recovered.
pub(super) fn call_handler<'a, T>(
    t: &T,
    state: AssertUnwindSafe<State>,
) -> Box<Future<Item = Response<Body>, Error = CompatError> + Send + 'a>
where
    T: NewHandler + 'a,
{
    let res = catch_unwind(move || {
        // Hyper doesn't allow us to present an affine-typed `Handler` interface directly. We have
        // to emulate the promise given by hyper's documentation, by creating a `Handler` value and
        // immediately consuming it.
        t.new_handler()
            .into_future()
            .map_err(|e| failure::Error::from(e).compat())
            .and_then(move |handler| {
                let AssertUnwindSafe(state) = state;

                handler.handle(state).then(move |result| match result {
                    Ok((_state, res)) => future::ok(res),
                    Err((state, err)) => finalize_error_response(state, err),
                })
            })
    });

    if let Ok(f) = res {
        return Box::new(
            UnwindSafeFuture::new(f)
                .catch_unwind()
                .then(finalize_catch_unwind_response), // must be Future<Item = impl Payload>
        );
    }

    Box::new(finalize_panic_response())
}

fn finalize_error_response(
    state: State,
    err: HandlerError,
) -> FutureResult<Response<Body>, CompatError> {
    {
        // HandlerError::cause() is far more interesting for logging, but the
        // API doesn't guarantee its presence (even though it always is).
        let err_description = err
            .cause()
            .map(Error::description)
            .unwrap_or(err.description());

        error!(
            "[ERROR][{}][Error: {}]",
            request_id(&state),
            err_description
        );
    }
    future::ok(err.into_response(&state))
}

fn finalize_panic_response() -> FutureResult<Response<Body>, CompatError> {
    error!("[PANIC][A panic occurred while invoking the handler]");

    future::ok(
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::default())
            .unwrap(),
    )
}

fn finalize_catch_unwind_response(
    result: Result<Result<Response<Body>, CompatError>, Box<Any + Send>>,
) -> FutureResult<Response<Body>, CompatError> {
    let response = result
        .unwrap_or_else(|_| {
            let e = io::Error::new(
                io::ErrorKind::Other,
                "Attempting to poll the future caused a panic",
            );

            Err(failure::Error::from(e).compat())
        }).unwrap_or_else(|_| {
            error!("[PANIC][A panic occurred while polling the future]");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::default())
                .unwrap()
        });

    future::ok(response)
}

/// Wraps a future to ensure that a panic does not escape and terminate the event loop.
enum UnwindSafeFuture<F>
where
    F: Future<Error = CompatError> + Send,
{
    /// The future is available for polling.
    Available(AssertUnwindSafe<F>),

    /// The future has been poisoned because a previous call to `poll` caused a panic.
    Poisoned,
}

impl<F> Future for UnwindSafeFuture<F>
where
    F: Future<Error = CompatError> + Send,
{
    type Item = F::Item;
    type Error = CompatError;

    fn poll(&mut self) -> Result<Async<Self::Item>, CompatError> {
        // Mark as poisoned in case `f.poll()` panics below.
        match mem::replace(self, UnwindSafeFuture::Poisoned) {
            UnwindSafeFuture::Available(mut f) => {
                let r = f.poll();
                // Replace with the original value again, now that the potential panic has not
                // occurred. This allows for a poll to occur next time.
                *self = UnwindSafeFuture::Available(f);
                r
            }
            UnwindSafeFuture::Poisoned => {
                let e = io::Error::new(
                    io::ErrorKind::Other,
                    "Poisoned future due to previous panic",
                );

                Err(failure::Error::from(e).compat())
            }
        }
    }
}

impl<F> UnwindSafeFuture<F>
where
    F: Future<Error = CompatError> + Send,
{
    fn new(f: F) -> UnwindSafeFuture<F> {
        UnwindSafeFuture::Available(AssertUnwindSafe(f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;

    use hyper::{HeaderMap, Method, StatusCode};

    use handler::{HandlerFuture, IntoHandlerError};
    use helpers::http::response::create_empty_response;
    use state::set_request_id;

    #[test]
    fn success() {
        let new_handler = || {
            Ok(|state| {
                let res = create_empty_response(&state, StatusCode::ACCEPTED);
                (state, res)
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn async_success_repeat_poll() {
        let new_handler = || {
            Ok(|state| {
                let f = future::lazy(move || {
                    let res = create_empty_response(&state, StatusCode::ACCEPTED);
                    future::ok((state, res))
                });

                let f = future::lazy(move || f);
                let f = future::lazy(move || f);
                let f = future::lazy(move || f);

                Box::new(f) as Box<HandlerFuture>
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn error() {
        let new_handler = || {
            Ok(|state| {
                Box::new(future::err((
                    state,
                    io::Error::last_os_error().into_handler_error(),
                ))) as Box<HandlerFuture>
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn panic() {
        let new_handler = || {
            Ok(|_| {
                let val: Option<Box<HandlerFuture>> = None;
                val.expect("test panic")
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn async_panic() {
        let new_handler = || {
            Ok(|_| {
                let val: Option<Box<HandlerFuture>> = None;
                Box::new(future::lazy(move || val.expect("test panic"))) as Box<HandlerFuture>
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn async_panic_repeat_poll() {
        let new_handler = || {
            Ok(|_| {
                let val: Option<Box<HandlerFuture>> = None;
                let f = future::lazy(move || val.expect("test panic"));
                let f = future::lazy(move || f);
                let f = future::lazy(move || f);
                let f = future::lazy(move || f);
                Box::new(f) as Box<HandlerFuture>
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
