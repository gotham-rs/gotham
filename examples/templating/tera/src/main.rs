//! An example usage of Tera template engine working with Gotham.

extern crate gotham;
extern crate hyper;
extern crate mime;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate tera;

use hyper::{Response, StatusCode};

use gotham::helpers::http::response::create_response;
use gotham::state::State;
use tera::{Context, Tera};

/// Assuming the Rust file is at the same level as the templates folder
/// we can get a Tera instance that way:
lazy_static! {
    pub static ref TERA: Tera = {
        compile_templates!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
    };
}

/// Create a `Handler` which calls the Tera static reference, renders
/// a template with a given Context, and returns the result as a String
/// to be used as Response Body
pub fn say_hello(state: State) -> (State, Response) {
    let mut context = Context::new();
    context.add("user", "Gotham");
    let rendered = TERA.render("example.html.tera", &context).unwrap();

    let res = create_response(
        &state,
        StatusCode::Ok,
        Some((rendered.into_bytes(), mime::TEXT_HTML)),
    );

    (state, res)
}

/// Start a server and call the `Handler` we've defined above for each `Request` we receive.
pub fn main() {
    println!("{:?}", std::env::current_exe());
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, || Ok(say_hello))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;

    #[test]
    fn receive_hello_world_response() {
        let test_server = TestServer::new(|| Ok(say_hello)).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        let expected_body = concat!("<!DOCTYPE html>\n<html>\n<head>\n  <meta charset=\"utf-8\" />\n"
                                    ,"  <title>Gotham Tera example</title>\n</head>\n<body>\n"
                                    ,"  <h1>Hello Gotham!</h1>\n</body>\n</html>\n");
        assert_eq!(body, expected_body.as_bytes());
    }
}
