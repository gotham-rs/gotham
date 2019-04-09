use gotham::handler::HandlerFuture;
use gotham::handler::IntoHandlerError;
use gotham::helpers::http::response::create_response;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};
use gotham_middleware_diesel::DieselMiddleware;

static DATABASE_URL: &'static str = "products.db";

fn create_product_handler(mut state: State) -> Box<HandlerFuture> {
    let repo = Repo::borrow_from(&state).clone();
    repo.run(move |conn| users.find(user_id).first(&conn))
}

fn get_products_handler(mut state: State) -> Box<HandlerFuture> {}


fn router(repo: Repo) -> Router {
    // Add the middleware to a new pipeline
    let (chain, pipeline) = single_pipeline(new_pipeline().add(DieselMiddleware::new(repo)).build());


    // Build the router
    build_router(chain, pipeline, |route| {
        route.get("/").to(get_products_handler);
        route.post("/").to(create_product_handler);
    })
}

/// Start a server and use a `Router` to dispatch requests
fn main() {
    let addr = "127.0.0.1:7878";

    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router(middleware));
}
