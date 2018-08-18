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
extern crate tokio_core;

use futures::{future, stream, Future, Stream};

use hyper::StatusCode;
#[cfg(not(test))]
use hyper::{Client, Uri};

use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_response;
use gotham::router::builder::DefineSingleRoute;
use gotham::router::builder::{build_simple_router, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};

use tokio_core::reactor::Handle;

type ResponseContentFuture = Box<Future<Item = Vec<u8>, Error = hyper::Error>>;

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct QueryStringExtractor {
    length: i8,
}

/// This helper function does an HTTP GET, and returns the body as a `Vec`, so that it can be passed
/// into `create_response` easily, and the example handlers can focus on the business logic.
/// You may notice that the body collecting looks very similar to the POST example in
/// `examples/handlers/request_data`.
/// Notice also that this function needs a reference to a `Handle`. This is a handle to the event
/// loop that the web request is being handled on. There are plans to stop needing it in later
/// versions of tokio, so for now you can treat it as a bit of annoying bookkeeping and get on with
/// your life.
#[cfg(not(test))]
fn http_get(handle: &Handle, url_str: &str) -> ResponseContentFuture {
    let client = Client::new(handle);
    let url: Uri = url_str.parse().unwrap();
    let f = client.get(url).and_then(|response| {
        response
            .body()
            .concat2()
            .and_then(|full_body| Ok(full_body.to_vec()))
    });

    Box::new(f)
}

/// The other advantage of using a helper function is that you can easily patch it out for testing.
/// You typically don't want to rely on external http services for your unit tests, because they
/// will fail unexpectedly, and cause you to stop believing your unit tests when they fail.
/// The subject of patching/mocking things out for test purposes is a big one, and this is just a
/// toy example, so we just return success.
#[cfg(test)]
fn http_get(_handle: &Handle, _url_str: &str) -> ResponseContentFuture {
    // We make the test version return something different from what a real view would, to make
    // it easier to spot in the tests.
    Box::new(future::ok(b"y".to_vec()))
}

/// Now we come to the business end of the example.
///
/// This is a contrived example, that calls itself recursively over http, to produce a string of
/// 'z's of length `length`. This is not something that you would want to do in real life.
/// That said, the techniques used should be transferrable to any code that makes calls
/// to external services, and wants to do so without blocking other Handlers from running on the
/// same thread while it's waiting for a response.
///
/// Something to note about this example is that because we're accumulating results from one future
/// to the next, our code drifts to the right. If you are using nightly, you can avoid this by
/// using something like:
/// https://github.com/alexcrichton/futures-await
fn series_handler(mut state: State) -> Box<HandlerFuture> {
    let length = QueryStringExtractor::take_from(&mut state).length;
    println!("series length: {} starting", length);

    // We have two base cases (`n = 0` and `n = 1`) and a block that recurses.
    // Note that we pick a signature for our future that makes lives easier for our business logic,
    // and then convert it into a `Box<HandlerFuture>` in the end.
    let data_future: ResponseContentFuture = if length == 0 {
        Box::new(future::ok(Vec::new()))
    } else if length == 1 {
        Box::new(future::ok(b"z".to_vec()))
    } else {
        // As discussed in `http_get`, we need to pass around a handle to the event loop
        // we're running on. This is just book-keeping really, but here's how to do it:
        let handle = Handle::borrow_from(&state).clone();

        // These are the two URLs we're going to request. We're just splitting the length into
        // two roughly equal parts and calling ourselves. In a real application, these might
        // be external web apis or internal microservices.
        let url_a = format!("http://127.0.0.1:7878/series?length={}", length / 2);
        let url_b = format!(
            "http://127.0.0.1:7878/series?length={}",
            length / 2 + length % 2
        );

        // Here, we get the first URL, and then get the second URL, and then concatenate the
        // two together. Notice that we have to move body_a into the second closure, and so our
        // code drifts to the right.
        let f = http_get(&handle, &url_a).and_then(move |mut body_a| {
            http_get(&handle, &url_b).and_then(move |body_b| {
                body_a.extend(body_b);
                Ok(body_a)
            })
        });

        Box::new(f)
    };

    // Here, we convert the future from our handler into the form that Gotham expects.
    // All we do is move `state` in, to return it, and convert any errors that we have.
    Box::new(data_future.then(move |result| match result {
        Ok(data) => {
            let res = create_response(&state, StatusCode::Ok, Some((data, mime::TEXT_PLAIN)));
            println!("series length: {} finished", length);
            Ok((state, res))
        }
        Err(err) => Err((state, err.into_handler_error())),
    }))
}

/// This example uses a `future::Stream` to implement a `for` loop. This example only has two urls
/// to call `http_get` on, but you can hopefully see how it is a useful pattern.
///
/// If any `http_get` call returns an error, then processing will stop, and the error will be
/// returned.
///
/// https://github.com/alexcrichton/futures-await has a more readable syntax for this as
/// well, if you are using nightly Rust.
fn loop_handler(mut state: State) -> Box<HandlerFuture> {
    let length = QueryStringExtractor::take_from(&mut state).length;
    println!("loop length: {} starting", length);

    // The structure is the same as `series_handler`, above.
    let data_future: ResponseContentFuture = if length == 0 {
        Box::new(future::ok(Vec::new()))
    } else if length == 1 {
        Box::new(future::ok(b"z".to_vec()))
    } else {
        let handle = Handle::borrow_from(&mut state).clone();

        let url_a = format!("http://127.0.0.1:7878/loop?length={}", length / 2);
        let url_b = format!(
            "http://127.0.0.1:7878/loop?length={}",
            length / 2 + length % 2
        );

        // Here, we create a stream that contains our two URLs, and call fold to loop over all URLs
        // and get the urls, concatenating the results into the accumulator (which starts off as the
        // empty `Vec`).
        let f =
            stream::iter_ok(vec![url_a, url_b]).fold(Vec::new(), move |mut accumulator, url| {
                // Do the http_get(), and append the result to the accumulator so that it can
                // be returned.
                http_get(&handle, &url).and_then(move |body| {
                    accumulator.extend(body);
                    Ok(accumulator)
                })
            });

        Box::new(f)
    };

    Box::new(data_future.then(move |result| match result {
        Ok(data) => {
            let res = create_response(&state, StatusCode::Ok, Some((data, mime::TEXT_PLAIN)));
            println!("loop length: {} finished", length);
            Ok((state, res))
        }
        Err(err) => Err((state, err.into_handler_error())),
    }))
}

