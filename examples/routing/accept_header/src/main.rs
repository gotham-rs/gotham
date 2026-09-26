//! An example of the Gotham web framework `Router` that shows how to route requests to different
//! handlers based on the media types listed in their `Accept` header.
use gotham::mime::{APPLICATION_JSON, TEXT_HTML, TEXT_PLAIN};
use gotham::prelude::*;
use gotham::router::route::matcher::AcceptHeaderRouteMatcher;
use gotham::router::{build_simple_router, Router};
use gotham::state::State;

const JSON_GREETING: &str = r#"{"greeting":"Hello World!"}"#;
const TEXT_GREETING: &str = "Hello World!";

fn json_greeting(state: State) -> (State, (gotham::mime::Mime, &'static str)) {
    (state, (APPLICATION_JSON, JSON_GREETING))
}

fn text_greeting(state: State) -> (State, (gotham::mime::Mime, &'static str)) {
    (state, (TEXT_PLAIN, TEXT_GREETING))
}

/// Create a `Router`
///
/// Both routes share the path `/greeting`. The `AcceptHeaderRouteMatcher` added to each route
/// decides which handler is invoked:
///
/// - `Accept: application/json` is dispatched to `json_greeting`.
/// - `Accept: text/plain` or `Accept: text/html` is dispatched to `text_greeting`.
/// - A header listing several types, like `Accept: application/xml, text/plain;q=0.9`, is
///   dispatched to the first route supporting any of them.
/// - Wildcards like `text/*` match any supported type of that kind.
/// - A missing `Accept` header or `*/*` matches every route, so the first route wins.
/// - Any other `Accept` header results in `406 Not Acceptable`.
fn router() -> Router {
    build_simple_router(|route| {
        route
            .get("/greeting")
            .add_route_matcher(AcceptHeaderRouteMatcher::new(vec![APPLICATION_JSON]))
            .to(json_greeting);

        route
            .get("/greeting")
            .add_route_matcher(AcceptHeaderRouteMatcher::new(vec![TEXT_PLAIN, TEXT_HTML]))
            .to(text_greeting);
    })
}

/// Start a server and use a `Router` to dispatch requests
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::http::header::{ACCEPT, CONTENT_TYPE};
    use gotham::http::StatusCode;
    use gotham::test::TestServer;

    fn get_greeting(accept: Option<&str>) -> gotham::test::TestResponse {
        let test_server = TestServer::new(router()).unwrap();
        let client = test_server.client();
        let mut request = client.get("http://localhost/greeting");
        if let Some(accept) = accept {
            request = request.with_header(ACCEPT, accept.parse().unwrap());
        }
        request.perform().unwrap()
    }

    fn assert_greeting(accept: Option<&str>, content_type: &str, body: &str) {
        let response = get_greeting(accept);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get(CONTENT_TYPE).unwrap(), content_type);
        assert_eq!(response.read_utf8_body().unwrap(), body);
    }

    #[test]
    fn accept_json() {
        assert_greeting(Some("application/json"), "application/json", JSON_GREETING);
    }

    #[test]
    fn accept_text_plain() {
        assert_greeting(Some("text/plain"), "text/plain", TEXT_GREETING);
    }

    #[test]
    fn accept_text_html() {
        assert_greeting(Some("text/html"), "text/plain", TEXT_GREETING);
    }

    #[test]
    fn accept_text_wildcard() {
        assert_greeting(Some("text/*"), "text/plain", TEXT_GREETING);
    }

    #[test]
    fn accept_multiple_types() {
        assert_greeting(
            Some("application/xml, text/plain;q=0.9"),
            "text/plain",
            TEXT_GREETING,
        );
    }

    #[test]
    fn accept_anything() {
        assert_greeting(Some("*/*"), "application/json", JSON_GREETING);
    }

    #[test]
    fn no_accept_header() {
        assert_greeting(None, "application/json", JSON_GREETING);
    }

    #[test]
    fn accept_unsupported_type() {
        let response = get_greeting(Some("image/png"));
        assert_eq!(response.status(), StatusCode::NOT_ACCEPTABLE);
    }
}
