//! Defines HTTP headers which are set by Gotham for various (often security) purposes that are not
//! defined in Hyper.

pub mod x_request_id;

pub use http::response::headers::x_request_id::XRequestId;

use hyper::header::{self, Header, Raw};
