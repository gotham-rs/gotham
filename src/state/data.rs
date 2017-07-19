use std::any::Any;

use hyper::{Headers, Uri, HttpVersion, Method};

use http::request_path::RequestPathSegments;
use state::request_id::RequestId;

/// A marker trait for types that can be stored in `State`.
pub trait StateData: Any + Send {}

impl StateData for Method {}
impl StateData for Uri {}
impl StateData for HttpVersion {}
impl StateData for Headers {}

impl StateData for RequestPathSegments {}
impl StateData for RequestId {}
