//! Headers recognised by Gotham which do not exist in the standard headers
//! provided by the Hyper library.

/// Marks the identifier of a request to a Gotham server.
pub const X_REQUEST_ID: &'static str = "x-request-id";

/// Marks the execution time of a Gotham request in microseconds.
pub const X_RUNTIME_MICROSECONDS: &'static str = "x-runtime-microseconds";
