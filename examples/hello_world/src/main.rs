//! A Hello World example application for working with [Gotham](https://gotham.rs).

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;

use hyper::{Response, StatusCode};

use gotham::http::response::create_response;
use gotham::start;
use gotham::state::State;

/// This is an example of a Gotham `Handler` which will always respond regardless of
/// the `Request` path.
///
/// Dealing with breaking up the Request path and dispatching to application code is a one part of
/// what the full Gotham `Router` offers. This is shown in subsequent examples.
///
/// How does a function become a `Handler`?.
/// We've simply implemented the `Handler` trait for functions that match the signature used here
/// within Gotham itself.
pub fn say_hello(state: State) -> (State, Response) {
    let res = create_response(
        &state,
        StatusCode::Ok,
        Some((String::from("Hello World!").into_bytes(), mime::TEXT_PLAIN)),
    );

    (state, res)
}

/// Start a server and call the `Handler` we've defined above for `Request` we receive.
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    start(addr, || Ok(say_hello))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;

    #[test]
    fn recieve_hello_world_response() {
        let test_server = TestServer::new(|| Ok(say_hello)).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"Hello World!");
    }
}
