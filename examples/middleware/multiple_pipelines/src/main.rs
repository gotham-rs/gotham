//! Provides examples of using multiple middleware pipelines for different routes.
//!
//! We'll create an app with a few different routes that need
//! different combinations of middleware.
//!
//! By default, we'll expect users to be logged in, using a session middleware
//! to track client cookies.
//! On the homepage, however, we don't want users logged in, so we'll override
//! this path to not use any middleware.
//! Our app also has some admin functionality, so we'll also override admin paths
//! to require an additional admin session.
//! Finally, our app exposes an JSON endpoint, which needs its own middleware.
extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;

#[macro_use]
extern crate serde_derive;
extern crate hyper;
extern crate mime;

use futures::future;
use hyper::header::{HeaderMap, ACCEPT};
use hyper::{Body, Response, StatusCode};

use gotham::handler::HandlerFuture;
use gotham::helpers::http::response::create_response;
use gotham::middleware::session::NewSessionMiddleware;
use gotham::middleware::Middleware;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::set::{finalize_pipeline_set, new_pipeline_set};
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};

/// A simple struct to represent our default session data.
#[derive(Default, Serialize, Deserialize)]
struct Session;

/// We're going to define a separate struct for
/// admin session data, to apply on the admin routes
/// in addition to the default session data.
#[derive(Default, Serialize, Deserialize)]
struct AdminSession;

/// As well as the session middlewares defined above,
/// our example will provide an API route that uses
/// its own middleware.
#[derive(Clone, NewMiddleware)]
pub struct ApiMiddleware;

/// Our example API middleware will reject any requests that
/// don't accept JSON as the response content type.
impl Middleware for ApiMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture> + 'static,
    {
        let accepts = HeaderMap::borrow_from(&state)
            .get(ACCEPT)
            .map(|ct| ct.to_str().unwrap().to_string());

        match accepts {
            None => chain(state),
            Some(ref s) if s == "application/json" || s == "*/*" => chain(state),
            _ => {
                let body = r#"{"message":"Invalid accept type"}"#;
                let response = create_response(
                    &state,
                    StatusCode::BAD_REQUEST,
                    mime::APPLICATION_JSON,
                    body,
                );
                Box::new(future::ok((state, response)))
            }
        }
    }
}

/// A basic handler for our routes that respond with HTML.
pub fn html_handler(state: State) -> (State, Response<Body>) {
    let doc = "
    <html>
    <head>Gotham</head>
    <body>
        <p>A flexible web framework that promotes stability, safety, security and speed.</p>
    </body>
    </html>
    ";
    let res = create_response(&state, StatusCode::OK, mime::TEXT_HTML, doc);
    (state, res)
}

/// And a handler for our API that returns JSON.
pub fn api_handler(state: State) -> (State, Response<Body>) {
    let doc = r#"{
        "Gotham": "A flexible web framework that promotes stability, safety, security and speed."
    }"#;
    let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, doc);
    (state, res)
}

/// Next, we define our router to connect the middlewares and routes.
fn router() -> Router {
    // The steps to build more advanced pipelines, as shown here, are a little
    // cumbersome - this is something we'd like review in the future.
    // We start by creating a (editable) pipeline set,
    // allowing us to add multiple pipelines of middleware.
    let pipelines = new_pipeline_set();
    // We can then create a pipeline - here with only one middleware for
    // our default sessions, but of course any number could be added.
    // This creates a default in-memory session store for our `Session` struct.
    let (pipelines, default) = pipelines.add(
        new_pipeline()
            .add(NewSessionMiddleware::default().with_session_type::<Session>())
            .build(),
    );
    // Similarly, we want a separate session middleware for our admin sessions.
    let (pipelines, extended) = pipelines.add(
        new_pipeline()
            .add(NewSessionMiddleware::default().with_session_type::<AdminSession>())
            .build(),
    );
    // Before we can use a pipeline set, we have to 'freeze' it. This returns a
    // version of it that is immutable, and can be used by our router to handle requests.
    let pipeline_set = finalize_pipeline_set(pipelines);

    // Next, we can chain together pipelines - in this case our default chain only
    // has our default pipeline, but our extended chain joins our extended 'admin'
    // pipeline onto the default chain - so middlewares in both pipelines will be
    // called in that chain.
    let default_chain = (default, ());
    let extended_chain = (extended, default_chain);

    // Finally, for our API, we can give it its own router and pipeline
    // altogether, which will be nested within our main router below.
    let (api_chain, api) = single_pipeline(new_pipeline().add(ApiMiddleware).build());
    let api_router = build_router(api_chain, api, |route| {
        route.get("/").to(api_handler);
    });

    // We build our router - giving it our default chain to apply to all routes
    // by default, unless overridden.
    build_router(default_chain, pipeline_set, |route| {
        // Requests dispatched to the '/account' route will only invoke one session
        // middleware which is the default behavior.
        route.get("/account").to(html_handler);

        // We override the base path to use an empty set of pipelines, skipping the session
        // middlewares.
        route.with_pipeline_chain((), |route| {
            route.get("/").to(html_handler);
        });

        // Requests for the admin handler will additionally invoke the admin session
        // middleware - as defined by our `extended_chain`.
        route.with_pipeline_chain(extended_chain, |route| {
            route.get("/admin").to(html_handler);
        });

        // Finally, we can mount our API router, telling it not to
        // pass down the default pipelines of the main router.
        // This achieves a similar result to the examples above, but
        // for more deeply nested routes, can be a neater way to
        // break things up. You can also use a `single_pipeline` per
        // router - avoiding the set up of a pipeline set as we did above.
        route
            .delegate_without_pipelines("/api")
            .to_router(api_router);
    })
}

/// Start a server and use a `Router` to dispatch requests
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;
    use hyper::header::HeaderValue;

    #[test]
    fn no_middleware_on_base_path() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // On the base path we override the default middleware to provide an empty set,
        // so expect no session cookie to be set.
        assert!(response.headers().get("set-cookie").is_none());
    }

    #[test]
    fn single_session_middleware_on_account_path() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/account")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // We'll get an iterator over the session cookie headers.
        let mut headers = response.headers().get_all("set-cookie").iter();
        // Let's check that we have only one cookie being set.
        assert!(headers.next().is_some());
        assert!(headers.next().is_none());
    }

    #[test]
    fn admin_session_middleware_on_admin_path() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/admin")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Again, we'll get an iterator over the session cookie headers.
        let mut headers = response.headers().get_all("set-cookie").iter();
        // This time let's check that we have one cookie being set for the
        // default session middleware.
        assert!(headers.next().is_some());
        // And another cookie being set for the admin session middleware.
        assert!(headers.next().is_some());
        // And that these are the only two.
        assert!(headers.next().is_none());
    }

    #[test]
    fn api_accepts_json_content_type() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/api")
            .with_header("Accept", HeaderValue::from_static("application/json"))
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn api_does_not_accept_xml_content_type() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/api")
            .with_header("Accept", HeaderValue::from_static("text/xml"))
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
