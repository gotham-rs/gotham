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
use gotham::state::{State, FromState};

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
    description: String,
}

// Generate a `Vec<Product>`
fn generate_products() -> Vec<Product> {
    vec![
        Product {
            name: "t-shirt".to_string(),
            description: "t-shirt".to_string(),
        },
    ]
}

/// Returns a `Response` with the `Product` serialized to JSON
/// If no `Product` is found, a `Response` containing a `StatusCode::NotFound`
/// is returned.
fn product_matcher(requested: &str, products: &Vec<Product>, state: &State) -> Response {
    match products.iter().find(|p| p.name == requested) {
        Some(product) => {
            create_response(
                state,
                StatusCode::Ok,
                Some((
                    serde_json::to_vec(product).unwrap(),
                    mime::APPLICATION_JSON,
                )),
            )
        }
        None => Response::new().with_status(StatusCode::NotFound),
    }
}

/// Function to handle the `GET` requests coming to `/widgets/:name`
fn get_product_handler(state: State) -> (State, Response) {
    // Build a response
    let res = {
        // Extract the `QueryParam` from `state`
        let query_param = QueryParam::borrow_from(&state);
        let name = &query_param.name;
        // Generate the response
        product_matcher(&name, &generate_products(), &state)
    };
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
    fn tshirt_found() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/widgets?name=t-shirt")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        let expected_product = Product {
            name: "t-shirt".to_string(),
            description: "t-shirt".to_string(),
        };
        let expected_body = serde_json::to_string(&expected_product).expect("Serialize product");
        assert_eq!(&body[..], expected_body.as_bytes());
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
