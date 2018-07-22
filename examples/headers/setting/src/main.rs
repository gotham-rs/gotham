//! Setting a header value for a Gotham web framework response

extern crate gotham;
#[macro_use]
extern crate hyper;
extern crate mime;

use gotham::helpers::http::response::create_response;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::State;
use hyper::{Response, StatusCode};

// Define a custom header -- just a &'static str
const GothamHeader = "X-Gotham";

/// Create a `Handler` that adds a custom header.
pub fn handler(state: State) -> (State, Response) {
    let mut res = create_response(&state, StatusCode::Ok, None);
    {
        let headers = res.headers_mut();
        headers.set(GothamHeader,"Hello World!".to_owned());
    };

    (state, res)
}

/// Create a `Router`
fn router() -> Router {
    build_simple_router(|route| {
        route.get("/").to(handler);
    })
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
    fn sets_header() {
        let test_server = TestServer::new(|| Ok(handler)).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(
            response.headers().get(GothamHeader).unwrap(),
            &GothamHeader("Hello World!".to_string())
        );
    }
}
