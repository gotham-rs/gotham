//! Defines functionality for processing a request and trapping errors and panics in response
//! generation.

use std::error::Error;
use std::panic::catch_unwind;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;

use failure;
use futures::prelude::*;

use hyper::{Body, Response, StatusCode};
use log::error;

use crate::handler::{Handler, HandlerError, IntoResponse, NewHandler};
use crate::state::{request_id, State};

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
) -> Pin<Box<dyn Future<Output = Result<Response<Body>, CompatError>> + Send + 'a>>
where
    T: NewHandler + 'a,
{
    // Need to consume the NewHandler eagerly (vs lazy) since its borrowed
    // The rest of the processing occurs in a future
    match catch_unwind(move || t.new_handler()) {
        Ok(handler) => {
            let res = future::ready(handler)
                .map_err(failure::Error::compat)
                .and_then(move |handler| {
                    let AssertUnwindSafe(state) = state;

                    handler.handle(state).then(move |result| match result {
                        Ok((_state, res)) => {
                            future::ok::<_, CompatError>(res).err_into().left_future()
                        }
                        Err((state, err)) => finalize_error_response(state, err)
                            .err_into()
                            .right_future(),
                    })
                });

            AssertUnwindSafe(res)
                .catch_unwind()
                .then(|unwind_result| match unwind_result {
                    Ok(result) => finalize_catch_unwind_response(result).left_future(),
                    Err(_) => finalize_panic_response().right_future(),
                })
                .left_future()
        }
        // Pannicked creating the handler from NewHandler
        Err(_) => finalize_panic_response().right_future(),
    }
    .boxed()
}

fn finalize_error_response(
    state: State,
    err: HandlerError,
) -> impl Future<Output = Result<Response<Body>, CompatError>> {
    {
        // HandlerError::source() is far more interesting for logging, but the
        // API doesn't guarantee its presence (even though it always is).
        let err_description = err
            .source()
            .map(Error::to_string)
            .unwrap_or_else(|| err.to_string());

        error!(
            "[ERROR][{}][Error: {}]",
            request_id(&state),
            err_description
        );
    }
    future::ok(err.into_response(&state))
}

fn finalize_panic_response() -> impl Future<Output = Result<Response<Body>, CompatError>> {
    error!("[PANIC][A panic occurred while invoking the handler]");

    future::ok(
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::default())
            .unwrap(),
    )
}

fn finalize_catch_unwind_response(
    result: Result<Response<Body>, CompatError>,
) -> impl Future<Output = Result<Response<Body>, CompatError>> {
    let response = result.unwrap_or_else(|_| {
        error!("[PANIC][A panic occurred while polling the future]");
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::default())
            .unwrap()
    });

    future::ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;

    use hyper::{HeaderMap, Method, StatusCode};

    use crate::error::Result;
    use crate::handler::{HandlerFuture, IntoHandlerError};
    use crate::helpers::http::response::create_empty_response;
    use crate::state::set_request_id;

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
        let response = futures::executor::block_on(r).unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn async_success_repeat_poll() {
        let new_handler = || {
            Ok(|state| {
                let f = future::lazy(move |_| {
                    let res = create_empty_response(&state, StatusCode::ACCEPTED);
                    Ok((state, res))
                });

                let f = f.map(|v| v);
                let f = f.map(|v| v);
                let f = f.map(|v| v);

                f.boxed()
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = futures::executor::block_on(r).unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn error() {
        let new_handler = || {
            Ok(|state| {
                future::err((state, io::Error::last_os_error().into_handler_error())).boxed()
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = futures::executor::block_on(r).unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn panic() {
        let new_handler = || {
            Ok(|_| {
                let val: Option<Pin<Box<HandlerFuture>>> = None;
                val.expect("test panic")
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = futures::executor::block_on(r).unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn async_panic() {
        let new_handler = || Ok(|_| future::lazy(move |_| panic!("test panic")).boxed());

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = futures::executor::block_on(r).unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn async_panic_repeat_poll() {
        let new_handler = || {
            Ok(|_| {
                let f = future::lazy(move |_| panic!("test panic"));

                let f = f.map(|v| v);
                let f = f.map(|v| v);
                let f = f.map(|v| v);

                f.boxed()
            })
        };

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = futures::executor::block_on(r).unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn new_handler_panic() {
        struct PanicNewHandler;
        impl NewHandler for PanicNewHandler {
            type Instance = Self;

            fn new_handler(&self) -> Result<Self::Instance> {
                panic!("Pannicked creating a new handler");
            }
        }

        impl Handler for PanicNewHandler {
            fn handle(self, _state: State) -> Pin<Box<HandlerFuture>> {
                unreachable!();
            }
        }

        let mut state = State::new();
        state.put(HeaderMap::new());
        state.put(Method::GET);
        set_request_id(&mut state);

        let new_handler = PanicNewHandler {};
        let r = call_handler(&new_handler, AssertUnwindSafe(state));
        let response = futures::executor::block_on(r).unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
