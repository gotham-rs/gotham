//! Defines functionality for extending a Response

use std::panic::RefUnwindSafe;
use hyper::Response;
use state::{request_id, State};

/// Extend the Response based on current State and Response data
pub trait StaticResponseExtender: RefUnwindSafe {
    /// Extend the response.
    fn extend(&mut State, &mut Response);
}

/// Allow complex types to extend the Response based on current State and Response data
pub trait ResponseExtender: RefUnwindSafe {
    /// Extend the Response
    fn extend(&self, &mut State, &mut Response);
}

impl<F> ResponseExtender for F
where
    F: Fn(&mut State, &mut Response) + Send + Sync + RefUnwindSafe,
{
    fn extend(&self, state: &mut State, res: &mut Response) {
        trace!(
            "[{}] running closure based response extender",
            request_id(&state)
        );
        self(state, res);
    }
}

/// Extender that does not further alter the response.
///
/// This is likely to only be useful in documentation or example code.
pub struct NoopResponseExtender {}

impl NoopResponseExtender {
    /// Creates a new NoopResponseExtender instance.
    pub fn new() -> Self {
        NoopResponseExtender {}
    }
}

impl StaticResponseExtender for NoopResponseExtender {
    fn extend(state: &mut State, res: &mut Response) {
        trace!(
            "[{}] NoopResponseExtender invoked, does not make any changes to Response",
            request_id(&state)
        );
        match res.body_ref() {
            Some(_) => {
                // Full implementations should not make extensions if they find this state
                trace!(
                    "[{}] found response body, no change made",
                    request_id(&state)
                );
            }
            None => {
                // Full implementations should make extensions if they find this state
                trace!("[{}] no response body, no change made", request_id(&state));
            }
        }
    }
}

impl ResponseExtender for NoopResponseExtender {
    fn extend(&self, state: &mut State, res: &mut Response) {
        trace!(
            "[{}] NoopResponseExtender invoked on instance, does not make any changes to Response",
            request_id(&state)
        );
        match res.body_ref() {
            Some(_) => {
                // Full implementations should not make extensions if they find this state
                trace!(
                    "[{}] found response body, no change made",
                    request_id(&state)
                );
            }
            None => {
                // Full implementations should make extensions if they find this state
                trace!("[{}] no response body, no change made", request_id(&state));
            }
        }
    }
}
