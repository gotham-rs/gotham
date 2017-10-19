//! Defines functionality for processing a request and trapping errors and panics in response
//! generation.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::error::Error;

use hyper::{self, Response, StatusCode};
use futures::future::{self, Future, FutureResult};

use handler::{NewHandler, Handler, HandlerError, IntoResponse};
use handler::timing::Timer;
use state::{State, request_id};

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
        Ok(f) => f,
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
        let err_description = err.cause().map(Error::description).unwrap_or(
            err.description(),
        );

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
