use std::any::Any;

use hyper::{Body, HeaderMap, Method, Uri, Version};

use helpers::http::request::path::RequestPathSegments;
use state::request_id::RequestId;

/// A marker trait for types that can be stored in `State`.
///
/// This is typically implemented using `#[derive(StateData)]`, which is provided by the
/// `gotham_derive` crate.
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate gotham_derive;
/// #
/// # use gotham::state::{FromState, State};
/// #
/// #[derive(StateData)]
/// struct MyStateData {
///     x: u32,
/// }
/// # fn main() {
/// #   State::with_new(|state| {
/// #       state.put(MyStateData { x: 1 });
/// #       assert_eq!(MyStateData::borrow_from(state).x, 1);
/// #   });
/// # }
/// ```
pub trait StateData: Any + Send {}

impl StateData for Body {}
impl StateData for Method {}
impl StateData for Uri {}
impl StateData for Version {}
impl StateData for HeaderMap {}

impl StateData for RequestPathSegments {}
impl StateData for RequestId {}
