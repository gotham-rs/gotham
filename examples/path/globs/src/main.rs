//! Shows how to match arbitrarily many path segments.

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

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct PathExtractor {
    // If there is exactly one * in the route, and it is the last path segment, this will be a Vec
    // containing each path segment as a separate String, with no /s.
    #[serde(rename = "*")]
    parts: Vec<String>,
}

fn parts_handler(state: State) -> (State, Response) {
    let res = {
        let path = PathExtractor::borrow_from(&state);

        let mut response_string = format!(
            "Got {} part{}:",
            path.parts.len(),
            if path.parts.len() == 1 { "" } else { "s" }
        );
        for part in path.parts.iter() {
            response_string.push_str("\n");
            response_string.push_str(&part);
        }

        create_response(
            &state,
            StatusCode::Ok,
            Some((response_string.into_bytes(), mime::TEXT_PLAIN)),
        )
    };

    (state, res)
}

fn router() -> Router {
    build_simple_router(|route| {
        route
            // The last path segment is allowed to be a *, and it will match one or more path segments.
            .get("/parts/*")
            .with_path_extractor::<PathExtractor>()
            .to(parts_handler);
    })
}

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
    fn empty_glob_does_not_match() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/parts")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NotFound);
    }

    #[test]
    fn just_trailing_slash_does_not_match() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/parts/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NotFound);
    }

    #[test]
    fn extracts_one_component() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/parts/head")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], &b"Got 1 part:\nhead"[..]);
    }

    #[test]
    fn extracts_multiple_components() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/parts/head/shoulders/knees/toes")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(
            &body[..],
            &b"Got 4 parts:\nhead\nshoulders\nknees\ntoes"[..]
        );
    }
}
