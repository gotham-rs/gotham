//! A basic example showing the request components

extern crate gotham;
extern crate hyper;
extern crate mime;

use hyper::{Body, Headers, HttpVersion, Method, Response, StatusCode, Uri};

use gotham::http::response::create_response;
use gotham::state::{FromState, State};
use gotham::router::Router;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};

/// Show the request components by printing them.
pub fn show_request(state: State) -> (State, Response) {
    {
        let method = Method::borrow_from(&state);
        let uri = Uri::borrow_from(&state);
        let http_version = HttpVersion::borrow_from(&state);
        let headers = Headers::borrow_from(&state);
        let body = Body::borrow_from(&state);
        println!("Method: {:?}", method);
        println!("URI: {:?}", uri);
        println!("HTTP Version: {:?}", http_version);
        println!("Headers: {:?}", headers);
        println!("Body: {:?}", body);
    }
    let res = create_response(&state, StatusCode::Ok, Some((vec![], mime::TEXT_PLAIN)));

    (state, res)
}

/// Create a `Router`
fn router() -> Router {
    build_simple_router(|route| {
        route.get("/").to(show_request);
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
    fn receive_response() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"");
    }
}
