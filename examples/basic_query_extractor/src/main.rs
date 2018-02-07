//! A basic example application for working with the Gotham Query String Extractor 

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate gotham_derive;

use hyper::{Response, StatusCode};

use gotham::http::response::create_response;
use gotham::router::Router;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::state::State;

/// `QueryParam` struct
///
/// It contains only a `name` field for the sake of simplicity
/// It permits to use it as query parameter like:
/// GET /widgets?name=t-shirt
#[derive(Deserialize, StateData, StaticResponseExtender)]
struct QueryParam {
    name: String,
}

/// `Product` struct
///
/// It represents products that can be queried
#[derive(Serialize)]
struct Product {
    name: String,
    price: f32,
}

// Generate a `Vec<Product>`
fn generate_products() -> Vec<Product> {
    let mut products = Vec::with_capacity(3);
    products.push(Product {
        name: "t-shirt".to_string(),
        price: 15.5,
    });
    products.push(Product {
        name: "sticker".to_string(),
        price: 1.5,
    });
    products.push(Product {
        name: "mug".to_string(),
        price: 10.0,
    });
    products
}

/// Function to handle the `GET` requests coming to `/widgets/:name`
fn get_product_handler(mut state: State) -> (State, Response) {
    // Extract `name` from `state`
    let name = state.take::<QueryParam>().name;
    // Generate the products and try to find a match
    let products = generate_products()
        .into_iter()
        .filter(|p| p.name == name)
        .collect::<Vec<Product>>();
    // Check if a product is found. If not, return `StatusCode::NotFound`
    let product = match products.get(0) {
        Some(p) => p,
        None => return (state, Response::new().with_status(StatusCode::NotFound)),
    };
    // Create a response using `Product` serialized to JSON
    let res = create_response(
        &state,
        StatusCode::Ok,
        Some((
            format!("{}", serde_json::to_string(&product).unwrap())
                .into_bytes(),
            mime::TEXT_PLAIN,
        )),
    );

    (state, res)
}

/// Create a `Router`
///
/// /widgets?name=...             --> GET
fn router() -> Router {
    build_simple_router(|route| {
        route
            .get("/widgets")
            .with_query_string_extractor::<QueryParam>()
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
    fn tshirt_found() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/widgets?name=t-shirt")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"{\"name\":\"t-shirt\",\"price\":15.5}");
    }

    #[test]
    fn foo_not_found() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/widgets?name=foo")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NotFound);
    }

}
