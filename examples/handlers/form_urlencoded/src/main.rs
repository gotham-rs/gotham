//! An example of decoding requests from an HTML form element

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate url;

use futures::{future, Future, Stream};
use hyper::{Body, StatusCode};
use url::form_urlencoded;

use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_response;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};

/// Extracts the elements of the POST request and responds with the form keys and values
fn form_handler(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let body_content = valid_body.into_bytes();
                // Perform decoding on request body
                let form_data = form_urlencoded::parse(&body_content).into_owned();
                // Add form keys and values to response body
                let mut res_body = String::new();
                for (key, value) in form_data {
                    let res_body_line = format!("{}: {}\n", key, value);
                    res_body.push_str(&res_body_line);
                }
                let res = create_response(&state, StatusCode::OK, mime::TEXT_PLAIN, res_body);
                future::ok((state, res))
            }
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(f)
}

/// Create a `Router`
fn router() -> Router {
    build_simple_router(|route| {
        route.post("/").to(form_handler);
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
    fn form_request() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost",
                "name=Bob&address=123+Jersey Ave.&message=Hello world%21",
                mime::APPLICATION_WWW_FORM_URLENCODED,
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.read_body().unwrap();
        assert_eq!(
            body,
            "name: Bob\naddress: 123 Jersey Ave.\nmessage: Hello world!\n".as_bytes()
        );
    }
}
