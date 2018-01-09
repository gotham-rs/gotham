use std::any::Any;

use hyper::{Body, Headers, HttpVersion, Method, Uri};
use tokio_core::reactor::Handle;

use http::request::path::RequestPathSegments;
use state::request_id::RequestId;

/// A marker trait for types that can be stored in `State`.
///
/// Typically implemented using `#[derive(StateData)]`, which is provided by the `gotham_derive`
/// crate.
pub trait StateData: Any {}

impl StateData for Body {}
impl StateData for Method {}
impl StateData for Uri {}
impl StateData for HttpVersion {}
impl StateData for Headers {}

impl StateData for Handle {}

impl StateData for RequestPathSegments {}
impl StateData for RequestId {}
