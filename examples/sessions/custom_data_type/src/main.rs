//! Storing and retrieving session data with a custom data type, in a type safe
//! way, with the Gotham web framework.

use gotham::middleware::session::{NewSessionMiddleware, SessionData};
use gotham::pipeline::{new_pipeline, single_pipeline};
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State, StateData};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

// A custom type for storing data associated with the user's session.
#[derive(Clone, Deserialize, Serialize, StateData)]
struct VisitData {
    count: usize,
    last_visit: String,
}

/// Handler function for `GET` requests directed to `/`
///
/// Each request made will update state about your recent visits, and report it back.
fn get_handler(mut state: State) -> (State, String) {
    let maybe_visit_data = {
        let visit_data: &Option<VisitData> = SessionData::<Option<VisitData>>::borrow_from(&state);
        visit_data.clone()
    };

    let body = match maybe_visit_data {
        Some(ref visit_data) => format!(
            "You have visited this page {} time(s) before. Your last visit was {}.\n",
            visit_data.count, visit_data.last_visit,
        ),
        None => "You have never visited this page before.\n".to_owned(),
    };

    {
        let visit_data: &mut Option<VisitData> =
            SessionData::<Option<VisitData>>::borrow_mut_from(&mut state);
        let old_count = maybe_visit_data.map(|v| v.count).unwrap_or(0);
        let last_visit = OffsetDateTime::now_local()
            .unwrap_or_else(|_| OffsetDateTime::now_utc())
            .format(&Rfc3339)
            .expect("Failed to format time");
        *visit_data = Some(VisitData {
            count: old_count + 1,
            last_visit,
        });
    }

    (state, body)
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
    gotham::start(addr, router()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::hyper::header::{COOKIE, SET_COOKIE};
    use gotham::hyper::StatusCode;
    use gotham::test::TestServer;

    #[test]
    fn cookie_is_set_and_updates_response() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get_all(SET_COOKIE).iter().count(), 1);

        let headers = response.headers().clone();
        let cookie = headers.get(SET_COOKIE).unwrap();

        let body = response.read_body().unwrap();
        assert_eq!(
            &body[..],
            "You have never visited this page before.\n".as_bytes()
        );

        let response = test_server
            .client()
            .get("http://localhost/")
            .with_header(COOKIE, cookie.to_owned())
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        let body_string = String::from_utf8(body).unwrap();
        let expected = "You have visited this page 1 time(s) before. Your last visit was ";

        assert!(
            body_string.starts_with(expected),
            "Wrong body: {}",
            body_string
        );
    }
}
