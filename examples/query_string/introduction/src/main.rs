//! An introduction to extracting query string name/value pairs, in a type safe way, with the
//! Gotham web framework

extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
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
use gotham::state::{FromState, State};

/// Holds data extracted from the Request query string.
///
/// When a query string extraction struct is configured for a route as part of `Router` creation
/// the `Router` will attempt to extract data from each matching request's query string and store
/// it in `state` ready for your application to use, ensuring that all type safety requirements have
/// been met by the request before handing back control.
///
/// The key requirements for struct to act as a query string extractor are:
///
///     1. That the struct implements the `serde::de::Deserialize` trait which we're doing here by
///        simply deriving it. The Gotham router uses this property during Request query string
///        evaluation to create and instance of your struct, populate it and store it into state
///        ready for access by application code.
///     2. That the struct implements `gotham::state::data::StateData` trait so that it can be
///        stored, retrieved and removed from state. You generally get this for free by deriving
///        `StateData` as shown here.
///     3. That the struct implements the
///        `gotham::router::response::extender::StaticResponseExtender` trait so that bad request
///        query string data can be appropriately refused by the Router. You generally get this
///        for free by deriving `StaticResponseExtender` as shown here which results in bad
///        requests being refuted with a HTTP 400 (BadRequest) response status code.
///
/// Naming of fields in extraction structs is important, the same names must appear in the
/// query string.
#[derive(Deserialize, StateData, StaticResponseExtender)]
struct QueryStringExtractor {
    name: String,
}

/// A Product
#[derive(Serialize)]
struct Product {
    name: String,
}

/// Handler function for `GET` requests directed to `/products`
///
/// This handler uses the Serde project when generating responses. You don't need to
/// know about Serde in order to understand the response that is being created here but if you're
/// interested you can learn more at `http://serde.rs`.
fn get_product_handler(mut state: State) -> (State, Response) {
    let res = {
        // Access the `QueryStringExtractor` instance from `state` which was put there for us by the
        // `Router` during request evaluation.
        //
        // As well as permitting storage in `State` by deriving `StateData` our query string
        // extractor struct automatically gains the `take_from` method and a number of other
        // methods via the `gotham::state::FromState` trait.
        //
        // n.b. Once taken out of `state` values can no longer be accessed by other application
        // code or middlewares.
        let query_param = QueryStringExtractor::take_from(&mut state);

        let product = Product {
            name: query_param.name,
        };
        create_response(
            &state,
            StatusCode::Ok,
            Some((
                serde_json::to_vec(&product).expect("serialized product"),
                mime::APPLICATION_JSON,
            )),
        )
    };
    (state, res)
}

/// Create a `Router`
///
/// /products?name=...             --> GET
fn router() -> Router {
    build_simple_router(|route| {
        route
            .get("/products")
            // This tells the Router that for requests which match this route that query string
            // extraction should be invoked storing the result in a `QueryStringExtractor` instance.
            .with_query_string_extractor::<QueryStringExtractor>()
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
    fn product_name_is_extracted() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/products?name=t-shirt")
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
