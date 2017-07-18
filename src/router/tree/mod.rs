//! Defines a hierarchial `Tree` with subtrees of `Node`.

use std::collections::HashMap;

use http::PercentDecoded;
use router::route::Route;
use router::tree::node::{Node, NodeBuilder, SegmentType};

pub mod node;

/// A depth ordered `Vec` of `Node` instances that create a routable path through the `Tree` for the
/// matched `Request` path.
pub type Path<'a> = Vec<&'a Node>;

/// Data which is returned from Tree traversal, mapping internal segment value to segment(s)
/// which have been matched against the `Request` path.
///
/// Data is percent and utf8 decoded.
#[derive(Debug)]
pub struct SegmentMapping<'a, 'b> {
    data: HashMap<&'a str, Vec<&'b PercentDecoded>>,
}

/// Number of segments from a `Request` path that are considered to have been processed
/// by an `Router` traversing its `Tree`.
type SegmentsProcessed = usize;

impl<'a, 'b> SegmentMapping<'a, 'b> {
    /// Returns a reference for `Request` path segments mapped to the segment key.
    pub fn get(&self, key: &'a str) -> Option<&Vec<&'b PercentDecoded>> {
        self.data.get(key)
    }

    /// Determines if `Request` path segments are mapped to the segment key.
    pub fn contains_key(&self, key: &'a str) -> bool {
        self.data.contains_key(key)
    }

    /// Adds an empty value for a segment key, useful for segments that are considered
    /// optional and haven't been explicitly provided as part of a `Request` path
    pub fn add_unmapped_segment(&mut self, key: &'a str) {
        if !self.data.contains_key(key) {
            self.data.insert(key, Vec::new());
        }
    }

    /// Number of segments from the Request path that have been mapped
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

/// A hierarchical structure that provides a root `Node` and subtrees of linked nodes
/// that represent valid `Request` paths.
///
/// Allows the `Router` to supply a `Request` path and obtain `[0..n]` valid
/// `Route` instances for that path for further evaluation.
///
/// # Examples
///
/// Desired tree:
///
/// ```text
///    /
///    | -- activate
///         | -- batsignal      (Routable)
/// ```
///
/// Code:
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::Method;
/// # use hyper::server::{Request, Response};
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// # use gotham::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
/// # use gotham::state::State;
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::tree::node::NodeBuilder;
/// # use gotham::router::tree::node::SegmentType;
/// # use gotham::http::request_path::{RequestPathSegments, NoopRequestPathExtractor};
/// # use gotham::router::request::query_string::NoopQueryStringExtractor;
/// # use gotham::http::PercentDecoded;
/// #
/// # fn handler(state: State, _req: Request) -> (State, Response) {
/// #   (state, Response::new())
/// # }
/// #
/// # fn main() {
/// # let pipeline_set = finalize_pipeline_set(new_pipeline_set());
///   let mut tree_builder: TreeBuilder = TreeBuilder::new();
///
///   let mut activate_node_builder = NodeBuilder::new("activate", SegmentType::Static);
///
///   let mut thing_node_builder = NodeBuilder::new("thing", SegmentType::Dynamic);
///   let batsignal_route = {
///       // elided ...
/// #     let methods = vec![Method::Get];
/// #     let matcher = MethodOnlyRequestMatcher::new(methods);
/// #     let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
/// #     let extractors: Extractors<NoopRequestPathExtractor, NoopQueryStringExtractor> = Extractors::new();
/// #     let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
/// #     Box::new(route)
///   };
///   thing_node_builder.add_route(batsignal_route);
///
///   activate_node_builder.add_child(thing_node_builder);
///   tree_builder.add_child(activate_node_builder);
///
///   let tree = tree_builder.finalize();
///
///   match tree.traverse(RequestPathSegments::new("/%61ctiv%61te/batsignal").segments().as_slice()) {
///       Some((path, leaf, segments_processed, segment_mapping)) => {
///         assert!(path.last().unwrap().is_routable());
///         assert_eq!(path.last().unwrap().segment(), leaf.segment());
///         assert_eq!(segments_processed, 2);
///         assert_eq!(segment_mapping.get("thing").unwrap().last().unwrap().val(), "batsignal");
///       }
///       None => panic!(),
///   }
///
///   // These paths are not routable but could be if 1 or more `Route` were added.
///   assert!(tree.traverse(&[&PercentDecoded::new("/").unwrap()]).is_none());
///   assert!(tree.traverse(&[&PercentDecoded::new("/activate").unwrap()]).is_none());
/// # }
/// ```
pub struct Tree {
    root: Node,
}

impl Tree {
    /// Borrow the root `Node` of the `Tree`.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn borrow_root(&self) -> &Node {
        &self.root
    }

    /// Attempt to acquire a path from the `Tree` which matches the `Request` path and is routable.
    pub fn traverse<'r, 'n>
        (&'n self,
         req_path_segments: &'r [&PercentDecoded])
         -> Option<(Path<'n>, &Node, SegmentsProcessed, SegmentMapping<'n, 'r>)> {
        trace!(" starting tree traversal");
        self.root.traverse(req_path_segments)
    }
}


/// Constructs a `Tree` which is sorted and immutable.
pub struct TreeBuilder {
    root: NodeBuilder,
}

impl TreeBuilder {
    /// Creates a new `Tree` and root `Node`.
    pub fn new() -> Self {
        trace!(" creating new tree");
        TreeBuilder { root: NodeBuilder::new("/", SegmentType::Static) }
    }

    /// Adds a direct child to the root of the `TreeBuilder`.
    pub fn add_child(&mut self, child: NodeBuilder) {
        self.root.add_child(child);
    }

    /// Determines if a child `Node` representing the exact segment provided
    /// exists at the root of the `Tree`.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn has_child(&self, segment: &str) -> bool {
        self.root.has_child(segment)
    }

    /// Adds a `Route` be evaluated by the `Router` when the root of the
    /// `Tree` is requested.
    pub fn add_route(&mut self, route: Box<Route + Send + Sync>) {
        self.root.add_route(route);
    }

    /// Finalizes and sorts all internal data and creates a Tree for use with a `Router`.
    pub fn finalize(self) -> Tree {
        Tree { root: self.root.finalize() }
    }
}
