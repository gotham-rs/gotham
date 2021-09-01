//! An finalizer example
use gotham::hyper::{body::Body, Response, StatusCode};
use gotham::router::builder::*;
use gotham::router::response::ResponseExtender;
use gotham::router::Router;
use gotham::state::State;

const HELLO_ROUTER: &str = "Hello Router!";

/// Create a `Handler` that is invoked for requests to the path "/"
pub fn say_hello(state: State) -> (State, &'static str) {
    (state, HELLO_ROUTER)
}

struct ErrorExtender;

///Define an `ResponseExtender`.
///
///Provides a callback after all other processing has  happend
impl ResponseExtender<Body> for ErrorExtender {
    fn extend(&self, _state: &mut State, response: &mut Response<Body>) {
        let body = format!("The status code is {}", response.status());
        *response.body_mut() = body.into();
    }
}

/// Create a `Router`
///
/// Provides tree of routes with only a single top level entry that looks like:
///
/// /                     --> GET
///
/// If no match for a request is found a 404 will be returned. Both the HTTP verb and the request
/// path are considered when determining if the request matches a defined route.
///
/// Also includes a 404 response as an ErrorExtender
fn router() -> Router {
    build_simple_router(|route| {
        // For the path "/" invoke the handler "say_hello"
        route.get("/").to(say_hello);
        route.add_response_extender(StatusCode::NOT_FOUND, ErrorExtender);
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
    use gotham::hyper::StatusCode;
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

    #[test]
    fn receive_404_response() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/no_such_path")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"The status code is 404 Not Found");
    }
}
