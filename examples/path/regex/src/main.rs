//! An example of the Gotham web framework `Router` that shows how to use Regex patterns in path segments.

extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use hyper::{Body, Response, StatusCode};

use gotham::helpers::http::response::create_response;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct PathExtractor {
    id: usize,
}

/// Create a `Handler` that is invoked for requests using a numeric identifier.
pub fn greet_user(state: State) -> (State, Response<Body>) {
    let res = {
        let path = PathExtractor::borrow_from(&state);
        let response_string = format!("Hello, User {}!", &path.id);

        create_response(&state, StatusCode::OK, (response_string, mime::TEXT_PLAIN))
    };

    (state, res)
}

/// Create a `Router`
///
/// Provides tree of routes with only a single top level entry that looks like:
///
/// /user/:id:[0-9]+                     --> GET
///
/// If no match for a request is found a 404 will be returned. Both the HTTP verb and the request
/// path are considered when determining if the request matches a defined route.
fn router() -> Router {
    build_simple_router(|route| {
        route
            // The pattern here will enforce the :id segment is numeric.
            .get("/user/:id:[0-9]+")
            .with_path_extractor::<PathExtractor>()
            .to(greet_user);
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
            .get("http://localhost/user/123")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"Hello, User 123!");
    }

    #[test]
    fn receive_missing_route_response() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/user/abc")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
