//! An example of decoding multipart form requests
use futures::prelude::*;
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_response;
use gotham::hyper::header::CONTENT_TYPE;
use gotham::hyper::{body, Body, HeaderMap, StatusCode};
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};
use multipart::server::Multipart;
use std::io::Cursor;
use std::io::Read;
use std::pin::Pin;

/// Extracts the elements of the POST request and responds with the form keys and values
fn form_handler(mut state: State) -> Pin<Box<HandlerFuture>> {
    const BOUNDARY: &str = "boundary=";
    let header_map = HeaderMap::take_from(&mut state);
    let boundary = header_map
        .get(CONTENT_TYPE)
        .and_then(|ct| {
            let ct = ct.to_str().ok()?;
            let idx = ct.find(BOUNDARY)?;
            Some(ct[idx + BOUNDARY.len()..].to_string())
        })
        .unwrap();

    body::to_bytes(Body::take_from(&mut state))
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let mut m = Multipart::with_body(Cursor::new(valid_body), boundary);
                match m.read_entry() {
                    Ok(Some(mut field)) => {
                        let mut data = Vec::new();
                        field.data.read_to_end(&mut data).expect("can't read");
                        let res_result = String::from_utf8(data);
                        let res_body;
                        match res_result {
                            Ok(r) => res_body = r.to_string(),
                            Err(e) => res_body = format!("{:?}", e),
                        }
                        let res =
                            create_response(&state, StatusCode::OK, mime::TEXT_PLAIN, res_body);
                        future::ok((state, res))
                    }
                    Ok(None) => {
                        let res = create_response(
                            &state,
                            StatusCode::OK,
                            mime::TEXT_PLAIN,
                            "can't read".to_string(),
                        );
                        future::ok((state, res))
                    }
                    Err(e) => {
                        let res = create_response(
                            &state,
                            StatusCode::OK,
                            mime::TEXT_PLAIN,
                            format!("{:?}", e),
                        );
                        future::ok((state, res))
                    }
                }
            }
            Err(e) => future::err((state, e.into_handler_error())),
        })
        .boxed()
}

/// Create a `Router`
fn router() -> Router {
    build_simple_router(|route| {
        route.post("/").to(form_handler);
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
    use gotham::hyper::header::HeaderValue;
    use gotham::test::TestServer;

    #[test]
    fn form_request() {
        let boundary = "--abcdef1234--";
        let body = format!(
            "--{0}\r\n\
             content-disposition: form-data; name=\"foo\"\r\n\r\n\
             bar\r\n\
             --{0}--\r\n",
            boundary
        );

        let test_server = TestServer::new(router()).unwrap();
        let client = test_server.client();
        let mut request = client.post("http://localhost", body, mime::MULTIPART_FORM_DATA);

        let content_type_string = format!("multipart/form-data; boundary={}", boundary);
        request.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_str(content_type_string.as_str()).unwrap(),
        );
        let response = request.perform().unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.read_body().unwrap();
        let r = String::from_utf8(body).unwrap();
        assert_eq!(r, "bar");
    }
}
