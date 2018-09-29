//! Security based middleware to handle security based sanitizations.
//!
//! Prior to v0.3, this middleware was baked into responses by default. It has
//! now been separated to allow optional usage. You can attach as a middleware
//! at startup to include behaviour as was present before.
//!
//! Currently this middleware will set the following headers:
//!
//! - X-CONTENT-TYPE-OPTIONS: "nosniff"
//! - X-FRAME-OPTIONS: "DENY"
//! - X-XSS-PROTECTION: "1; mode=block"
//!
//! More may be added in future, but these headers provide compatibility with
//! previous versions of Gotham.
use futures::{future, Future};
use handler::HandlerFuture;
use hyper::header::{HeaderValue, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS, X_XSS_PROTECTION};
use middleware::{Middleware, NewMiddleware};
use state::State;
use std::io;

// constant strings to be used as header values
const XFO_VALUE: &'static str = "DENY";
const XXP_VALUE: &'static str = "1; mode=block";
const XCTO_VALUE: &'static str = "nosniff";

/// Middleware binding for the Gotham security handlers.
///
/// This acts as nothing more than a trait implementation for the time
/// being; there are no fields on the struct in use (yet).
#[derive(Clone)]
pub struct SecurityMiddleware;

/// `Middleware` trait implementation.
impl Middleware for SecurityMiddleware {
    /// Attaches security headers to the response.
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        let f = chain(state).and_then(|(state, mut response)| {
            {
                let headers = response.headers_mut();

                headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static(XFO_VALUE));
                headers.insert(X_XSS_PROTECTION, HeaderValue::from_static(XXP_VALUE));
                headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static(XCTO_VALUE));
            }
            future::ok((state, response))
        });

        Box::new(f)
    }
}

/// `NewMiddleware` trait implementation.
impl NewMiddleware for SecurityMiddleware {
    type Instance = Self;

    /// Clones the current middleware to a new instance.
    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
}
