//! Storing and retrieving session data with a custom data type, in a type safe
//! way, with the Gotham web framework.

extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate time;

use hyper::{Response, StatusCode};

use gotham::protocol::response::create_response;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::Router;
use gotham::router::builder::*;
use gotham::state::{FromState, State};
use gotham::middleware::session::{NewSessionMiddleware, SessionData};

// A custom type for storing data associated with the user's session.
#[derive(Clone, Deserialize, Serialize, StateData)]
struct VisitData {
    count: usize,
    last_visit: String,
}

/// Handler function for `GET` requests directed to `/`
///
/// Each request made will update state about your recent visits, and report it back.
fn get_handler(mut state: State) -> (State, Response) {
    let maybe_visit_data = {
        let visit_data: &Option<VisitData> = SessionData::<Option<VisitData>>::borrow_from(&state);
        visit_data.clone()
    };

    let body = match &maybe_visit_data {
        &Some(ref visit_data) => format!(
            "You have visited this page {} time(s) before. Your last visit was {}.\n",
            visit_data.count, visit_data.last_visit,
        ),
        &None => "You have never visited this page before.\n".to_owned(),
    };
    let res = {
        create_response(
            &state,
            StatusCode::Ok,
            Some((body.as_bytes().to_vec(), mime::TEXT_PLAIN)),
        )
    };
    {
        let visit_data: &mut Option<VisitData> =
            SessionData::<Option<VisitData>>::borrow_mut_from(&mut state);
        let old_count = maybe_visit_data.map(|v| v.count).unwrap_or(0);
        *visit_data = Some(VisitData {
            count: old_count + 1,
            last_visit: format!("{}", time::now().rfc3339()),
        });
    }
    (state, res)
}

/// Create a `Router`
fn router() -> Router {
    let middleware = NewSessionMiddleware::default()
        .with_session_type::<Option<VisitData>>()
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
    fn cookie_is_set_and_updates_response() {
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
            "You have never visited this page before.\n".as_bytes()
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
        let body_string = String::from_utf8(body).unwrap();
        assert!(
            body_string
                .starts_with("You have visited this page 1 time(s) before. Your last visit was ",),
            "Wrong body: {}",
            body_string
        );
    }
}
