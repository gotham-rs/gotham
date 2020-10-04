//! An example usage of Tera template engine working with Gotham.
#[macro_use]
extern crate lazy_static;

use gotham::state::State;
use tera::{Context, Tera};

lazy_static! {
    pub static ref TERA: Tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).expect("Parsing error(s)");
}
/// Create a `Handler` which calls the Tera static reference, renders
/// a template with a given Context, and returns the result as a String
/// to be used as Response Body
pub fn say_hello(state: State) -> (State, (mime::Mime, String)) {
    let mut context = Context::new();
    context.insert("user", "Gotham");
    let rendered = TERA.render("example.html.tera", &context).unwrap();

    (state, (mime::TEXT_HTML, rendered))
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
    use gotham::hyper::StatusCode;
    use gotham::test::TestServer;

    #[test]
    fn receive_hello_world_response() {
        let test_server = TestServer::new(|| Ok(say_hello)).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        let expected_body = concat!(
            "<!DOCTYPE html>\n<html>\n<head>\n  <meta charset=\"utf-8\" />\n",
            "  <title>Gotham Tera example</title>\n</head>\n<body>\n",
            "  <h1>Hello Gotham!</h1>\n</body>\n</html>\n"
        );
        assert_eq!(body, expected_body.as_bytes());
    }
}
