//! An introduction to extracting request path segments, in a type safe way, with the
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

use hyper::{Response, StatusCode};

use gotham::http::response::create_response;
use gotham::router::Router;
use gotham::router::builder::*;
use gotham::state::{FromState, State};

/// Holds data extracted from the Request path.
///
/// When a path extraction struct is configured for a route as part of `Router` creation the `Router`
/// will attempt to extract data from each matching request path and store it in state ready for
/// your application to use, ensuring that all type safety requirements have been met by the request
/// before handing back control.
///
/// The key requirements for struct to act as a path extractor are:
///
///     1. That the struct implements `serde::de::Deserialize` which we're doing here by simply
///        deriving it. The Gotham router uses this property during Request path evaluation to
///        create and instance of your struct, populate it and store it into state ready for
///        access by application code.
///     2. That the struct implements `gotham::state::data::StateData` so that it can be stored,
///        retrieved and removed from state. You generally get this for free by deriving
///        `StateData` as shown here.
///     3. That the struct implements the
///        `gotham::router::response::extender::StaticResponseExtender` trait so that bad request
///        path data can be appropriately refused by the Router. You generally get this for free by
///        deriving `StaticResponseExtender` as shown here which results in bad requests being
///        refuted with a HTTP 400 (BadRequest) response status code.
///
/// Naming of fields in extraction structs is important, the same names must appear in the path,
/// proceeded by a colon to indicate a variable, when defining routes.
#[derive(Deserialize, StateData, StaticResponseExtender)]
struct PathExtractor {
    name: String,
}

/// Handler function for `GET` requests directed to `/products/:name`
fn get_product_handler(state: State) -> (State, Response) {
    let res = {
        // Access the `PathExtractor` instance from `state` which was put there for us by the
        // `Router` during request evaluation.
        //
        // As well as permitting storage in `State` by deriving `StateData` our path extractor
        // struct automatically gains the `borrow_from` method and a number of other methods
        // via the `gotham::state::FromState` trait.
        let product = PathExtractor::borrow_from(&state);
        create_response(
            &state,
            StatusCode::Ok,
            Some((
                format!("Product: {}", product.name).into_bytes(),
                mime::TEXT_PLAIN,
            )),
        )
    };

    (state, res)
}

/// Create a `Router`
///
/// /products/:name             --> GET
fn router() -> Router {
    build_simple_router(|route| {
        route
            // Note the use of :name variable in the path defined here. The router will map the
            // second (and last) segment of this path to the field `name` when extracting data.
            .get("/products/:name")
            // This tells the Router that for requests which match this route that path extraction
            // should be invoked storing the result in a `PathExtractor` instance.
            .with_path_extractor::<PathExtractor>()
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
            .get("http://localhost/products/t-shirt")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"Product: t-shirt");
    }
}
