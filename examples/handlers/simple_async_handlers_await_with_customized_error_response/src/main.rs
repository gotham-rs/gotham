//! A basic example showing the request components

use gotham::handler::MapHandlerErrorToCustomizedResponse;
use gotham::handler::MapHandlerErrorWithCustomizedResponse;
use gotham::handler::{HandlerError, HandlerResult, IntoResponse};
use gotham::helpers::http::response::create_empty_response;
use gotham::hyper::header::CONTENT_TYPE;
use gotham::hyper::{HeaderMap, StatusCode};
use gotham::router::builder::DefineSingleRoute;
use gotham::router::builder::{build_simple_router, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};

pub async fn map_err_with_customized_response(
    state: &mut State,
) -> Result<impl IntoResponse, HandlerError> {
    // here, we just simulate an err.
    let _io_error = Err(std::io::Error::last_os_error())
        .map_err_with_customized_response(
            state,
            |_state| {
                // an error occurs, but still sending **OK** to client
                (StatusCode::OK, mime::TEXT_PLAIN_UTF_8, "Customized response by the last os error (Intentionally return 200 even error occurs)")
            },
        )?;
    Ok(create_empty_response(&state, StatusCode::OK))
}

pub async fn map_err_with_customized_response_by_returning_json(
    state: &mut State,
) -> Result<impl IntoResponse, HandlerError> {
    // here, we just simulate an err.
    let _io_error =
        Err(std::io::Error::last_os_error()).map_err_with_customized_response(state, |state| {
            // do something with state ...
            let _ = state;
            (
                StatusCode::SERVICE_UNAVAILABLE,
                mime::APPLICATION_JSON,
                r##" {"customized_error_to_return_json_response": "yes"} "##,
            )
        })?;
    Ok(create_empty_response(&state, StatusCode::OK))
}

// error response will return json or plain by request content-type.
pub async fn map_err_to_customized_response(
    state: &mut State,
) -> Result<impl IntoResponse, HandlerError> {
    // here, we just simulate an err.
    let _io_error = Err(std::io::Error::last_os_error()).map_err_to_customized_response(
        state,
        |err, state| {
            // print the error
            println!("error occurs: {}", err);
            let content_type = HeaderMap::borrow_from(&state)
                .get(CONTENT_TYPE)
                .map(|x| x.to_str().unwrap())
                .unwrap_or("text/plain");
            if content_type.contains("json") {
                // an error occurs, but still we want to send OK to client
                let customized_response = (
                    StatusCode::SERVICE_UNAVAILABLE,
                    mime::APPLICATION_JSON,
                    r##" {"customized_error_to_return_json_response": "yes", "last_os_error": "##
                        .to_owned()
                        + &format!("{:?}", err.to_string())
                        + "}  ",
                );
                (err, customized_response)
            } else {
                let customized_response = (
                    StatusCode::SERVICE_UNAVAILABLE,
                    mime::TEXT_PLAIN_UTF_8,
                    format!(
                        "customized_error_to_return_json_response: yes, last_os_error: {}",
                        err
                    ),
                );
                (err, customized_response)
            }
        },
    )?;
    Ok(create_empty_response(&state, StatusCode::OK))
}

async fn use_handler_result_style(state: State) -> HandlerResult {
    println!("{}", "i5");
    let e = Err(std::io::Error::last_os_error())
        .map_err_with_customized_response(&state, |state| {
            // do something with state
            let _ = state;
            (
                StatusCode::SERVICE_UNAVAILABLE,
                mime::APPLICATION_JSON,
                r##" {"a": 1, "b": 2} "##,
            )
        })
        .map_err(|e| (state, e))?;

    return Ok(e);
}

/// Create a `Router`.
fn router() -> Router {
    build_simple_router(|route| {
        route
            .get("/map_err_with_customized_response")
            .to_async_borrowing(map_err_with_customized_response);
        route
            .get("/map_err_with_customized_response_by_returning_json")
            .to_async_borrowing(map_err_with_customized_response_by_returning_json);
        route
            .get("/map_err_to_customized_response")
            .to_async_borrowing(map_err_to_customized_response);
        route
            .get("/use_handler_result_style")
            .to_async(use_handler_result_style);
    })
}

/// Start a server and use a `Router` to dispatch requests.
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}

#[cfg(test)]
mod tests {
    use gotham::test::TestServer;

    use super::*;

    fn assert_returned_status_ok(url_str: &str, expected_status: StatusCode) {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server.client().get(url_str).perform().unwrap();

        assert_eq!(response.status(), expected_status);
        // assert_eq!(
        //     &String::from_utf8(response.read_body().unwrap()).unwrap(),
        //     expected_response
        // );
    }

    fn assert_returns_ok(url_str: &str, expected_response: &str) {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server.client().get(url_str).perform().unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            &String::from_utf8(response.read_body().unwrap()).unwrap(),
            expected_response
        );
    }

    #[test]
    fn test_map_err_with_customized_response() {
        assert_returns_ok(
            "http://localhost/map_err_with_customized_response",
            "Customized response by the last os error (Intentionally return 200 even error occurs)",
        );
    }

    #[test]
    fn test_map_err_with_customized_response_by_returning_json() {
        assert_returned_status_ok(
            "http://localhost/map_err_with_customized_response_by_returning_json",
            StatusCode::SERVICE_UNAVAILABLE,
        );
    }

    #[test]
    fn test_map_err_to_customized_response() {
        assert_returned_status_ok(
            "http://localhost/map_err_to_customized_response",
            StatusCode::SERVICE_UNAVAILABLE,
        );
    }

    #[test]
    fn test_use_handler_result_style() {
        assert_returned_status_ok(
            "http://localhost/use_handler_result_style",
            StatusCode::SERVICE_UNAVAILABLE,
        );
    }
}
