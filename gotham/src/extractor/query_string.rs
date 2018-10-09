use hyper::{body::Payload, Body, Response};
use serde::{Deserialize, Deserializer};

use router::response::extender::StaticResponseExtender;
use state::{State, StateData};

/// Defines a binding for storing the query parameters from the `Request` URI in `State`. On
/// failure the `StaticResponseExtender` implementation extends the `Response` to indicate why the
/// extraction process failed.
///
/// This trait is automatically implemented when the struct implements the `Deserialize`,
/// `StateData` and `StaticResponseExtender` traits. These traits can be derived, or implemented
/// manually for greater control.
///
/// The default behaviour given by deriving all three traits will use the automatically derived
/// behaviour from Serde, and result in a `400 Bad Request` HTTP response if the query string is
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
/// struct MyQueryParams {
///     x: i32,
///     y: MyEnum,
/// }
///
/// #[derive(Deserialize, Clone, Copy, Debug)]
/// #[serde(rename_all = "kebab-case")]
/// enum MyEnum {
///     A,
///     B,
///     C,
/// }
///
/// fn handler(state: State) -> (State, Response<Body>) {
///     let &MyQueryParams { x, y } = MyQueryParams::borrow_from(&state);
///     let body = format!("x = {}, y = {:?}", x, y);
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
///             .get("/test")
///             .with_query_string_extractor::<MyQueryParams>()
///             .to(handler);
///     })
/// }
/// #
/// # fn main() {
/// #   let test_server = TestServer::new(router()).unwrap();
/// #   let response = test_server
/// #       .client()
/// #       .get("http://example.com/test?x=15&y=b")
/// #       .perform()
/// #       .unwrap();
/// #   assert_eq!(response.status(), StatusCode::OK);
/// #   let body = response.read_utf8_body().unwrap();
/// #   assert_eq!(body, "x = 15, y = B");
/// # }
pub trait QueryStringExtractor<B>:
    for<'de> Deserialize<'de> + StaticResponseExtender<ResBody = B> + StateData
where
    B: Payload,
{
}

impl<T, B> QueryStringExtractor<B> for T
where
    B: Payload,
    for<'de> T: Deserialize<'de> + StaticResponseExtender<ResBody = B> + StateData,
{}

/// A `QueryStringExtractor` that does not extract/store any data.
///
/// This is the default `QueryStringExtractor` which is applied to a route when no other
/// `QueryStringExtractor` is provided. It ignores any query parameters, and always succeeds during
/// deserialization.
#[derive(Debug)]
pub struct NoopQueryStringExtractor;

// This doesn't get derived correctly if we just `#[derive(Deserialize)]` above, because the
// Deserializer expects to _ignore_ a value, not just do nothing. By filling in the impl ourselves,
// we can explicitly do nothing.
impl<'de> Deserialize<'de> for NoopQueryStringExtractor {
    fn deserialize<D>(_de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(NoopQueryStringExtractor)
    }
}

impl StateData for NoopQueryStringExtractor {}

impl StaticResponseExtender for NoopQueryStringExtractor {
    type ResBody = Body;
    fn extend(_state: &mut State, _res: &mut Response<Body>) {}
}
