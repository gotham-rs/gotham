//! Defines functionality for operating on `Request` path values

use std::str::FromStr;

use state::State;
use router::tree::SegmentMapping;

/// Allow applications to extract `Request` path data into a Struct for use in a type safe manner
/// by downstream `Middleware` and `Handler`.
pub type RequestPathExtractor = Box<Fn(&mut State, SegmentMapping) + Send + Sync>;

/// A generic Request Path Extractor that performs no action.
///
/// Useful for `Request` paths that are purely `Static` segments.
pub fn noop_request_path_extractor(_state: &mut State, _segment_mapping: SegmentMapping) {}
