use gotham::router::Router;
use gotham::router::builder::*;

use controllers::pages;
use config::middleware;

/// Create a `Router`
pub fn get() -> Router {
    let (default_pipeline_chain, final_pipeline_set) = middleware::make_pipelines();
    build_router(default_pipeline_chain, final_pipeline_set, |route| {
        route.get("/database").to(pages::database);
    })
}
