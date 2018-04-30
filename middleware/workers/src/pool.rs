use futures::{Async, Future, IntoFuture, Poll};
use futures_cpupool::{CpuFuture, CpuPool};

use gotham::state::{request_id, FromState, State};

#[derive(StateData)]
pub(crate) struct WorkersPool {
    pub pool: CpuPool,
}

// TODO: This can be removed when we can return `impl Future<..>` from `run_in_pool`.
pub(crate) struct CpuFutureWithState<T, E> {
    future: CpuFuture<T, E>,
    state: Option<State>,
}

impl<T, E> Future for CpuFutureWithState<T, E>
where
    CpuFuture<T, E>: Future<Item = T, Error = E>,
{
    type Item = (State, T);
    type Error = (State, E);

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.future.poll() {
            Ok(Async::Ready(t)) => Ok(Async::Ready((self.take_state(), t))),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err((self.take_state(), e)),
        }
    }
}

impl<T, E> CpuFutureWithState<T, E> {
    fn take_state(&mut self) -> State {
        self.state.take().expect(
            "State value already gone; \
             Future::poll on CpuFutureWithState was called too many times",
        )
    }
}

pub(crate) fn run_in_thread_pool<F, R, E, T>(state: State, f: F) -> CpuFutureWithState<T, E>
where
    F: FnOnce() -> R + Send + 'static,
    R: IntoFuture<Item = T, Error = E> + 'static,
    R::Future: Send + 'static,
    E: Send + 'static,
    T: Send + 'static,
{
    let future = match WorkersPool::try_borrow_from(&state) {
        Some(pool) => pool.pool.spawn_fn(f),
        None => {
            error!(
                "[{}] unable to execute worker, no WorkersPool available in state \
                 (workers middleware misconfigured?)",
                request_id(&state)
            );
            panic!("unable to execute worker, no WorkersPool available in state");
        }
    };

    CpuFutureWithState {
        future,
        state: Some(state),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;
    use hyper::StatusCode;
    use gotham::handler::{HandlerFuture, IntoHandlerError};
    use gotham::http::response::create_response;
    use gotham::test::TestServer;

    #[test]
    fn run_in_thread_pool_tests() {
        fn handler(mut state: State) -> Box<HandlerFuture> {
            // Simulate the job of the middleware.
            state.put(WorkersPool {
                pool: CpuPool::new(1),
            });

            let f = run_in_thread_pool(state, || Ok(StatusCode::Accepted)).then(
                |r: Result<(State, StatusCode), (State, io::Error)>| match r {
                    Ok((state, t)) => {
                        let response = create_response(&state, t, None);
                        Ok((state, response))
                    }
                    Err((state, e)) => Err((state, e.into_handler_error())),
                },
            );

            Box::new(f)
        }

        let test_server = TestServer::new(|| Ok(handler)).unwrap();
        let client = test_server.client();
        let response = client.get("https://example.com/").perform().unwrap();
        assert_eq!(response.status(), StatusCode::Accepted);
    }
}
