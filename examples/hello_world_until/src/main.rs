//! A Hello World example application for working with Gotham.
//! Supports graceful shutdown on Ctrl+C.

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate tokio_core;
extern crate tokio_signal;

#[cfg(all(test, unix))]
extern crate nix;

use std::thread;
use std::time::Duration;

use futures::sync::oneshot;
use futures::{Future, Stream};
use hyper::{Body, Response, StatusCode};
use tokio_core::reactor::Core;

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
        StatusCode::Ok,
        mime::TEXT_PLAIN,
        String::from("Hello World!")
    );

    (state, res)
}

/// Start a server and call the `Handler` we've defined above for each `Request` we receive.
pub fn main() {
    let addr = "127.0.0.1:7878";

    // Channel used by main thread to shut down Gotham thread.
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    // Channel used by Gotham thread to signal main thread about panic in Gotham thread.
    let (panic_tx, panic_rx) = oneshot::channel();

    // Gotham thread. gotham::run_until() will block this thread. Also, gotham::run_until() may spawn additional
    // threads depending on number of available CPU cores. Also see gotham::run_with_num_threads_until().
    let gotham_thread = thread::spawn(move || {
        println!("Listening for requests at http://{}", addr);
        println!("Press Ctrl+C to exit");
        gotham::run_until(
            addr,
            || Ok(say_hello),
            shutdown_rx.map_err(|error| panic!("Shutdown signal sender was dropped ({}).", error)),
            Duration::from_secs(5),
        );
    });

    // Second thread which is used to catch possible panic in the first thread (gotham_thread).
    // One also may try to use std::panic::catch_unwind() instead of creating additional thread.
    let catch_panic_thread = thread::spawn(move || {
        if let Err(panic) = gotham_thread.join() {
            if panic_tx.send(()).is_err() {
                eprintln!("Failed to propagate panic from thread: receiver dropped");
            };
            // Propagate panic further to the main thread.
            Err(panic)
        } else {
            Ok(())
        }
    });

    let mut core = Core::new().expect("Failed to create reactor::Core");
    let handle = core.handle();

    // Future to wait for Ctrl+C.
    let signal = tokio_signal::ctrl_c(&handle)
        .flatten_stream()
        .map_err(|error| panic!("Error listening for signal: {}", error))
        .take(1)
        .for_each(|()| {
            println!("Ctrl+C pressed");
            Ok(())
        });

    let panic_rx = panic_rx.map_err(|error| panic!("Panic sender was dropped ({}).", error));
    // Wait for either Ctrl+C or panic in Gotham thread.
    // `let _ = ...` drops unfinished future.
    let _ = core.run(signal.select(panic_rx))
        .map_err(|(error, _)| error)
        .unwrap();

    // Send shutdown signal to the Gotham thread.
    shutdown_tx.send(()).unwrap();

    // Wait for the last thread.
    catch_panic_thread
        .join()
        .unwrap()
        .expect("Late panic in the Gotham thread");

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

    #[test]
    fn receive_hello_world_response() {
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

    #[cfg(unix)]
    fn try_request() -> bool {
        let mut core = Core::new().unwrap();
        let client = Client::new(&core.handle());

        let uri = "http://127.0.0.1:7878/";
        let uri_parsed = uri.parse().unwrap();
        let work = client.get(uri_parsed).map(|res| {
            assert_eq!(res.status(), StatusCode::Ok);
        });

        match core.run(work) {
            Ok(_) => true,

            Err(error) => {
                eprintln!("Unable to get \"{}\": {}", uri, error);
                false
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn signal_self() {
        let thread_handle = thread::spawn(|| main());

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
