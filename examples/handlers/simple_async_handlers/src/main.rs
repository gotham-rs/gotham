//! A basic example showing the request components

extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio;

use futures::{stream, Future, Stream};
use std::time::{Duration, Instant};

use hyper::StatusCode;

use gotham::handler::{HandlerError, HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_response;
use gotham::router::builder::DefineSingleRoute;
use gotham::router::builder::{build_simple_router, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};

use tokio::timer::Delay;

type SleepFuture = Box<Future<Item = Vec<u8>, Error = HandlerError> + Send>;

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct QueryStringExtractor {
    seconds: u64,
}

/// Sneaky hack to make tests take less time. Nothing to see here ;-).
#[cfg(not(test))]
fn get_duration(seconds: &u64) -> Duration {
    Duration::from_secs(seconds.to_owned())
}
#[cfg(test)]
fn get_duration(seconds: &u64) -> Duration {
    Duration::from_millis(seconds.to_owned())
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
    let when = Instant::now() + get_duration(&seconds);
    let delay = Delay::new(when)
        .map_err(|e| panic!("timer failed; err={:?}", e))
        .and_then(move |_| {
            Ok(format!("slept for {} seconds\n", seconds)
                .as_bytes()
                .to_vec())
        });

    Box::new(delay)
}

/// This handler sleeps for the requested number of seconds, using the `sleep()`
/// helper method, above.
fn sleep_handler(mut state: State) -> Box<HandlerFuture> {
    let seconds = QueryStringExtractor::take_from(&mut state).seconds;
    println!("sleep for {} seconds once: starting", seconds);

    // Here, we call our helper function that returns a future.
    let sleep_future = sleep(seconds.clone());

    // Here, we convert the future from `sleep()` into the form that Gotham expects.
    // We have to use .then() rather than .and_then() because we need to coerce both
    // the success and error cases into the right shape.
    // `state` is moved in, so that we can return it, and we convert any errors
    // that we have into the form that Hyper expects, using the helper from
    // IntoHandlerError.
    Box::new(sleep_future.then(move |result| match result {
        Ok(data) => {
            let res = create_response(&state, StatusCode::OK, (data, mime::TEXT_PLAIN));
            println!("sleep for {} seconds once: finished", seconds);
            Ok((state, res))
        }
        Err(err) => Err((state, err.into_handler_error())),
    }))
}

/// This example uses a `future::Stream` to implement a `for` loop. It calls sleep(1)
/// as many times as needed to make the requested duration.
///
/// https://github.com/alexcrichton/futures-await has a more readable syntax for
/// async for loops, if you are using nightly Rust.
fn loop_handler(mut state: State) -> Box<HandlerFuture> {
    let seconds = QueryStringExtractor::take_from(&mut state).seconds;
    println!("sleep for one second {} times: starting", seconds);

    // Here, we create a stream of Ok(_) that's as long as we need, and use fold
    // to loop over it asyncronously, accumulating the return values from sleep().
    let sleep_future: SleepFuture = Box::new(stream::iter_ok(0..seconds).fold(
        Vec::new(),
        move |mut accumulator, _| {
            // Do the sleep(), and append the result to the accumulator so that it can
            // be returned.
            sleep(1).and_then(move |body| {
                accumulator.extend(body);
                Ok(accumulator)
            })
        },
    ));

    // This bit is the same as the bit in the first example.
    Box::new(sleep_future.then(move |result| match result {
        Ok(data) => {
            let res = create_response(&state, StatusCode::OK, (data, mime::TEXT_PLAIN));
            println!("sleep for one second {} times: finished", seconds);
            Ok((state, res))
        }
        Err(err) => Err((state, err.into_handler_error())),
    }))
}

/// Create a `Router`.
fn router() -> Router {
    build_simple_router(|route| {
        route
            .get("/sleep")
            .with_query_string_extractor::<QueryStringExtractor>()
            .to(sleep_handler);
        ;
        route
            .get("/loop")
            .with_query_string_extractor::<QueryStringExtractor>()
            .to(loop_handler);
        ;
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
