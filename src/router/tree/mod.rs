//! Defines an unordered `Tree` holding a collection of recursive `Node` instances.
//!
//! Valid paths are located by recursively matching HTTP request path segments, resulting in a `Node`
//! that has one or more `Route` instances which can be futher considered for dispatch.

pub mod node;
pub mod segment_matcher;
