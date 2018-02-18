//! Setting a header value for a Gotham web framework response

extern crate futures;
extern crate gotham;
#[macro_use]
extern crate hyper;
extern crate mime;

use hyper::{Response, StatusCode};
use gotham::http::response::create_response;
use gotham::state::State;
use gotham::router::Router;
use gotham::router::builder::*;

// Define a custom header via the standard Hyper provided macro
header! { (GothamHeader, "X-Gotham") => [String] }

/// Create a `Handler` that adds a custom header.
pub fn handler(state: State) -> (State, Response) {
    let mut res = create_response(&state, StatusCode::Ok, None);
    {
        let headers = res.headers_mut();
        headers.set(GothamHeader("Hello World!".to_owned()));
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
            response.headers().get::<GothamHeader>().unwrap(),
            &GothamHeader("Hello World!".to_string())
        );
    }
}
