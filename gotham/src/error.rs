//! The error module is nascent. At present, it re-exports types from the `failure` crate and adds an alias for compatible errors.
//! Future directions for Gotham error types are an ongoing discussion. Feel free to chip in.
use failure::Compat;

pub use failure::Error;

/// An implementation of the single-parameter Result pattern, using our `pub use failure::Error`
pub type Result<T> = ::std::result::Result<T, Error>;

/// An alias for `failure::Error.compat()`, which exists to fulfill the std::error::Error trait.
pub type CompatError = Compat<Error>;
