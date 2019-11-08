//! A Hello World example application for working with Gotham.
//! Supports graceful shutdown on Ctrl+C.

#[cfg(all(test, unix))]
extern crate nix;

use futures::prelude::*;
use hyper::{Body, Response, StatusCode};

use gotham::helpers::http::response::create_response;
use gotham::state::State;

/// Create a `Handler` which is invoked when responding to a `Request`.
///
/// How does a function become a `Handler`?.
/// We've simply implemented the `Handler` trait, for functions that match the signature used here,
/// within Gotham itself.
pub fn say_hello(state: State) -> (State, Response<Body>) {
    let res = create_response(
        &state,
        StatusCode::OK,
        mime::TEXT_PLAIN,
        String::from("Hello World!"),
    );

    (state, res)
}

/// Start a server and call the `Handler` we've defined above for each `Request` we receive.
pub fn main() {
    let addr = "127.0.0.1:7878";

    let server = gotham::init_server(addr, || Ok(say_hello));
    // Future to wait for Ctrl+C.
    let signal = tokio_signal::ctrl_c()
        .flatten_stream()
        .map_err(|error| panic!("Error listening for signal: {}", error))
        .take(1)
        .for_each(|()| {
            println!("Ctrl+C pressed");
            Ok(())
        });

    let serve_until = signal
        .select(server)
        .map(|(res, _)| res)
        .map_err(|(error, _)| error);

    tokio::run(serve_until);

    println!("Shutting down gracefully");
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;
    #[cfg(unix)]
    use hyper::Client;
    #[cfg(unix)]
    use nix::sys::signal::{kill, Signal};
    use std::thread;
    use std::time::Duration;
    #[cfg(unix)]
    use tokio::runtime::Runtime;

    #[test]
    fn receive_hello_world_response() {
        let test_server = TestServer::new(|| Ok(say_hello)).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"Hello World!");
    }

    #[cfg(unix)]
    fn try_request() -> bool {
        let client = Client::new();

        let uri = "http://127.0.0.1:7878/";
        let uri_parsed = uri.parse().unwrap();
        let work = client.get(uri_parsed);

        let mut rt = Runtime::new().unwrap();

        match rt.block_on(work) {
            Ok(req) => {
                assert_eq!(req.status(), StatusCode::OK);
                true
            }

            Err(error) => {
                eprintln!("Unable to get \"{}\": {}", uri, error);
                false
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn signal_self() {
        let thread_handle = thread::spawn(main);

        // Wait until server will be able to answer.
        let mut max_retries = 25;
        while (max_retries != 0) && !try_request() {
            max_retries -= 1;
            thread::sleep(Duration::from_millis(200));
        }
        assert_ne!(max_retries, 0);

        // Send SIGINT to self.
        kill(nix::unistd::getpid(), Signal::SIGINT).unwrap();
        thread_handle.join().unwrap();
    }
}
