use super::create_response;
use crate::state::State;
use askama::Template;
use hyper::{Body, Response, StatusCode};

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
