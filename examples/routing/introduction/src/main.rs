//! An introduction to fundamental `Router` and `Router Builder` concepts to create a routing tree.

extern crate gotham;
extern crate hyper;
extern crate mime;

use hyper::{Body, Response, StatusCode};

use gotham::helpers::http::response::create_response;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::State;

/// Create a `Handler` that is invoked for requests to the path "/"
pub fn say_hello(state: State) -> (State, Response<Body>) {
    let res = create_response(
        &state,
        StatusCode::OK,
        Some((String::from("Hello Router!").into_bytes(), mime::TEXT_PLAIN)),
    );

    (state, res)
}

/// Create a `Router`
///
/// Provides tree of routes with only a single top level entry that looks like:
///
/// /                     --> GET
///
/// If no match for a request is found a 404 will be returned. Both the HTTP verb and the request
/// path are considered when determining if the request matches a defined route.
fn router() -> Router {
    build_simple_router(|route| {
        // For the path "/" invoke the handler "say_hello"
        route.get("/").to(say_hello);
    })
}

/// Start a server and use a `Router` to dispatch requests
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);

    // All incoming requests are delegated to the router for further analysis and dispatch
    gotham::start(addr, router())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;

    #[test]
    fn receive_hello_router_response() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"Hello Router!");
    }
}
