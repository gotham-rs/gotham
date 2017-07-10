//! Defines functionality for extending a Response after it has been dispatched by the Router
//! but before providing it to the requesting client.

use std::sync::Arc;
use std::collections::HashMap;

use futures::{future, Future};
use hyper::server::Response;
use hyper::StatusCode;

use handler::{IntoHandlerFuture, HandlerFuture};
use state::{State, request_id};

/// Application specific response extenders.
pub trait Extender {
    /// Extend the response.
    fn extend(&self, state: State, res: Response) -> Box<HandlerFuture>;
}

/// Extender that does not further alter the response body.
///
/// This is likely to only be useful in documentation or example code.
pub struct NoopExtender {}

impl Extender for NoopExtender {
    fn extend(&self, state: State, res: Response) -> Box<HandlerFuture> {
        trace!("[{}] NoopExtender invoked, does not make any changes to Response",
               request_id(&state));
        match res.body_ref() {
            Some(_) => {
                // Full implementations should not make extensions if they find this state
                trace!("[{}] found response body, no change made",
                       request_id(&state));
                future::ok((state, res)).boxed()
            }
            None => {
                // Full implementations should make extensions if they find this state
                trace!("[{}] no response body, no change made", request_id(&state));
                future::ok((state, res)).boxed()
            }
        }
    }
}

impl NoopExtender {
    /// Creates a new NoopExtender instance.
    pub fn new() -> Self {
        NoopExtender {}
    }
}

impl<F, R> Extender for F
    where F: Fn(State, Response) -> R + Send + Sync,
          R: IntoHandlerFuture
{
    fn extend(&self, state: State, res: Response) -> Box<HandlerFuture> {
        trace!("[{}] running closure based response extender",
               request_id(&state));
        self(state, res).into_handler_future()
    }
}

/// Invokes a response extender if an extender has been associated with the status code of the
/// response and the body of the response has not yet been populated.
#[derive(Clone)]
pub struct ResponseExtender {
    data: Arc<HashMap<StatusCode, Box<Extender + Send + Sync>>>,
}

/// Builds an immutable ResponseExtender
pub struct ResponseExtenderBuilder {
    data: HashMap<StatusCode, Box<Extender + Send + Sync>>,
}

impl ResponseExtenderBuilder {
    /// Creates a new ResponseExtender instance.
    pub fn new() -> Self {
        let handlers = HashMap::new();
        ResponseExtenderBuilder { data: handlers }
    }

    /// Add an Extender for responses that have been assigned this status_code.
    pub fn add(&mut self, status_code: StatusCode, responder: Box<Extender + Send + Sync>) {
        trace!(" adding response extender for {}", status_code);
        self.data.insert(status_code, responder);
    }

    /// Finalize population of error handlers for the application, ready for use by a Router
    pub fn finalize(self) -> ResponseExtender {
        ResponseExtender { data: Arc::new(self.data) }
    }
}

impl ResponseExtender {
    /// Extend the `Response` if a `ResponseExtender` has been supplied for the
    /// status code assigned to the `Response`.
    pub fn extend(&self, state: State, res: Response) -> Box<HandlerFuture> {
        match self.data.get(&res.status()) {
            Some(responder) => {
                trace!("[{}] invoking {} response extender",
                       request_id(&state),
                       res.status());
                responder.extend(state, res)
            }
            None => {
                trace!("[{}] no response extender for {}",
                       request_id(&state),
                       res.status());
                future::ok((state, res)).boxed()
            }
        }
    }
}