/// This example does the same thing as `series_handler`, but doesn't wait for the first request
/// to return before starting the second. This approach is very tempting, but it is not recommended.
///
/// Problems with this approach include:
/// * If both requests fail then you will get the error from whichever one happened to fail first,
///   and the other error will be thrown on the floor.
/// * This approach tends to cause spikes in resource usage across the different parts of your
///   infrastructure, so a poorly written endpoint can amplify a single request into a storm,
///   without any back-pressure when things are going slowly/failing.
///
/// If you try to  `curl 'http://127.0.0.1:7878/parallel?length=100'` then you will find that
/// this example will cause the server to DoS itself with too many open tcp connections
/// ("Too many open files"), but http://127.0.0.1:7878/series?length=100'` works just fine.
///
/// A piece of advice from the Erlang community (but which applies to any language with lightweight
/// threads/promises) is:
///
///     "Use one parallel process to model each truly concurrent activity in the real world"
///
///     If there is a one-to-one mapping between the number of parallel processes and the number
///     of truly parallel activities in the real world, the program will be easy to understand.
///
///     -- http://www.erlang.se/doc/programming_rules.shtml#REF34191
///
/// In summary:
///     Don't do this at home kids. It is only included as a cautionary tale.
///
fn parallel_handler(mut state: State) -> Box<HandlerFuture> {
    let length = QueryStringExtractor::take_from(&mut state).length;
    println!("parallel length: {} starting", length);

    // The structure is the same as in `series_handler`, above.
    let data_future: ResponseContentFuture = if length == 0 {
        Box::new(future::ok(Vec::new()))
    } else if length == 1 {
        Box::new(future::ok(b"z".to_vec()))
    } else {
        let handle = Handle::borrow_from(&mut state);

        let url_a = format!("http://127.0.0.1:7878/parallel?length={}", length / 2);
        let url_b = format!(
            "http://127.0.0.1:7878/parallel?length={}",
            length / 2 + length % 2
        );

        // Here, we get both urls in parallel, and then join the futures together at the end.
        // See the docs of this function for a discussion of why this is a bad idea.
        let f1 = http_get(handle, &url_a);
        let f2 = http_get(handle, &url_b);

        Box::new(f1.join(f2).and_then(|(mut body_a, body_b)| {
            body_a.extend(body_b);
            Ok(body_a)
        }))
    };

    Box::new(data_future.then(move |result| match result {
        Ok(data) => {
            let res = create_response(&state, StatusCode::Ok, Some((data, mime::TEXT_PLAIN)));
            println!("parallel length: {} finished", length);
            Ok((state, res))
        }
        Err(err) => Err((state, err.into_handler_error())),
    }))
}

/// Create a `Router`.
fn router() -> Router {
    build_simple_router(|route| {
        route
            .get("/series")
            .with_query_string_extractor::<QueryStringExtractor>()
            .to(series_handler);
        ;
        route
            .get("/loop")
            .with_query_string_extractor::<QueryStringExtractor>()
            .to(loop_handler);
        ;
        route
            .get("/parallel")
            .with_query_string_extractor::<QueryStringExtractor>()
            .to(parallel_handler);
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

    fn assert_returns_ok(url_str: &str, expected_response: Vec<u8>) {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server.client().get(url_str).perform().unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(response.read_body().unwrap(), expected_response);
    }

    // Tests for `series`

    #[test]
    fn series_returns_zero_zs_if_length_is_zero() {
        assert_returns_ok("http://localhost/series?length=0", b"".to_vec());
    }

    #[test]
    fn series_returns_one_z_if_length_is_one() {
        assert_returns_ok("http://localhost/series?length=1", b"z".to_vec());
    }

    #[test]
    fn series_makes_two_http_gets_and_concatenates_the_responses_if_length_greater_than_one() {
        assert_returns_ok("http://localhost/series?length=2", b"yy".to_vec());
    }

    // Tests for `loop`

    #[test]
    fn loop_returns_zero_zs_if_length_is_zero() {
        assert_returns_ok("http://localhost/loop?length=0", b"".to_vec());
    }

    #[test]
    fn loop_returns_one_z_if_length_is_one() {
        assert_returns_ok("http://localhost/loop?length=1", b"z".to_vec());
    }

    #[test]
    fn loop_makes_two_http_gets_and_concatenates_the_responses_if_length_greater_than_one() {
        assert_returns_ok("http://localhost/loop?length=2", b"yy".to_vec());
    }

    // Tests for `parallel`

    #[test]
    fn parallel_returns_zero_zs_if_length_is_zero() {
        assert_returns_ok("http://localhost/parallel?length=0", b"".to_vec());
    }

    #[test]
    fn parallel_returns_one_z_if_length_is_one() {
        assert_returns_ok("http://localhost/parallel?length=1", b"z".to_vec());
    }

    #[test]
    fn parallel_makes_two_http_gets_and_concatenates_the_responses_if_length_greater_than_one() {
        assert_returns_ok("http://localhost/parallel?length=2", b"yy".to_vec());
    }
}
