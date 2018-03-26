//! An introduction to storing and retrieving cookie data, with the Gotham
//! web framework.

extern crate gotham;
extern crate hyper;
extern crate mime;

use hyper::header::{Cookie, Headers, SetCookie};
use hyper::{Response, StatusCode};

use gotham::http::response::create_response;
use gotham::state::{FromState, State};

/// The first request will set a cookie, and subsequent requests will echo it back.
fn handler(state: State) -> (State, Response) {
    // Define a narrow scope so that state can be borrowed/moved later in the function.
    let adjective_from_cookie = {
        // Get the request headers.
        let headers: &Headers = Headers::borrow_from(&state);
        // Get the Cookie header from the request.
        let maybe_cookie = headers.get::<Cookie>();
        // Get the value of the "adjective" cookie, if set.
        maybe_cookie.and_then(|cookie| cookie.get("adjective").map(|s| s.to_owned()))
    };

    let adjective = adjective_from_cookie.unwrap_or("first time".to_owned());

    let mut response = {
        create_response(
            &state,
            StatusCode::Ok,
            Some((
                format!("Hello {} visitor\n", adjective).as_bytes().to_vec(),
                mime::TEXT_PLAIN,
            )),
        )
    };
    {
        // Make a new cookie. This is currently one of the less type-safe corners of Gotham.
        let cookie = "adjective=repeat; HttpOnly".to_owned();
        set_cookie(cookie, &mut response);
    }
    (state, response)
}

fn set_cookie(cookie: String, response: &mut Response) {
    // Get the response headers.
    let headers = response.headers_mut();
    if let Some(existing_cookies) = headers.get_mut::<SetCookie>() {
        // If some cookies are already being set (e.g. by some middleware), append to that list.
        existing_cookies.push(cookie);
        return;
    }
    // Else create a new SetCookie header.
    headers.set::<SetCookie>(SetCookie(vec![cookie]));
}

/// Start a server and use a `Router` to dispatch requests
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, || Ok(handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;
    use hyper::header::{Cookie, SetCookie};

    #[test]
    fn cookie_is_set_and_counter_increments() {
        let test_server = TestServer::new(|| Ok(handler)).unwrap();
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
        assert_eq!(
            set_cookie.get(0),
            Some(&"adjective=repeat; HttpOnly".to_owned())
        );

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], "Hello first time visitor\n".as_bytes());

        let mut cookie = Cookie::new();
        cookie.append("adjective", "repeat");

        let response = test_server
            .client()
            .get("http://localhost/")
            .with_header(cookie)
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        let body = response.read_body().unwrap();
        assert_eq!(&body[..], "Hello repeat visitor\n".as_bytes());
    }
}
