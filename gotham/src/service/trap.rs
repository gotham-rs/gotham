//! Defines functionality for processing a request and trapping errors and panics in response
//! generation.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::error::Error;
use std::any::Any;
use std::{io, mem};

use hyper::{self, Response, StatusCode};
use futures::Async;
use futures::future::{self, Future, FutureResult};

use handler::{Handler, HandlerError, IntoResponse, NewHandler};
use service::timing::Timer;
use state::{request_id, State};

pub(super) fn call_handler<T>(
    t: &T,
    state: AssertUnwindSafe<State>,
) -> Box<Future<Item = Response, Error = hyper::Error>>
where
    T: NewHandler,
{
    let timer = Timer::new();

    let res = catch_unwind(move || {
        type ResponseFuture = Future<Item = Response, Error = hyper::Error>;

        // Hyper doesn't allow us to present an affine-typed `Handler` interface directly. We have
        // to emulate the promise given by hyper's documentation, by creating a `Handler` value and
        // immediately consuming it.
        match t.new_handler() {
            Ok(handler) => {
                let AssertUnwindSafe(state) = state;

                let f = handler.handle(state).then(move |result| match result {
                    Ok((state, res)) => finalize_success_response(timer, state, res),
                    Err((state, err)) => finalize_error_response(timer, state, err),
                });

                Box::new(f) as Box<ResponseFuture>
            }
            Err(e) => Box::new(future::err(e.into())) as Box<ResponseFuture>,
        }
    });

    match res {
        Ok(f) => Box::new(
            UnwindSafeFuture::new(f)
                .catch_unwind()
                .then(finalize_catch_unwind_response),
        ),
        Err(_) => Box::new(finalize_panic_response(timer)),
    }
}

fn finalize_success_response(
    timer: Timer,
    state: State,
    response: Response,
) -> FutureResult<Response, hyper::Error> {
    let timing = timer.elapsed(&state);

    info!(
        "[RESPONSE][{}][{}][{}][{}]",
        request_id(&state),
        response.version(),
        response.status(),
        timing
    );

    future::ok(timing.add_to_response(response))
}

fn finalize_error_response(
    timer: Timer,
    state: State,
    err: HandlerError,
) -> FutureResult<Response, hyper::Error> {
    let timing = timer.elapsed(&state);

    {
        // HandlerError::cause() is far more interesting for logging, but the
        // API doesn't guarantee its presence (even though it always is).
        let err_description = err.cause()
            .map(Error::description)
            .unwrap_or(err.description());

        error!(
            "[ERROR][{}][Error: {}][{}]",
            request_id(&state),
            err_description,
            timing
        );
    }

    future::ok(err.into_response(&state))
}

fn finalize_panic_response(timer: Timer) -> FutureResult<Response, hyper::Error> {
    let timing = timer.elapsed_no_logging();

    error!(
        "[PANIC][A panic occurred while invoking the handler][{}]",
        timing
    );

    future::ok(Response::new().with_status(StatusCode::InternalServerError))
}

fn finalize_catch_unwind_response(
    result: Result<Result<Response, hyper::Error>, Box<Any + Send>>,
) -> FutureResult<Response, hyper::Error> {
    let response = result
        .unwrap_or_else(|_| {
            let e = io::Error::new(
                io::ErrorKind::Other,
                "Attempting to poll the future caused a panic",
            );

            Err(hyper::Error::Io(e))
        })
        .unwrap_or_else(|_| {
            error!("[PANIC][A panic occurred while polling the future]");
            Response::new().with_status(StatusCode::InternalServerError)
        });

    future::ok(response)
}

enum UnwindSafeFuture<F>
where
    F: Future<Error = hyper::Error>,
{
    Available(AssertUnwindSafe<F>),
    Poisoned,
}

impl<F> Future for UnwindSafeFuture<F>
where
    F: Future<Error = hyper::Error>,
{
    type Item = F::Item;
    type Error = hyper::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, hyper::Error> {
        match mem::replace(self, UnwindSafeFuture::Poisoned) {
            UnwindSafeFuture::Available(mut f) => {
                let r = f.poll();
                *self = UnwindSafeFuture::Available(f);
                r
            }
            UnwindSafeFuture::Poisoned => {
                let e = io::Error::new(
                    io::ErrorKind::Other,
                    "Poisoned future due to previous panic",
                );

                Err(hyper::Error::Io(e))
            }
        }
    }
}

impl<F> UnwindSafeFuture<F>
where
    F: Future<Error = hyper::Error>,
{
    fn new(f: F) -> UnwindSafeFuture<F> {
        UnwindSafeFuture::Available(AssertUnwindSafe(f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;

    use hyper::{Headers, StatusCode};

    use http::response::create_response;
    use state::set_request_id;
    use handler::{HandlerFuture, IntoHandlerError};

    #[test]
    fn success() {
        let new_handler = || {
            Ok(|state| {
                let res = create_response(&state, StatusCode::Accepted, None);
                (state, res)
            })
        };

        let mut state = State::new();
        state.put(Headers::new());
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::Accepted);
    }

    #[test]
    fn async_success_repeat_poll() {
        let new_handler = || {
            Ok(|state| {
                let f = future::lazy(move || {
                    let res = create_response(&state, StatusCode::Accepted, None);
                    future::ok((state, res))
                });

                let f = future::lazy(move || f);
                let f = future::lazy(move || f);
                let f = future::lazy(move || f);

                Box::new(f) as Box<HandlerFuture>
            })
        };

        let mut state = State::new();
        state.put(Headers::new());
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::Accepted);
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
        state.put(Headers::new());
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::InternalServerError);
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
        state.put(Headers::new());
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::InternalServerError);
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
        state.put(Headers::new());
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::InternalServerError);
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
        state.put(Headers::new());
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = r.wait().unwrap();
        assert_eq!(response.status(), StatusCode::InternalServerError);
    }
}
