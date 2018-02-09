//! A basic example application for working with the Gotham Query String Extractor

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
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::state::State;
use gotham::handler::IntoResponse;

/// `Product` struct
///
/// It represents products that can be queried
#[derive(Serialize)]
struct Product {
    name: String,
    price: f32,
}

/// Implement `IntoResponse` trait for `Product`
///
/// It allows to create a response out of a `Product`
impl IntoResponse for Product {
    fn into_response(self, state: &State) -> Response {
        create_response(
            state,
            StatusCode::Ok,
            Some((
                serde_json::to_string(&self).unwrap().into_bytes(),
                mime::APPLICATION_JSON,
            )),
        )
    }
}

/// Function to handle the `GET` requests coming to `/widgets/t-shirt`
/// Returns a `(State, Product)` instead of the usual `(State, Response)`
fn get_product_handler(state: State) -> (State, Product) {
    let product = Product {
        name: "t-shirt".to_string(),
        price: 15.5,
    };
    (state, product)
}

/// Create a `Router`
///
/// /widgets/t-shirt            --> GET
fn router() -> Router {
    build_simple_router(|route| {
        route.get("/widgets/t-shirt").to(get_product_handler);
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
            .get("http://localhost/widgets/t-shirt")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"{\"name\":\"t-shirt\",\"price\":15.5}");
    }
}
