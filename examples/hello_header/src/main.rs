//! A simple introduction to working with Gotham and custom headers.

extern crate futures;
extern crate gotham;
#[macro_use]
extern crate hyper;
extern crate mime;

use hyper::{Response, StatusCode};
use gotham::http::response::create_response;
use gotham::state::State;
use gotham::router::Router;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};

// this macro defines our custom header
header! { (GothamHeader, "X-Gotham") => [String] }

/// Create a `Handler` that adds a custom header.
pub fn say_hello_through_header(state: State) -> (State, Response) {
    let mut res = create_response(
        &state,
        StatusCode::Ok,
        Some((b"".to_vec(), mime::TEXT_PLAIN)),
    );
    {
        let headers = res.headers_mut();
        headers.set(GothamHeader("Hello World!".to_owned()));
    };

    (state, res)
}

/// Create a `Router`
fn router() -> Router {
    build_simple_router(|route| { route.get("/").to(say_hello_through_header); })
}

/// Start a server and call the `Handler` we've defined above for each `Request` we receive.
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
    fn recieve_hello_world_header() {
        let test_server = TestServer::new(|| Ok(say_hello_through_header)).unwrap();
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
