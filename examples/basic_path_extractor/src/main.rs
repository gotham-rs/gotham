//! A basic example application for working with the Gotham Path Extractor

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate gotham_derive;

use hyper::{Response, StatusCode};

use gotham::http::response::create_response;
use gotham::router::Router;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::state::State;


/// `Product` struct
///
/// It contains only a `name` field for the sake of simplicity
#[derive(Deserialize, StateData, StaticResponseExtender)]
struct MyProduct {
    name: String,
}

/// Function to handle the `GET` requests coming to `/widgets/:name`
fn get_product_handler(mut state: State) -> (State, Response) {
    // Extract `name` from `state`
    let name = state.take::<MyProduct>().name;
    // Create a response using `name`
    let res = create_response(
        &state,
        StatusCode::Ok,
        Some((
            format!("Product: {}", name).into_bytes(),
            mime::TEXT_PLAIN,
        )),
    );

    (state, res)
}

/// Create a `Router`
///
/// /widgets/:name             --> GET
fn router() -> Router {
    build_simple_router(|route| {
        route
            .get("/widgets/:name")
            .with_path_extractor::<MyProduct>()
            .to(get_product_handler);
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
    fn index_not_found() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NotFound);
    }

    #[test]
    fn product_name_is_extracted() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/widgets/t-shirt")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"Product: t-shirt");
    }

    #[test]
    fn not_found_if_uri_not_following_pattern() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/widgets/foo/bar")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NotFound);
    }

}
