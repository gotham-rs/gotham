use super::create_response;
use crate::state::State;
use askama::Template;
use hyper::{Body, Response, StatusCode};

/// Renders an `askama::Template` data instance and returns a `Result<Response<Body>, askama::Error>`
///
/// ```
/// #[macro_use] extern crate askama;
/// use gotham::state::State;
/// use hyper::{Response, Body};
/// use gotham::helpers::http::response::try_create_html_response;
/// #[derive(Template)]
/// #[template(source = "<p>Hello, {{ name }}</p>")]
/// struct Greeter {
///     name: &'static str,
/// }
///
/// fn handler(state: State) -> (State, Response<Body>) {
///     let greeting = Greeter { name: "World" };
///     let res = try_create_html_response(&state, &greeting).expect("Failed to render html response");
///
///     (state, res)
/// }
/// #
/// # fn main() {
/// #     use gotham::test::TestServer;
/// #     use hyper::StatusCode;
/// #     use hyper::header::{CONTENT_TYPE, CONTENT_LENGTH};
/// #     let test_server = TestServer::new(|| Ok(handler)).unwrap();
/// #     let response = test_server
/// #         .client()
/// #         .get("http://example.com/")
/// #         .perform()
/// #         .unwrap();
/// #
/// #     assert_eq!(response.status(), StatusCode::OK);
/// #
/// #     assert_eq!(
/// #         *response.headers().get(CONTENT_TYPE).unwrap(),
/// #         mime::TEXT_HTML_UTF_8.to_string()
/// #     );
/// #
/// #     assert_eq!(
/// #         *response.headers().get(CONTENT_LENGTH).unwrap(),
/// #         format!("{}", 19)
/// #     );
/// # }
/// ```
pub fn try_create_html_response(
    state: &State,
    template: &impl Template,
) -> Result<Response<Body>, askama::Error> {
    let rendered = template.render()?;
    Ok(create_response(
        state,
        StatusCode::OK,
        mime::TEXT_HTML_UTF_8,
        rendered,
    ))
}
