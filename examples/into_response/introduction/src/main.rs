//! An introduction to the Gotham web framework's `IntoResponse` trait.

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use hyper::{Response, StatusCode};

use gotham::http::response::create_response;
use gotham::router::Router;
use gotham::router::builder::*;
use gotham::state::State;
use gotham::handler::IntoResponse;

/// A Product
#[derive(Serialize)]
struct Product {
    name: String,
}

/// Implements `gotham::handler::IntoResponse` trait for `Product`
///
/// `IntoResponse` represents a type which can be converted to a response. This trait is used in
/// converting the return type of a function into a response.
///
/// This trait implementation uses the Serde project when generating responses. You don't need to
/// know about Serde in order to understand the response that is being created here but if you're
/// interested you can learn more at `http://serde.rs`.
impl IntoResponse for Product {
    fn into_response(self, state: &State) -> Response {
        create_response(
            state,
            StatusCode::Ok,
            Some((
                serde_json::to_string(&self)
                    .expect("serialized product")
                    .into_bytes(),
                mime::APPLICATION_JSON,
            )),
        )
    }
}

/// Function to handle the `GET` requests coming to `/products/t-shirt`
///
/// Note that this function returns a `(State, Product)` instead of the usual `(State, Response)`.
/// As we've implemented `IntoResponse` above Gotham will correctly handle this and call our
/// `into_response` method when appropriate.
fn get_product_handler(state: State) -> (State, Product) {
    let product = Product {
        name: "t-shirt".to_string(),
    };

    (state, product)
}

/// Create a `Router`
///
/// /products/t-shirt            --> GET
fn router() -> Router {
    build_simple_router(|route| {
        route.get("/products/t-shirt").to(get_product_handler);
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
    fn get_product_response() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/products/t-shirt")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        let expected_product = Product {
            name: "t-shirt".to_string(),
        };
        let expected_body = serde_json::to_string(&expected_product).expect("serialized product");
        assert_eq!(&body[..], expected_body.as_bytes());
    }
}
