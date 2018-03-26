//! A basic example showing the request components

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;

use futures::{future, Future, Stream};
use hyper::{Body, Headers, HttpVersion, Method, Response, StatusCode, Uri};

use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::http::response::create_response;
use gotham::router::Router;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::state::{FromState, State};

/// Extract the main elements of the request except for the `Body`
fn print_request_elements(state: &State) {
    let method = Method::borrow_from(state);
    let uri = Uri::borrow_from(state);
    let http_version = HttpVersion::borrow_from(state);
    let headers = Headers::borrow_from(state);
    println!("Method: {:?}", method);
    println!("URI: {:?}", uri);
    println!("HTTP Version: {:?}", http_version);
    println!("Headers: {:?}", headers);
}

/// Extracts the elements of the POST request and prints them
fn post_handler(mut state: State) -> Box<HandlerFuture> {
    print_request_elements(&state);
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
                println!("Body: {}", body_content);
                let res = create_response(&state, StatusCode::Ok, None);
                future::ok((state, res))
            }
            Err(e) => return future::err((state, e.into_handler_error())),
        });

    Box::new(f)
}

/// Show the GET request components by printing them.
fn get_handler(state: State) -> (State, Response) {
    print_request_elements(&state);
    let res = create_response(&state, StatusCode::Ok, None);

    (state, res)
}

/// Create a `Router`
fn router() -> Router {
    build_simple_router(|route| {
        route.associate("/", |assoc| {
            assoc.get().to(get_handler);
            assoc.post().to(post_handler);
        });
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
    fn get_request() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
    }

    #[test]
    fn post_request() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post("http://localhost", None, mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
    }
}
