use hyper::{body::Payload, Body, Response};
use serde::{Deserialize, Deserializer};

use router::response::extender::StaticResponseExtender;
use state::{State, StateData};

/// Defines a binding for storing the dynamic segments of the `Request` path in `State`. On failure
/// the `StaticResponseExtender` implementation extends the `Response` to indicate why the
/// extraction process failed.
///
/// This trait is automatically implemented when the struct implements the `Deserialize`,
/// `StateData` and `StaticResponseExtender` traits. These traits can be derived, or implemented
/// manually for greater control.
///
/// The default behaviour given by deriving all three traits will use the automatically derived
/// behaviour from Serde, and result in a `400 Bad Request` HTTP response if the path segments are
/// not able to be deserialized.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate gotham_derive;
/// # extern crate hyper;
/// # extern crate mime;
/// # extern crate serde;
/// # #[macro_use]
/// # extern crate serde_derive;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use gotham::state::{FromState, State};
/// # use gotham::helpers::http::response::create_response;
/// # use gotham::router::Router;
/// # use gotham::router::builder::*;
/// # use gotham::test::TestServer;
/// #
/// #[derive(Deserialize, StateData, StaticResponseExtender)]
/// struct MyPathParams {
///     id: i32,
///     slug: String,
/// }
///
/// fn handler(mut state: State) -> (State, Response<Body>) {
///     let MyPathParams { id, slug } = MyPathParams::take_from(&mut state);
///     let body = format!("id = {}, slug = {}", id, slug);
///
///     let response = create_response(
///         &state,
///         StatusCode::OK,
///         mime::TEXT_PLAIN,
///         body,
///     );
///
///     (state, response)
/// }
///
/// fn router() -> Router {
///     build_simple_router(|route| {
///         route
///             .get("/article/:id/:slug")
///             .with_path_extractor::<MyPathParams>()
///             .to(handler);
///     })
/// }
/// #
/// # fn main() {
/// #   let test_server = TestServer::new(router()).unwrap();
/// #   let response = test_server
/// #       .client()
/// #       .get("http://example.com/article/1551/ten-reasons-serde-is-amazing")
/// #       .perform()
/// #       .unwrap();
/// #   assert_eq!(response.status(), StatusCode::OK);
/// #   let body = response.read_utf8_body().unwrap();
/// #   assert_eq!(body, "id = 1551, slug = ten-reasons-serde-is-amazing");
/// # }
pub trait PathExtractor<B>:
    for<'de> Deserialize<'de> + StaticResponseExtender<ResBody = B> + StateData
where
    B: Payload,
{
}

impl<T, B> PathExtractor<B> for T
where
    B: Payload,
    for<'de> T: Deserialize<'de> + StaticResponseExtender<ResBody = B> + StateData,
{}

/// A `PathExtractor` that does not extract/store any data from the `Request` path.
///
/// This is the default `PathExtractor` which is applied to a route when no other `PathExtractor`
/// is provided. It ignores any dynamic path segments, and always succeeds during deserialization.
pub struct NoopPathExtractor;

// This doesn't get derived correctly if we just `#[derive(Deserialize)]` above, because the
// Deserializer expects to _ignore_ a value, not just do nothing. By filling in the impl ourselves,
// we can explicitly do nothing.
impl<'de> Deserialize<'de> for NoopPathExtractor {
    fn deserialize<D>(_de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(NoopPathExtractor)
    }
}

impl StateData for NoopPathExtractor {}

impl StaticResponseExtender for NoopPathExtractor {
    type ResBody = Body;
    fn extend(_state: &mut State, _res: &mut Response<Body>) {}
}
