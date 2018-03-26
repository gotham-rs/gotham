//! An introduction to storing and retrieving session data, in a type safe way, with the Gotham
//! web framework.

extern crate gotham;
extern crate hyper;
extern crate mime;

use hyper::{Response, StatusCode};

use gotham::http::response::create_response;
use gotham::middleware::session::{NewSessionMiddleware, SessionData};
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::Router;
use gotham::router::builder::*;
use gotham::state::{FromState, State};

/// Handler function for `GET` requests directed to `/`
///
/// Each request made will increment a counter of requests which have been made,
/// and tell you how many times you've visited the page.
fn get_handler(mut state: State) -> (State, Response) {
    // Define a narrow scope so that state can be borrowed/moved later in the function.
    let visits = {
        // Borrow a reference to the usize stored for the session (keyed by a cookie) from state.
        // We don't need to worry about the underlying cookie mechanics, we just ask for our usize.
        let visits: &usize = SessionData::<usize>::borrow_from(&state);
        *visits
    };

    let res = {
        create_response(
            &state,
            StatusCode::Ok,
            Some((
                format!("You have visited this page {} time(s) before\n", visits)
                    .as_bytes()
                    .to_vec(),
                mime::TEXT_PLAIN,
            )),
        )
    };
    {
        // Mutably borrow the usize, so we can increment it.
        let visits: &mut usize = SessionData::<usize>::borrow_mut_from(&mut state);
        *visits += 1;
    }
    (state, res)
}

/// Create a `Router`
fn router() -> Router {
    // Install middleware which handles session creation before, and updating after, our handler is
    // called.
    // The default NewSessionMiddleware stores session data in an in-memory map, which means that
    // server restarts will throw the data away, but it can be customized as needed.
    let middleware = NewSessionMiddleware::default()
        // Configure the type of data which we want to store in the session.
        // See the custom_data_type example for storing more complex data.
        .with_session_type::<usize>()
        // By default, the cookies used are only sent over secure connections. For our test server,
        // we don't set up an HTTPS certificate, so we allow the cookies to be sent over insecure
        // connections. This should not be done in real applications.
        .insecure();
    let (chain, pipelines) = single_pipeline(new_pipeline().add(middleware).build());
    build_router(chain, pipelines, |route| {
        route.get("/").to(get_handler);
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
    use hyper::header::{Cookie, SetCookie};
    use std::borrow::Cow;

    #[test]
    fn cookie_is_set_and_counter_increments() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let set_cookie: Vec<String> = {
            let cookie_header = response.headers().get::<SetCookie>();
            assert!(cookie_header.is_some());
            cookie_header.unwrap().0.clone()
        };
        assert!(set_cookie.len() == 1);

        let body = response.read_body().unwrap();
        assert_eq!(
            &body[..],
            "You have visited this page 0 time(s) before\n".as_bytes()
        );

        let cookie = {
            let mut cookie = Cookie::new();

            let only_cookie: String = set_cookie.get(0).unwrap().clone();
            let cookie_components: Vec<_> = only_cookie.split(";").collect();
            let cookie_str_parts: Vec<_> = cookie_components.get(0).unwrap().split("=").collect();
            cookie.append(
                Cow::Owned(cookie_str_parts.get(0).unwrap().to_string()),
                Cow::Owned(cookie_str_parts.get(1).unwrap().to_string()),
            );
            cookie
        };

        let response = test_server
            .client()
            .get("http://localhost/")
            .with_header(cookie)
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        let body = response.read_body().unwrap();
        assert_eq!(
            &body[..],
            "You have visited this page 1 time(s) before\n".as_bytes()
        );
    }
}
