//! An example application working with the diesel middleware.

#[macro_use]
extern crate diesel;

#[cfg(test)]
#[macro_use]
extern crate diesel_migrations;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::{future, Future, Stream};
use gotham::handler::{HandlerError, HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_response;
use gotham::pipeline::{new_pipeline, single::single_pipeline};
use gotham::router::{builder::*, Router};
use gotham::state::{FromState, State};
use gotham_middleware_diesel::DieselMiddleware;
use hyper::{Body, StatusCode};
use serde_derive::Serialize;
use std::str::from_utf8;

mod models;
mod schema;

use models::{NewProduct, Product};
use schema::products;

// For this example, we'll use a static database URL,
// although one might commonly pass this in via
// environment variables instead.
static DATABASE_URL: &'static str = "products.db";

// We'll use a file based Sqlite database to keep things simple.
// Don't forget to run the step in the README to create the database
// first using the diesel cli.
// For convenience, we define a type for our app's database "Repo",
// with `SqliteConnection` as it's connection type.
pub type Repo = gotham_middleware_diesel::Repo<SqliteConnection>;

#[derive(Serialize)]
struct RowsUpdated {
    rows: usize,
}

fn create_product_handler(mut state: State) -> Box<HandlerFuture> {
    let repo = Repo::borrow_from(&state).clone();
    let f = extract_json::<NewProduct>(&mut state)
        .and_then(move |product| {
            repo.run(move |conn| {
                // Insert the `NewProduct` in the DB
                diesel::insert_into(products::table)
                    .values(&product)
                    .execute(&conn)
            })
            .map_err(|e| e.into_handler_error())
        })
        .then(|result| match result {
            Ok(rows) => {
                let body = serde_json::to_string(&RowsUpdated { rows })
                    .expect("Failed to serialise to json");
                let res =
                    create_response(&state, StatusCode::CREATED, mime::APPLICATION_JSON, body);
                future::ok((state, res))
            }
            Err(e) => future::err((state, e)),
        });
    Box::new(f)
}

fn get_products_handler(state: State) -> Box<HandlerFuture> {
    use crate::schema::products::dsl::*;

    let repo = Repo::borrow_from(&state).clone();
    let f = repo
        .run(move |conn| products.load::<Product>(&conn))
        .then(|result| match result {
            Ok(users) => {
                let body = serde_json::to_string(&users).expect("Failed to serialize users.");
                let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
                future::ok((state, res))
            }
            Err(e) => future::err((state, e.into_handler_error())),
        });
    Box::new(f)
}

fn router(repo: Repo) -> Router {
    // Add the diesel middleware to a new pipeline
    let (chain, pipeline) =
        single_pipeline(new_pipeline().add(DieselMiddleware::new(repo)).build());

    // Build the router
    build_router(chain, pipeline, |route| {
        route.get("/").to(get_products_handler);
        route.post("/").to(create_product_handler);
    })
}

fn bad_request<E>(e: E) -> HandlerError
where
    E: std::error::Error + Send + 'static,
{
    e.into_handler_error().with_status(StatusCode::BAD_REQUEST)
}

fn extract_json<T>(state: &mut State) -> impl Future<Item = T, Error = HandlerError>
where
    T: serde::de::DeserializeOwned,
{
    Body::take_from(state)
        .concat2()
        .map_err(bad_request)
        .and_then(|body| {
            let b = body.to_vec();
            from_utf8(&b)
                .map_err(bad_request)
                .and_then(|s| serde_json::from_str::<T>(s).map_err(bad_request))
        })
}

/// Start a server and use a `Router` to dispatch requests
fn main() {
    let addr = "127.0.0.1:7878";

    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router(Repo::new(DATABASE_URL)));
}

// In tests `Repo::with_test_transactions` allows queries to run
// within an isolated test transaction. This means multiple tests
// can run in parallel without trampling on each other's data.
// This isn't necessary when using an SQLite in-memory only database
// as is used here, but is demonstrated here anyway to show how it
// might be used agaist a real database.
#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;
    use gotham_middleware_diesel::Repo;
    use hyper::StatusCode;
    use std::str;
    use tokio::runtime;

    static DATABASE_URL: &'static str = ":memory:";

    // For this example, we run migrations automatically in each test.
    // You could also choose to do this separately using something like
    // `cargo-make` (https://sagiegurari.github.io/cargo-make/) to run
    // migrations before the test suite.
    embed_migrations!();

    #[test]
    fn get_empty_products() {
        let repo = Repo::with_test_transactions(DATABASE_URL);
        runtime::run(repo.run(|conn| embedded_migrations::run(&conn).map_err(|_| ())));
        let test_server = TestServer::new(router(repo)).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        let str_body = str::from_utf8(&body).unwrap();
        let index = "[]";
        assert_eq!(str_body, index);
    }

    #[test]
    fn create_and_retrieve_product() {
        let repo = Repo::with_test_transactions(DATABASE_URL);
        runtime::run(repo.run(|conn| embedded_migrations::run(&conn).map_err(|_| ())));
        let test_server = TestServer::new(router(repo)).unwrap();

        //  First we'll insert something into the DB with a post
        let body = r#"{"title":"test","price":1.0,"link":"http://localhost"}"#;
        let response = test_server
            .client()
            .post("http://localhost", body, mime::APPLICATION_JSON)
            .perform()
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // Then we'll query it and test that it is returned
        // As long as we're hitting a `test_server` created with the same
        // `Repo` instance, we're in the same test transaction, and our
        // data will be there across queries.
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        let str_body = str::from_utf8(&body).unwrap();
        let index = r#"[{"id":1,"title":"test","price":1.0,"link":"http://localhost"}]"#;
        assert_eq!(str_body, index);
    }
}
