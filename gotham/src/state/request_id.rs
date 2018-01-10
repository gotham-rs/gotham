//! Defines a unique id per `Request` that should be output with all logging

use hyper::header::Headers;
use uuid::Uuid;

use http::header::XRequestId;
use state::{FromState, State};

/// Holds details about the current Request that are useful for enhancing logging.
pub struct RequestId {
    val: String,
}

/// Sets a unique identifier for the request if it has not already been stored.
///
/// The unique identifier chosen depends on the the request environment:
///
/// 1. If the header X-Request-ID is provided this value is used as is;
/// 2. Alternatively creates and stores a UUID v4 value.
///
/// This method MUST be invoked by Gotham, before handing control to
/// pipelines or Handlers to ensure that a value for `RequestId` is always available.
pub fn set_request_id<'a>(state: &'a mut State) -> &'a str {
    if !state.has::<RequestId>() {
        let request_id = match Headers::borrow_from(state).get::<XRequestId>() {
            Some(ex_req_id) => {
                trace!(
                    "[{}] RequestId set from external source via X-Request-ID header",
                    ex_req_id.0.clone()
                );
                RequestId {
                    val: ex_req_id.0.clone(),
                }
            }
            None => {
                let val = Uuid::new_v4().hyphenated().to_string();
                trace!("[{}] RequestId generated internally", val);
                RequestId { val }
            }
        };
        state.put(request_id);
    };

    request_id(state)
}

/// Returns the unique Id associated with the current request.
///
/// This is very useful for logging/correlating events across distributed systems.
///
/// # Panics
///
/// Will panic if the Gotham `Router` has not already populated `State` with a value for `RequestId`
/// prior to handling control to middleware pipelines and application handlers.
pub fn request_id(state: &State) -> &str {
    match RequestId::try_borrow_from(state) {
        Some(request_id) => &request_id.val,
        None => panic!("RequestId must be populated before application code is invoked"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "RequestId must be populated before application code is invoked")]
    fn panics_before_request_id_set() {
        let state = State::new();
        request_id(&state);
    }

    #[test]
    fn uses_an_external_request_id() {
        let mut state = State::new();

        let mut headers = Headers::new();
        headers.set(XRequestId("1-2-3-4".to_string()));
        state.put(headers);

        {
            let r = set_request_id(&mut state);
            assert_eq!("1-2-3-4", r);
        };
        assert_eq!("1-2-3-4", request_id(&state));
    }

    #[test]
    fn sets_a_unique_request_id() {
        let mut state = State::new();
        state.put(Headers::new());

        {
            let r = set_request_id(&mut state);
            assert_eq!(4, Uuid::parse_str(r).unwrap().get_version_num());
        };
        assert_eq!(
            4,
            Uuid::parse_str(request_id(&state))
                .unwrap()
                .get_version_num()
        );
    }

    #[test]
    fn does_not_overwrite_existant_request_id() {
        let mut state = State::new();
        state.put(RequestId {
            val: "1-2-3-4".to_string(),
        });

        {
            set_request_id(&mut state);
        }
        assert_eq!("1-2-3-4", request_id(&state));
    }
}
