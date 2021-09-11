//! An introduction to sharing state across handlers in a safe way.
//!
//! This example demonstrates a basic request counter which can be
//! used across server threads, and be used to track the number of
//! requests sent to the backend.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::mutex_atomic))]

use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::{single_middleware, single_pipeline};
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State, StateData};

use std::sync::{Arc, Mutex};

/// Request counting struct, used to track the number of requests made.
///
/// Due to being shared across many worker threads, the internal counter
/// is bound inside an `Arc` (to enable sharing) and a `Mutex` (to enable
/// modification from multiple threads safely).
///
/// This struct must implement `Clone` and `StateData` to be applicable
/// for use with the `StateMiddleware`, and be shared via `Middleware`.
#[derive(Clone, StateData)]
struct RequestCounter {
    inner: Arc<Mutex<usize>>,
}

/// Counter implementation.
impl RequestCounter {
    /// Creates a new request counter, setting the base state to `0`.
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(0)),
        }
    }

    /// Increments the internal counter state by `1`, and returns the
    /// new request counter as an atomic operation.
    fn incr(&self) -> usize {
        let mut w = self.inner.lock().unwrap();
        *w += 1;
        *w
    }
}

/// Basic `Handler` to say hello and return the current request count.
///
/// The request counter is shared via the state, so we can safely
/// borrow one from the provided state. As the counter uses locks
/// internally, we don't have to borrow a mutable reference either!
fn say_hello(state: State) -> (State, String) {
    let message = {
        // borrow a reference of the counter from the state
        let counter = RequestCounter::borrow_from(&state);

        // create our message, incrementing our request counter
        format!("Hello from request #{}!\n", counter.incr())
    };

    // return message
    (state, message)
}

/// Constructs a simple router on `/` to say hello, along with
/// the current request count.
fn router() -> Router {
    // create the counter to share across handlers
    let counter = RequestCounter::new();

    // create our state middleware to share the counter
    let middleware = StateMiddleware::new(counter);

    // create a middleware pipeline from our middleware
    let pipeline = single_middleware(middleware);

    // construct a basic chain from our pipeline
    let (chain, pipelines) = single_pipeline(pipeline);

    // build a router with the chain & pipeline
    build_router(chain, pipelines, |route| {
        route.get("/").to(say_hello);
    })
}

/// Start a server and call the `Handler` we've defined above
/// for each `Request` we receive.
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::hyper::StatusCode;
    use gotham::test::TestServer;

    #[test]
    fn receive_incrementing_hello_response() {
        let test_server = TestServer::new(router()).unwrap();

        for i in 1..6 {
            let response = test_server
                .client()
                .get("http://localhost")
                .perform()
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);

            let body = response.read_body().unwrap();
            let expc = format!("Hello from request #{}!\n", i);

            assert_eq!(&body[..], expc.as_bytes());
        }
    }
}
