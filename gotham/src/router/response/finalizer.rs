//! Defines functionality for finalizing a `Response` after all pipelines, middlewares, handlers
//! and internal extenders have completed.

use std::collections::HashMap;
use std::sync::Arc;

use futures::future;
use hyper::{Body, Response, StatusCode};

use handler::HandlerFuture;
use state::{request_id, State};

use router::response::extender::ResponseExtender;

/// Holds an immutable collection of `ResponseExtender` values, as configured using
/// `ResponseFinalizerBuilder::add`. This type is constructed automatically when using the
/// `gotham::router::builder` API. See `RouterBuilder::add_response_extender` for details on
/// configuring `ResponseExtender` values for each `StatusCode`.
#[derive(Clone)]
pub struct ResponseFinalizer {
    data: Arc<HashMap<StatusCode, Box<ResponseExtender<Body> + Send + Sync>>>,
}

/// Builds an immutable `ResponseFinalizer`.
pub struct ResponseFinalizerBuilder {
    data: HashMap<StatusCode, Box<ResponseExtender<Body> + Send + Sync>>,
}

impl ResponseFinalizerBuilder {
    /// Creates a new ResponseFinalizer instance.
    #[deprecated(
        since = "0.2.0",
        note = "use the new `gotham::router::builder` API to configure ResponseExtenders"
    )]
    pub fn new() -> Self {
        ResponseFinalizerBuilder::internal_new()
    }

    pub(in router) fn internal_new() -> Self {
        let handlers = HashMap::new();
        ResponseFinalizerBuilder { data: handlers }
    }

    /// Add an Finalizer for responses that have been assigned this status_code.
    pub fn add(
        &mut self,
        status_code: StatusCode,
        extender: Box<ResponseExtender<Body> + Send + Sync>,
    ) {
        trace!(" adding response extender for {}", status_code);
        self.data.insert(status_code, extender);
    }

    /// Finalize population of error handlers for the application, ready for use by a `Router`
    pub fn finalize(self) -> ResponseFinalizer {
        ResponseFinalizer {
            data: Arc::new(self.data),
        }
    }
}

impl ResponseFinalizer {
    /// Finalize the `Response` if a `ResponseFinalizer` has been supplied for the
    /// status code assigned to the `Response`.
    pub fn finalize(&self, mut state: State, mut res: Response<Body>) -> Box<HandlerFuture> {
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
