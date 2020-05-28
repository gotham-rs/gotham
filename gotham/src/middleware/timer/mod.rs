//! Request timing middleware, used to measure response times of requests.
use crate::handler::HandlerFuture;
use crate::helpers::http::header::X_RUNTIME_DURATION;
use crate::helpers::timing::Timer;
use crate::middleware::{Middleware, NewMiddleware};
use crate::state::State;
use futures::prelude::*;
use std::pin::Pin;

/// Middleware binding to attach request execution times inside headers.
///
/// This can be used to easily measure request time from outside the
/// application, via the `x-runtime-duration` header in the response.
#[derive(Clone)]
pub struct RequestTimer;

/// `Middleware` trait implementation.
impl Middleware for RequestTimer {
    /// Attaches the request execution time to the response headers.
    fn call<Chain>(self, state: State, chain: Chain) -> Pin<Box<HandlerFuture>>
    where
        Chain: FnOnce(State) -> Pin<Box<HandlerFuture>>,
    {
        // start the timer
        let timer = Timer::new();

        // execute the chain and attach the time on complete
        let f = chain(state).and_then(move |(state, mut response)| {
            // attach the formatted header
            response.headers_mut().insert(
                X_RUNTIME_DURATION,
                timer.elapsed().to_string().parse().unwrap(),
            );

            future::ok((state, response))
        });

        f.boxed()
    }
}

/// `NewMiddleware` trait implementation.
impl NewMiddleware for RequestTimer {
    type Instance = Self;

    /// Clones the current middleware to a new instance.
    fn new_middleware(&self) -> anyhow::Result<Self::Instance> {
        Ok(self.clone())
    }
}
