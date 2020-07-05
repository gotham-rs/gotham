//! A basic example showing the request components

use futures::prelude::*;
use std::pin::Pin;
use std::time::{Duration, Instant};

use gotham::hyper::StatusCode;

use gotham::handler::{HandlerError, IntoResponse};
use gotham::router::builder::DefineSingleRoute;
use gotham::router::builder::{build_simple_router, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};
use gotham_derive::{StateData, StaticResponseExtender};
use serde_derive::Deserialize;

use tokio::time::delay_until;

type SleepFuture = Pin<Box<dyn Future<Output = Vec<u8>> + Send>>;

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct QueryStringExtractor {
    seconds: u64,
}

/// Sneaky hack to make tests take less time. Nothing to see here ;-).
#[cfg(not(test))]
fn get_duration(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}
#[cfg(test)]
fn get_duration(seconds: u64) -> Duration {
    Duration::from_millis(seconds)
}
/// All this function does is return a future that resolves after a number of
/// seconds, with a Vec<u8> that tells you how long it slept for.
///
/// Note that it does not block the thread from handling other requests, because
/// it returns a `Future`, which will be managed by the tokio reactor, and
/// called back once the timeout has expired.
///
/// Vec<u8> is chosen because it is one of the things that you need to resolve
/// a HandlerFuture and respond to a request.
///
/// Most things that you call to access remote services (e.g databases and
/// web apis) can be coerced into returning futures that yield useful data,
/// so the patterns that you learn in this example should be applicable to
/// real world problems.
fn sleep(seconds: u64) -> SleepFuture {
    let when = Instant::now() + get_duration(seconds);
    let delay = delay_until(when.into()).map(move |_| {
        format!("slept for {} seconds\n", seconds)
            .as_bytes()
            .to_vec()
    });

    delay.boxed()
}

/// This handler sleeps for the requested number of seconds, using the `sleep()`
/// helper method, above.
async fn sleep_handler(state: &mut State) -> Result<impl IntoResponse, HandlerError> {
    let seconds = QueryStringExtractor::borrow_from(state).seconds;
    println!("sleep for {} seconds once: starting", seconds);
    // Here, we call the sleep function. Note that this step doesn't block:
    // it just sets up the timer so that we can use it later.
    let sleep_future = sleep(seconds);

    // Here is where the serious sleeping happens. We yield execution of
    // this block until sleep_future is resolved.
    // The "slept for x seconds" value is stored in data.
    let data = sleep_future.await;

    // We return a `Result<impl IntoResponse, HandlerError>` directly
    // where the success type can be anything implementing `IntoResponse`
    // (including a `Response<Body>`)
    println!("sleep for {} seconds once: finished", seconds);
    Ok((StatusCode::OK, mime::TEXT_PLAIN, data))
}

/// It calls sleep(1) as many times as needed to make the requested duration.
///
/// Notice how much easier it is to read than the version in
/// `simple_async_handlers`.
async fn loop_handler(state: &mut State) -> Result<impl IntoResponse, HandlerError> {
    let seconds = QueryStringExtractor::borrow_from(state).seconds;
    println!("sleep for one second {} times: starting", seconds);

    // The code within this block reads exactly like syncronous code.
    // This is the style that you should aim to write your business
    // logic in.
    let mut accumulator = Vec::new();
    for _ in 0..seconds {
        let body = sleep(1).await;
        accumulator.extend(body)
    }

    println!("sleep for one second {} times: finished", seconds);
    Ok((StatusCode::OK, mime::TEXT_PLAIN, accumulator))
}

/// Create a `Router`.
fn router() -> Router {
    build_simple_router(|route| {
        route
            .get("/sleep")
            .with_query_string_extractor::<QueryStringExtractor>()
            .to_async_borrowing(sleep_handler);
        route
            .get("/loop")
            .with_query_string_extractor::<QueryStringExtractor>()
            .to_async_borrowing(loop_handler);
    })
}

/// Start a server and use a `Router` to dispatch requests.
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}

#[cfg(test)]
mod tests {
    use gotham::test::TestServer;

    use super::*;

    fn assert_returns_ok(url_str: &str, expected_response: &str) {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server.client().get(url_str).perform().unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            &String::from_utf8(response.read_body().unwrap()).unwrap(),
            expected_response
        );
    }

    #[test]
    fn sleep_says_how_long_it_slept_for() {
        assert_returns_ok("http://localhost/sleep?seconds=2", "slept for 2 seconds\n");
    }

    #[test]
    fn loop_breaks_the_time_into_one_second_sleeps() {
        assert_returns_ok(
            "http://localhost/loop?seconds=2",
            "slept for 1 seconds\nslept for 1 seconds\n",
        );
    }
}
