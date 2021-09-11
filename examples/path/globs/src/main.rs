//! Shows how to match arbitrarily many path segments.

use gotham::router::builder::*;
use gotham::router::response::StaticResponseExtender;
use gotham::router::Router;
use gotham::state::{FromState, State, StateData};
use serde::Deserialize;

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct PathExtractor {
    // This will be a Vec containing each path segment as a separate String, with no '/'s.
    #[serde(rename = "*")]
    parts: Vec<String>,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct NamedPathExtractor {
    parts: Vec<String>,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct MultiGlobExtractor {
    top: Vec<String>,
    bottom: Vec<String>,
}

fn parts_handler(state: State) -> (State, String) {
    let res = {
        let path = PathExtractor::borrow_from(&state);

        let mut response_string = format!(
            "Got {} part{}:",
            path.parts.len(),
            if path.parts.len() == 1 { "" } else { "s" }
        );

        for part in path.parts.iter() {
            response_string.push('\n');
            response_string.push_str(part);
        }

        response_string
    };

    (state, res)
}

fn named_parts_handler(state: State) -> (State, String) {
    let res = {
        let path = NamedPathExtractor::borrow_from(&state);

        let mut response_string = format!(
            "Got {} part{}:",
            path.parts.len(),
            if path.parts.len() == 1 { "" } else { "s" }
        );

        for part in path.parts.iter() {
            response_string.push('\n');
            response_string.push_str(part);
        }

        response_string
    };

    (state, res)
}

fn multi_parts_handler(state: State) -> (State, String) {
    let res = {
        let path = MultiGlobExtractor::borrow_from(&state);

        let mut top = format!(
            "Got {} part{} for top:",
            path.top.len(),
            if path.top.len() == 1 { "" } else { "s" }
        );

        for part in path.top.iter() {
            top.push('\n');
            top.push_str(part);
        }

        let mut bottom = format!(
            "Got {} part{} for bottom:",
            path.bottom.len(),
            if path.bottom.len() == 1 { "" } else { "s" }
        );

        for part in path.bottom.iter() {
            bottom.push('\n');
            bottom.push_str(part);
        }

        vec![top, bottom].join("\n\n")
    };

    (state, res)
}

fn router() -> Router {
    build_simple_router(|route| {
        route
            // A path segment is allowed to be a *, and it will match one or more path segments.
            .get("/parts/*")
            .with_path_extractor::<PathExtractor>()
            .to(parts_handler);

        route
            // You can provide a param name for this glob segment.
            // It doesn't need to be the last segment
            .get("/middle/*parts/foobar")
            .with_path_extractor::<NamedPathExtractor>()
            .to(named_parts_handler);

        route
            // You can even have multiple glob segments
            .get("/multi/*top/foobar/*bottom")
            .with_path_extractor::<MultiGlobExtractor>()
            .to(multi_parts_handler);
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
    use gotham::hyper::StatusCode;
    use gotham::test::TestServer;

    #[test]
    fn empty_glob_does_not_match() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/parts")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn just_trailing_slash_does_not_match() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/parts/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn extracts_one_component() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/parts/head")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(
            &body[..],
            &b"Got 4 parts:\nhead\nshoulders\nknees\ntoes"[..]
        );
    }

    #[test]
    fn extracts_named_multiple_components_from_middle() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/middle/head/shoulders/knees/toes/foobar")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(
            &body[..],
            &b"Got 4 parts:\nhead\nshoulders\nknees\ntoes"[..]
        );
    }

    #[test]
    fn extracts_multiple_multiple_components() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/multi/head/shoulders/foobar/knees/toes")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(
            &body[..],
            &b"Got 2 parts for top:\nhead\nshoulders\n\nGot 2 parts for bottom:\nknees\ntoes"[..]
        );
    }
}
