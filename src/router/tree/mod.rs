//! Defines a hierarchial `Tree` with subtrees of `Node`.

use std::collections::HashMap;

use http::PercentDecoded;
use router::route::Route;
use router::tree::node::{Node, NodeBuilder, NodeSegmentType};

pub mod node;

/// A depth ordered `Vec` of `Node` instances that create a routable path through the `Tree` for the
/// matched `Request` path.
pub type Path<'n, 'a, P> = Vec<&'a Node<'n, P>>;

/// Data which is returned from Tree traversal, mapping internal segment value to segment(s)
/// which have been matched against the `Request` path.
///
/// Data is percent and utf8 decoded.
pub struct SegmentMapping<'a, 'b> {
    data: HashMap<&'a str, Vec<&'b PercentDecoded>>,
}

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
/// # use gotham::router::route::{RouteImpl, Extractors};
/// # use gotham::dispatch::Dispatcher;
/// # use gotham::state::State;
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::router::tree::TreeBuilder;
/// # use gotham::router::tree::node::NodeBuilder;
/// # use gotham::router::tree::node::NodeSegmentType;
/// # use gotham::http::request_path::NoopRequestPathExtractor;
/// # use gotham::http::query_string::NoopQueryStringExtractor;
/// # use gotham::http::PercentDecoded;
/// # use gotham::http::request_path;
/// #
/// # fn handler(state: State, _req: Request) -> (State, Response) {
/// #   (state, Response::new())
/// # }
/// #
/// # fn main() {
///   let mut tree_builder: TreeBuilder<()> = TreeBuilder::new();
///
///   let mut activate_node_builder = NodeBuilder::new("activate", NodeSegmentType::Static);
///
///   let mut thing_node_builder = NodeBuilder::new("thing", NodeSegmentType::Dynamic);
///   let batsignal_route = {
///       // elided ...
/// #     let methods = vec![Method::Get];
/// #     let matcher = MethodOnlyRequestMatcher::new(methods);
/// #     let dispatcher = Dispatcher::new(|| Ok(handler), ());
/// #     let extractors: Extractors<NoopRequestPathExtractor, NoopQueryStringExtractor> = Extractors::new();
/// #     let route = RouteImpl::new(matcher, dispatcher, extractors);
/// #     Box::new(route)
///   };
///   thing_node_builder.add_route(batsignal_route);
///
///   activate_node_builder.add_child(thing_node_builder);
///   tree_builder.add_child(activate_node_builder);
///
///   let tree = tree_builder.finalize();
///
///   match tree.traverse(request_path::split("/%61ctiv%61te/batsignal").as_slice()) {
///       Some((path, segment_mapping)) => {
///         assert!(path.last().unwrap().is_routable());
///         assert_eq!(segment_mapping.get("thing").unwrap().last().unwrap().val(), "batsignal");
///       }
///       None => panic!(),
///   }
///
///   // These paths are not routable but could be if 1 or more `Route` were added.
///   assert!(tree.traverse(&[PercentDecoded::new("/").unwrap()]).is_none());
///   assert!(tree.traverse(&[PercentDecoded::new("/activate").unwrap()]).is_none());
/// # }
/// ```
pub struct Tree<'n, P> {
    root: Node<'n, P>,
}

impl<'n, P> Tree<'n, P> {
    /// Borrow the root `Node` of the `Tree`.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn borrow_root(&self) -> &Node<'n, P> {
        &self.root
    }

    /// Attempt to acquire a path from the `Tree` which matches the `Request` path and is routable.
    pub fn traverse<'r>(&'n self,
                        req_path_segments: &'r [PercentDecoded])
                        -> Option<(Path<'n, 'r, P>, SegmentMapping<'n, 'r>)> {
        self.root.traverse(req_path_segments)
    }
}


/// Constructs a `Tree` which is sorted and immutable.
pub struct TreeBuilder<'n, P> {
    root: NodeBuilder<'n, P>,
}

impl<'n, P> TreeBuilder<'n, P> {
    /// Creates a new `Tree` and root `Node`.
    pub fn new() -> Self {
        TreeBuilder { root: NodeBuilder::new("/", NodeSegmentType::Static) }
    }

    /// Adds a direct child to the root of the `TreeBuilder`.
    pub fn add_child(&mut self, child: NodeBuilder<'n, P>) {
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
    pub fn add_route(&mut self, route: Box<Route<P> + Send + Sync>) {
        self.root.add_route(route);
    }

    /// Finalizes and sorts all internal data and creates a Tree for use with a `Router`.
    pub fn finalize(self) -> Tree<'n, P> {
        Tree { root: self.root.finalize() }
    }
}
