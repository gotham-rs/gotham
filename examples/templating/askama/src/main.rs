use askama::Template;
use gotham::helpers::http::response::{create_empty_response, create_response};
use gotham::hyper::{Body, Response, StatusCode};
use gotham::state::State;

pub const MESSAGE: &str = "Hello, Gotham!";

/// The index displays a message to the browser.
/// The default template directory is `$CRATE_ROOT/templates`,which is what we are using in this example
#[derive(Debug, Template)]
#[template(path = "index.html")]
pub struct Index {
    pub message: String,
}

/// Renders the `index.html` template with the `MESSAGE` constant as the message
pub fn index(state: State) -> (State, Response<Body>) {
    let tpl = Index {
        message: MESSAGE.to_string(),
    };

    // The response is either the rendered template, or a server error if something really goes wrong
    let res = match tpl.render() {
        Ok(content) => create_response(
            &state,
            StatusCode::OK,
            mime::TEXT_HTML_UTF_8,
            content.into_bytes(),
        ),
        Err(_) => create_empty_response(&state, StatusCode::INTERNAL_SERVER_ERROR),
    };

    (state, res)
}

/// Run on the normal port for Gotham examples, passing the handler as the only function for the gotham web server.
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening at {}", addr);
    gotham::start(addr, || Ok(index));
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;

    #[test]
    fn askama_template_variable_included_in_response() {
        let test_server =
            TestServer::new(|| Ok(index)).expect("Failed to launch test server with index handler");

        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .expect("Failed to send request to test server");

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .read_utf8_body()
            .expect("Failed to read utf-8 from response body");

        assert!(&body.contains(MESSAGE));
    }
}
