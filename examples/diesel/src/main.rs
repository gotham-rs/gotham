#[macro_use]
extern crate diesel;

use diesel::sqlite::SqliteConnection;
use futures::{future, Future, Stream};
use gotham::handler::{HandlerError, HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_response;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};
use gotham_middleware_diesel::{repo, DieselMiddleware};
use hyper::{Body, StatusCode};
use std::str::from_utf8;

mod models;
mod schema;

use models::{NewProduct, Product};
use schema::products;

static DATABASE_URL: &'static str = "products.db";
pub type Repo = repo::Repo<SqliteConnection>;

fn create_product_handler(mut state: State) -> Box<HandlerFuture> {
    let repo = Repo::borrow_from(&state).clone();
    let f = extract_json::<NewProduct>(&mut state)
        .and_then(|product| {
            repo.run(move |conn| {
                // Insert the `NewProduct` in the DB
                diesel::insert_into(products::table)
                    .values(&product)
                    .execute(conn)
            })
        })
        .and_then(|result| match result {
            Ok(num_rows) => {
                let body = format!("{{\"rows\": {} }}", num_rows);
                let res =
                    create_response(&state, StatusCode::CREATED, mime::APPLICATION_JSON, body);
                future::ok((state, res))
            }
            Err(e) => future::err((state, e.into_handler_error())),
        });
    Box::new(f)
}

fn get_products_handler(mut state: State) -> Box<HandlerFuture> {
    use crate::schema::products::dsl::*;

    let repo = Repo::borrow_from(&state).clone();
    let f = repo
        .run(move |conn| products.load(&conn))
        .and_then(|result| match result {
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
    // Add the middleware to a new pipeline
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
