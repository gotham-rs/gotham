//! Defines functionality for extending a Response after it has been dispatched by the Router
//! but before providing it to the requesting client.

use futures::{future, Future};
use std::sync::Arc;
use std::collections::HashMap;
use hyper::server::Response;
use hyper::status::StatusCode;
use hyper::Uri;

use handler::HandlerFuture;
use state::State;

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
        // TODO: Request hyper API ext to determine if body is present or not or some other
        // mechanism to signal that the response is 'committed'. If so, bail.
        //
        // https://github.com/hyperium/hyper/issues/1216

        future::ok((state, res)).boxed()
    }
}

impl NoopExtender {
    /// Creates a new NoopExtender instance.
    pub fn new() -> Self {
        NoopExtender {}
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
            Some(responder) => responder.extend(state, res),
            None => future::ok((state, res)).boxed(),
        }
    }
}
