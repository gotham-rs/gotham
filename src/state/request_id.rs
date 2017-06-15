//! Defines a unique id per `Request` that should be output with all logging

use hyper::Request;
use uuid::Uuid;

use state::{StateData, State};

struct RequestId {
    val: String,
}

impl StateData for RequestId {}

/// Creates and stores a UUID v4 value that uniquely* identifies every request entering the system.
///
/// This method MUST be invoked by Gotham, specifically by the `Router`, before handing control to
/// pipelines or Handlers to ensure that a value for `RequestId` is always available.
pub fn set_request_id<'a>(state: &'a mut State, _req: &Request) -> &'a str {
    if !state.has::<RequestId>() {
        let val = Uuid::new_v4().hyphenated().to_string();
        let request_id = RequestId { val };
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
    match state.borrow::<RequestId>() {
        Some(request_id) => &request_id.val,
        None => panic!("RequestId must be populated before application code is invoked"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;
    use hyper::{Method, Uri};

    #[test]
    #[should_panic(expected = "RequestId must be populated before application code is invoked")]
    fn panics_before_request_id_set() {
        let state = State::new();
        request_id(&state);
    }

    #[test]
    fn sets_a_unique_request_id() {
        let mut state = State::new();
        let req = Request::new(Method::Get,
                               Uri::from_str("https://test.gotham.rs").unwrap());

        {
            let r = set_request_id(&mut state, &req);
            assert_eq!(4, Uuid::parse_str(r).unwrap().get_version_num());
        };
        assert_eq!(4,
                   Uuid::parse_str(request_id(&state))
                       .unwrap()
                       .get_version_num());
    }
}
