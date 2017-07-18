//! Defines functionality for extending a Response

use hyper::server::Response;
use state::{State, request_id};

/// Application specific response extenders.
pub trait ResponseExtender {
    /// Extend the response.
    fn extend(&self, state: &mut State, res: &mut Response);
}

impl<F> ResponseExtender for F
    where F: Fn(&mut State, &mut Response) + Send + Sync
{
    fn extend(&self, state: &mut State, res: &mut Response) {
        trace!("[{}] running closure based response extender",
               request_id(&state));
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

impl ResponseExtender for NoopResponseExtender {
    fn extend(&self, state: &mut State, res: &mut Response) {
        trace!("[{}] NoopResponseExtender invoked, does not make any changes to Response",
               request_id(&state));
        match res.body_ref() {
            Some(_) => {
                // Full implementations should not make extensions if they find this state
                trace!("[{}] found response body, no change made",
                       request_id(&state));
            }
            None => {
                // Full implementations should make extensions if they find this state
                trace!("[{}] no response body, no change made", request_id(&state));
            }
        }
    }
}
