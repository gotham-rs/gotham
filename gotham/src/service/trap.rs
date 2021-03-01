//! Defines functionality for processing a request and trapping errors and panics in response
//! generation.

use std::panic::catch_unwind;
use std::panic::{AssertUnwindSafe, UnwindSafe};

use futures::prelude::*;

use hyper::{Body, Response, StatusCode};
use log::error;

use crate::handler::{Handler, HandlerError, IntoResponse, NewHandler};
use crate::state::{request_id, State};

async fn handle<H>(
    handler: H,
    state: AssertUnwindSafe<State>,
) -> Result<(State, Response<Body>), (State, HandlerError)>
where
    H: Handler,
{
    let AssertUnwindSafe(state) = state;
    handler.handle(state).await
}

/// Instantiates a `Handler` from the given `NewHandler`, and invokes it with the request. If a
/// panic occurs from `NewHandler::new_handler` or `Handler::handle`, it is trapped and will result
/// in a `500 Internal Server Error` response.
///
/// Timing information is recorded and logged, except in the case of a panic where the timer is
/// moved and cannot be recovered.
pub async fn call_handler<T>(t: T, state: AssertUnwindSafe<State>) -> anyhow::Result<Response<Body>>
where
    T: NewHandler + Send + UnwindSafe,
{
    match catch_unwind(move || t.new_handler()) {
        Ok(handler) => {
            let unwind_result = AssertUnwindSafe(handle(handler?, state))
                .catch_unwind()
                .await;
            let result = match unwind_result {
                Ok(result) => result.map(|(_, res)| res),
                Err(_) => Ok(finalize_panic_response()),
            };
            Ok(match result {
                Ok(res) => res,
                Err((state, err)) => finalize_error_response(state, err),
            })
        }
        // Error while creating the handler from NewHandler
        Err(_) => Ok(finalize_panic_response()),
    }
}

fn finalize_error_response(state: State, err: HandlerError) -> Response<Body> {
    error!("[ERROR][{}][Error: {:?}]", request_id(&state), err);

    err.into_response(&state)
}

fn finalize_panic_response() -> Response<Body> {
    error!("[PANIC][A panic occurred while invoking the handler]");

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::default())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;
    use std::pin::Pin;

    use hyper::{HeaderMap, Method, StatusCode};

    use crate::handler::HandlerFuture;
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
        let new_handler =
            || Ok(|state| future::err((state, io::Error::last_os_error().into())).boxed());

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

            fn new_handler(&self) -> anyhow::Result<Self::Instance> {
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
        let r = call_handler(new_handler, AssertUnwindSafe(state));
        let response = futures::executor::block_on(r).unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
