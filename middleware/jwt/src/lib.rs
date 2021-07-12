//! Ensures that only requests with valid JSON Web Tokens
//! included in the HTTP `Authorization` header are allowed
//! to pass.
//!
//! Requests that lack a token are returned with the
//! Status Code `400: Bad Request`. Tokens that fail
//! validation cause the middleware to return Status Code
//! `401: Unauthorized`.
#![warn(missing_docs, rust_2018_idioms)]

#[macro_use]
extern crate gotham_derive;
#[macro_use]
extern crate log;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

mod middleware;
mod state_data;

pub use self::middleware::JwtMiddleware;
pub use self::state_data::AuthorizationToken;

/// This type alias is deprecated. Use `JwtMiddleware` instead.
#[deprecated(since = "0.6.1", note = "Please use `JwtMiddleware` instead")]
#[allow(clippy::upper_case_acronyms)]
pub type JWTMiddleware<T> = JwtMiddleware<T>;
