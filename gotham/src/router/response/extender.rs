//! Defines functionality for extending a Response.

use hyper::Response;
use state::{request_id, State};
use std::panic::RefUnwindSafe;

/// Extend the `Response` based on current `State` and `Response` data.
pub trait StaticResponseExtender<B>: RefUnwindSafe {
    /// Extend the response.
    fn extend(&mut State, &mut Response<B>);
}

/// Allow complex types to extend the `Response` based on current `State` and `Response` data.
pub trait ResponseExtender<B>: RefUnwindSafe {
    /// Extend the Response
    fn extend(&self, &mut State, &mut Response<B>);
}

impl<F, B> ResponseExtender<B> for F
where
    F: Fn(&mut State, &mut Response<B>) + Send + Sync + RefUnwindSafe,
{
    fn extend(&self, state: &mut State, res: &mut Response<B>) {
        trace!(
            "[{}] running closure based response extender",
            request_id(&state)
        );
        self(state, res);
    }
}

/// An extender that does not alter the response.
///
/// This is likely to only be useful in documentation or example code.
pub struct NoopResponseExtender;

impl<B> StaticResponseExtender<B> for NoopResponseExtender {
    fn extend(state: &mut State, res: &mut Response<B>) {
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

impl<B> ResponseExtender<B> for NoopResponseExtender {
    fn extend(&self, state: &mut State, res: &mut Response<B>) {
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
