//! A basic example showing the request components
#[macro_use]
extern crate gotham_derive;
#[macro_use]
extern crate serde_derive;

use futures::prelude::*;
use std::pin::Pin;
use std::time::{Duration, Instant};

use gotham::hyper::StatusCode;

use gotham::handler::HandlerFuture;
use gotham::helpers::http::response::create_response;
use gotham::router::builder::DefineSingleRoute;
use gotham::router::builder::{build_simple_router, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};

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
fn sleep_handler(mut state: State) -> Pin<Box<HandlerFuture>> {
    let seconds = QueryStringExtractor::take_from(&mut state).seconds;
    println!("sleep for {} seconds once: starting", seconds);

    // Here, we call our helper function that returns a future.
    let sleep_future = sleep(seconds);

    // Here, we convert the future from `sleep()` into the form that Gotham expects.
    // We have to use .then() rather than .and_then() because we need to coerce both
    // the success and error cases into the right shape.
    // `state` is moved in, so that we can return it, and we convert any errors
    // that we have into the form that Hyper expects, using the helper from
    // IntoHandlerError.
    sleep_future
        .map(move |data| {
            let res = create_response(&state, StatusCode::OK, mime::TEXT_PLAIN, data);
            println!("sleep for {} seconds once: finished", seconds);
            Ok((state, res))
        })
        .boxed()
}

/// This example uses a `future::Stream` to implement a `for` loop. It calls sleep(1)
/// as many times as needed to make the requested duration.
///
/// https://github.com/alexcrichton/futures-await has a more readable syntax for
/// async for loops, if you are using nightly Rust.
fn loop_handler(mut state: State) -> Pin<Box<HandlerFuture>> {
    let seconds = QueryStringExtractor::take_from(&mut state).seconds;
    println!("sleep for one second {} times: starting", seconds);

    // Here, we create a stream of Ok(_) that's as long as we need, and use fold
    // to loop over it asyncronously, accumulating the return values from sleep().
    let sleep_future: SleepFuture = futures::stream::iter(0..seconds)
        .fold(Vec::new(), move |mut accumulator, _| {
            // Do the sleep(), and append the result to the accumulator so that it can
            // be returned.
            sleep(1).map(move |body| {
                accumulator.extend(body);
                accumulator
            })
        })
        .boxed();

    // This bit is the same as the bit in the first example.
    sleep_future
        .map(move |data| {
            let res = create_response(&state, StatusCode::OK, mime::TEXT_PLAIN, data);
            println!("sleep for one second {} times: finished", seconds);
            Ok((state, res))
        })
        .boxed()
}

/// Create a `Router`.
fn router() -> Router {
    build_simple_router(|route| {
        route
            .get("/sleep")
            .with_query_string_extractor::<QueryStringExtractor>()
            .to(sleep_handler);
        route
            .get("/loop")
            .with_query_string_extractor::<QueryStringExtractor>()
            .to(loop_handler);
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
