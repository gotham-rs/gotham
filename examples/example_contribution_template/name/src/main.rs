//! Module level comment describing the example ...
//!
//! Delete the comments below from final versions.
//!
//! Please ensure that concepts which a previous example have not introduced are
//! well commented inline. Have a look at other examples to get a feeling for what we mean here.
//!
//! The goal is that someone who has not come across the functionality you're providing an example
//! for previously comes away with a solid understanding which they can directly implement or
//! further enhance by reading through specific Gotham web framework API docs.
//!
//! Minimal examples necessary to describe a specific piece of functionality work out better. There
//! is no need to create a collection of 10 items for example where 1 will do perfectly well.
//!
//! Many examples use the theme of a "web store" to help explain concepts which is something we'd
//! like to continue encouraging for unity purposes.
//!
//! Finally please ships tests for the specific functionality your example is exploring, there is no
//! need however to show tests for Gotham web framework functionality that is outside of the scope
//! of your specific example i.e. That a 404 is correctly returned for a missing endpoint when
//! you're writing an example for setting Cookies.

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;

use hyper::{Response, StatusCode};
use gotham::http::response::create_response;
use gotham::state::State;
use gotham::router::Router;
use gotham::router::builder::*;

/// Create a `Handler` that ...
pub fn well_named_function(state: State) -> (State, Response) {
    let res = create_response(&state, StatusCode::Ok, None);
    (state, res)
}

/// Create a `Router`
fn router() -> Router {
    build_simple_router(|route| {
        route.get("/").to(well_named_function);
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
    fn well_named_test() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
    }
}
