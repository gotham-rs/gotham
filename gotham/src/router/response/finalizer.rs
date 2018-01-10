//! Defines functionality for finalizing a Response after all Pipelines, Middlewares, Handlers
//! and interal Extenders have completed.

use std::sync::Arc;
use std::collections::HashMap;

use futures::future;
use hyper::{Response, StatusCode};

use handler::HandlerFuture;
use state::{request_id, State};

use router::response::extender::ResponseExtender;

/// Invokes a response finalizer if a finalizer has been associated with the status code of the
/// response and the body of the response has not yet been populated.
#[derive(Clone)]
pub struct ResponseFinalizer {
    data: Arc<HashMap<StatusCode, Box<ResponseExtender + Send + Sync>>>,
}

/// Builds an immutable ResponseFinalizer
pub struct ResponseFinalizerBuilder {
    data: HashMap<StatusCode, Box<ResponseExtender + Send + Sync>>,
}

impl ResponseFinalizerBuilder {
    /// Creates a new ResponseFinalizer instance.
    pub fn new() -> Self {
        let handlers = HashMap::new();
        ResponseFinalizerBuilder { data: handlers }
    }

    /// Add an Finalizer for responses that have been assigned this status_code.
    pub fn add(&mut self, status_code: StatusCode, extender: Box<ResponseExtender + Send + Sync>) {
        trace!(" adding response extender for {}", status_code);
        self.data.insert(status_code, extender);
    }

    /// Finalize population of error handlers for the application, ready for use by a Router
    pub fn finalize(self) -> ResponseFinalizer {
        ResponseFinalizer {
            data: Arc::new(self.data),
        }
    }
}

impl ResponseFinalizer {
    /// Finalize the `Response` if a `ResponseFinalizer` has been supplied for the
    /// status code assigned to the `Response`.
    pub fn finalize(&self, mut state: State, mut res: Response) -> Box<HandlerFuture> {
        match self.data.get(&res.status()) {
            Some(extender) => {
                trace!(
                    "[{}] invoking {} response extender",
                    request_id(&state),
                    res.status()
                );
                extender.extend(&mut state, &mut res);
            }
            None => {
                trace!(
                    "[{}] no response extender for {}",
                    request_id(&state),
                    res.status()
                );
            }
        }

        Box::new(future::ok((state, res)))
    }
}
