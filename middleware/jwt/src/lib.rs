//! Ensures that only requests with valid JSON Web Tokens
//! included in the HTTP `Authorization` header are allowed
//! to pass.
//!
//! Requests that lack a token are returned with the
//! Status Code `400: Bad Request`. Tokens that fail
//! validation cause the middleware to return Status Code
//! `401: Unauthorized`.
#![warn(missing_docs, rust_2018_idioms)]

mod middleware;
mod state_data;

pub use self::middleware::JwtMiddleware;
pub use self::state_data::AuthorizationToken;
