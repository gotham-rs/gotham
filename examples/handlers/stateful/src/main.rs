//! An example of using stateful handlers with the Gotahm web framework.

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;

use futures::future;
use hyper::StatusCode;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use gotham::error::Result;
use gotham::handler::{Handler, HandlerFuture, NewHandler};
use gotham::helpers::http::response::create_response;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::State;

// A struct which can store the state which it needs.
#[derive(Clone)]
struct CountingHandler {
    // Record what time the server started.
    // started_at will never change, so we can just store its value.
    started_at: SystemTime,
    // Count how many visits have been made to the server.
    // visits will change each time the handler is called, so we need to wrap it an an Arc of a
    // Mutex so that we can control concurrent access to it.
    visits: Arc<Mutex<usize>>,
}

impl CountingHandler {
    fn new() -> CountingHandler {
        CountingHandler {
            started_at: SystemTime::now(),
            visits: Arc::new(Mutex::new(0)),
        }
    }
}

impl Handler for CountingHandler {
    fn handle(self, state: State) -> Box<HandlerFuture> {
        let uptime = SystemTime::now().duration_since(self.started_at).unwrap();

        // Create a short scope so that self.visits will only be locked for long enough to
        // increment it, so that other calls to the handler can be processed in parallel.
        let visits = {
            let mut v = self.visits.lock().unwrap();
            *v += 1;
            *v
        };

        let response_text = format!(
            "This server has been up for {} second(s). This is visit number {}.\n",
            uptime.as_secs(),
            visits
        );

        let res = {
            create_response(
                &state,
                StatusCode::OK,
                Some((response_text, mime::TEXT_PLAIN)),
            )
        };
        Box::new(future::ok((state, res)))
    }
}

impl NewHandler for CountingHandler {
    type Instance = Self;

    fn new_handler(&self) -> Result<Self::Instance> {
        Ok(self.clone())
    }
}

/// Create a `Router`
fn router() -> Router {
    build_simple_router(|route| route.get("/").to_new_handler(CountingHandler::new()))
}

/// Start a server and use a `Router` to dispatch requests
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;

    #[test]
    fn counter_increments_per_request() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_utf8_body().unwrap();
        assert!(
            body.ends_with("This is visit number 1.\n"),
            "Wrong number of visits in first response string: {}",
            body
        );

        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_utf8_body().unwrap();
        assert!(
            body.ends_with("This is visit number 2.\n"),
            "Wrong number of visits in second response string: {}",
            body
        );
    }
}
