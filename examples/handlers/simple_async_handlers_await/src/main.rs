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

use futures::compat::Future01CompatExt;
use futures::{FutureExt, TryFutureExt};
use legacy_futures::Future as LegacyFuture;

use std::time::{Duration, Instant};

use hyper::StatusCode;

use gotham::handler::{HandlerError, HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_response;
use gotham::router::builder::DefineSingleRoute;
use gotham::router::builder::{build_simple_router, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};

use tokio::timer::Delay;

type SleepFuture = Box<dyn LegacyFuture<Item = Vec<u8>, Error = HandlerError> + Send>;

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
///
/// This function returns a LegacyFuture (a Future from the 0.1.x branch of
/// the `futures` crate, rather than a std::future::Future that we can .await).
/// This is partly to keep it the same as the simple_async_handlers example,
/// and partly to show you how to use the .compat() combinators (because you
/// will probably be using them a lot while the ecosystem stabilises).
/// 
/// For a better explanation of .compat(), please read this blog post:
/// https://rust-lang-nursery.github.io/futures-rs/blog/2019/04/18/compatibility-layer.html
fn sleep(seconds: u64) -> SleepFuture {
    let when = Instant::now() + get_duration(seconds);
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
    let async_block_future = async move {
        let seconds = QueryStringExtractor::take_from(&mut state).seconds;
        println!("sleep for {} seconds once: starting", seconds);
        // Here, we call the sleep function and turn its old-style future into
        // a new-style future. Note that this step doesn't block: it just sets
        // up the timer so that we can use it later.
        let sleep_future = sleep(seconds).compat();

        // Here is where the serious sleeping happens. We yield execution of
        // this block until sleep_future is resolved.
        // The Ok("slept for x seconds") value is stored in result.
        let result = sleep_future.await;

        // Here, we convert the result from `sleep()` into the form that Gotham
        // expects: `state` is owned by this block so we need to return it.
        // We also convert any errors that we have into the form that Hyper
        // expects, using the helper from IntoHandlerError.
        match result {
            Ok(data) => {
                let res = create_response(&state, StatusCode::OK, mime::TEXT_PLAIN, data);
                println!("sleep for {} seconds once: finished", seconds);
                Ok((state, res))
            }
            Err(err) => Err((state, err.into_handler_error())),
        }
    };
    // Here, we convert the new-style future produced by the async block into
    // an old-style future that gotham can understand. There are a couple of
    // layers of boxes, which is a bit sad, but these will go away once the
    // ecosystem settles and we can return std::future::Future from Handler
    // functions. Think of it as a temporary wart.
    Box::new(async_block_future.boxed().compat())
}

/// It calls sleep(1) as many times as needed to make the requested duration.
///
/// Notice how much easier it is to read than the version in
/// `simple_async_handlers`.
fn loop_handler(mut state: State) -> Box<HandlerFuture> {
    let async_block_future = async move {
        let seconds = QueryStringExtractor::take_from(&mut state).seconds;
        println!("sleep for one second {} times: starting", seconds);

        // We can't use the ? operator in the outermost async block, because we
        // need to need to return ownership of the State object back to gotham.
        // I quite like using ?, so I often find myself writing `async {}.await`
        // to get around this problem when I'm feeling lazy. Think of this
        // self-awaiting-block as a bit like a try block.
        //
        // In real code, you probably shouldn't be writing business logic in
        // your Handler functions anyway. Instead, you should be
        // unpacking everything you need from State in the Handler function
        // and then calling your business logic with only the dependencies that
        // they need. That way your business logic can use new-style futures
        // and ? as much as it likes, and you will only need to update your
        // handler functions (which don't contain any business logic) when you
        // upgrade your gotham.
        let result = async {
            // The code within this block reads exactly like syncronous code.
            // This is the style that you should aim to write your business
            // logic in.
            let mut accumulator = Vec::new();
            for _ in 0..seconds {
                let body = sleep(1).compat().await?;
                accumulator.extend(body)
            }
            // ? does type coercion for us, so we need to use a turbofish to
            // tell the compiler that we have a HandlerError. See this section
            // of the rust async book for more details:
            // https://rust-lang.github.io/async-book/07_workarounds/03_err_in_async_blocks.html
            Ok::<_, HandlerError>(accumulator)
        }
            .await;

        // This bit is the same boilerplate as the bit in the first example.
        // Nothing to see here.
        match result {
            Ok(data) => {
                let res = create_response(&state, StatusCode::OK, mime::TEXT_PLAIN, data);
                println!("sleep for one second {} times: finished", seconds);
                Ok((state, res))
            }
            Err(err) => Err((state, err.into_handler_error())),
        }
    };
    Box::new(async_block_future.boxed().compat())
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
