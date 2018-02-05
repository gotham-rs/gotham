extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate gotham_middleware_diesel;
extern crate diesel;
extern crate r2d2_diesel;
extern crate r2d2;
extern crate basic_diesel;
extern crate serde_json;

use hyper::{Response, StatusCode};
use gotham::state::{State, FromState};
use gotham::router::Router;
use gotham::pipeline::new_pipeline;
use gotham::router::builder::*;
use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set};
use gotham::handler::HandlerFuture;
use gotham::http::response::create_response;
use gotham_middleware_diesel::DieselMiddleware;
use gotham::handler::IntoHandlerError;
use diesel::sqlite::SqliteConnection;
use r2d2_diesel::ConnectionManager;
use r2d2::{Pool, PooledConnection};
use futures::{future, Future, Stream};
use std::str;
use basic_diesel::models::NewPost;


// The URL of the database.
static DATABASE_URL: &'static str = "posts.db";


/// Creates the `DieselMiddleware` from an `url` that is passed to the function
fn create_middleware(url: &str) -> DieselMiddleware<SqliteConnection> {
    let manager = ConnectionManager::new(url);
    let pool = Pool::<ConnectionManager<SqliteConnection>>::new(manager).unwrap();
    // Create the `DieselMiddleware`
    DieselMiddleware::with_pool(pool)
}

/// Handler function. Responsible of getting and displaying the posts from the DB
fn get_posts_handler(state: State) -> (State, Response) {
    let conn: PooledConnection<ConnectionManager<SqliteConnection>> =
        gotham_middleware_diesel::state_data::connection(&state);
    let posts = basic_diesel::get_posts(&conn);

    (
        state,
        Response::new().with_status(StatusCode::Ok).with_body(
            format!(
                "{}",
                serde_json::to_string(&posts).unwrap()
            ),
        ),
    )
}

/// Handle function. Manages the `NewPost` to insert to the DB
fn post_post_handler(mut state: State) -> Box<HandlerFuture> {
    let f = hyper::Body::take_from(&mut state).concat2().then(
        move |full_body| match full_body {
            Ok(valid_body) => {
                let post: NewPost = match serde_json::from_slice(&valid_body) {
                    Ok(p) => p,
                    Err(e) => return future::err((state, e.into_handler_error())),
                };
                let conn: PooledConnection<ConnectionManager<SqliteConnection>> =
                    gotham_middleware_diesel::state_data::connection(&state);
                let mut res: Response;
                match basic_diesel::create_post(&conn, post.title, post.body) {
                    Ok(_) => {
                        res = create_response(
                            &state,
                            StatusCode::Created,
                            Some((vec![], mime::TEXT_PLAIN)),
                        )
                    }
                    Err(e) => return future::err((state, e.into_handler_error())),
                }
                future::ok((state, res))
            }
            Err(e) => future::err((state, e.into_handler_error())),
        },
    );

    Box::new(f)
}


/// Create a `Router`
///
/// The resulting tree looks like:
///
/// /                         --> GET, POST
///
/// It returns the content of the SQLite DB file located in `.posts.db`
/// This DB consists of `Post` entries.
fn router(middleware: DieselMiddleware<SqliteConnection>) -> Router {
    // Create a new pipeline set
    let editable_pipeline_set = new_pipeline_set();

    // Add the middleware to a new pipeline
    let (editable_pipeline_set, pipeline) =
        editable_pipeline_set.add(new_pipeline().add(middleware).build());
    let pipeline_set = finalize_pipeline_set(editable_pipeline_set);

    let default_pipeline_chain = (pipeline, ());

    // Build the router
    build_router(default_pipeline_chain, pipeline_set, |route| {
        route.get("/").to(get_posts_handler);
        route.post("/").to(post_post_handler);
    })
}



/// Start a server and use a `Router` to dispatch requests
fn main() {
    let addr = "127.0.0.1:7878";

    let middleware = create_middleware(DATABASE_URL);

    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router(middleware));
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;
    use hyper::StatusCode;
    use std::str;


    #[test]
    fn get_empty_posts() {
        let middleware = create_middleware("empty.db");
        let test_server = TestServer::new(router(middleware)).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        let str_body = str::from_utf8(&body).unwrap();
        let index = "[]";
        assert_eq!(str_body, index);
    }

    #[test]
    fn create_post() {
        let middleware = create_middleware("test_post.db");
        let test_server = TestServer::new(router(middleware)).unwrap();
        let body = "{\"title\":\"test\",\"body\":\"test post\",\"published\":true}";
        let response = test_server
            .client()
            .post("http://localhost", body, mime::APPLICATION_JSON)
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Created);
    }
}
