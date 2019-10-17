//! Setting a header value for a Gotham web framework response

extern crate gotham;
extern crate hyper;
extern crate mime;

use gotham::helpers::http::response::create_empty_response;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::State;
use hyper::{Body, Response, StatusCode};

/// Create a `Handler` that adds a custom header.
pub fn handler(state: State) -> (State, Response<Body>) {
    let mut res = create_empty_response(&state, StatusCode::OK);

    {
        let headers = res.headers_mut();
        headers.insert("x-gotham", "Hello World!".parse().unwrap());
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

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("x-gotham").unwrap(), "Hello World!");
    }
}
