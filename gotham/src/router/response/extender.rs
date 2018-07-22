//! Defines functionality for extending a Response.

use hyper::{body::Payload, Body, Response};
use state::{request_id, State};
use std::panic::RefUnwindSafe;

/// Extend the `Response` based on current `State` and `Response` data.
pub trait StaticResponseExtender: RefUnwindSafe {
    /// The type of the response body. Almost always `hyper::Body`.
    type ResBody: Payload;

    /// Extend the response.
    fn extend(&mut State, &mut Response<Self::ResBody>);
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

impl StaticResponseExtender for NoopResponseExtender {
    type ResBody = Body;

    fn extend(state: &mut State, _res: &mut Response<Body>) {
        trace!(
            "[{}] NoopResponseExtender invoked, does not make any changes to Response",
            request_id(&state)
        );
        trace!("[{}] no response body, no change made", request_id(&state));
    }
}

impl ResponseExtender<Body> for NoopResponseExtender {
    fn extend(&self, state: &mut State, _res: &mut Response<Body>) {
        trace!(
            "[{}] NoopResponseExtender invoked on instance, does not make any changes to Response",
            request_id(&state)
        );
        trace!("[{}] no response body, no change made", request_id(&state));
    }
}
