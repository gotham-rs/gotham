//! Defines a hierarchial `Tree` with subtrees of `Node`.

use std::collections::HashMap;

use url;

use router::route::Route;
use router::tree::node::Node;
use router::tree::node::NodeSegmentType;

pub mod node;

/// A depth ordered `Vec` of `Node` instances that create a routable path through the `Tree` for the
/// matched `Request` path.
pub type Path<'n, 'a, P> = Vec<&'a Node<'n, P>>;

/// Data which is returned from Tree traversal, mapping internal segment value to segment(s)
/// which have been matched against the `Request` path.
pub type SegmentMapping<'a> = HashMap<&'a str, Vec<String>>;

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
/// # use gotham::router::route::RouteImpl;
/// # use gotham::dispatch::Dispatcher;
/// # use gotham::state::State;
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::router::tree::Tree;
/// # use gotham::router::tree::node::Node;
/// # use gotham::router::tree::node::NodeSegmentType;
/// # use gotham::http::request_path::noop_request_path_extractor as noop;
/// #
/// # fn handler(state: State, _req: Request) -> (State, Response) {
/// #   (state, Response::new())
/// # }
/// #
/// # fn main() {
///   let mut tree: Tree<()> = Tree::new();
///
///   let mut activate_node = Node::new("activate", NodeSegmentType::Static);
///
///   let mut variable_node = Node::new("thing", NodeSegmentType::Dynamic);
///   let batsignal_route = {
///       // elided ...
/// #     let methods = vec![Method::Get];
/// #     let matcher = MethodOnlyRequestMatcher::new(methods);
/// #     let dispatcher = Dispatcher::new(|| Ok(handler), ());
/// #     Box::new(RouteImpl::new(matcher, dispatcher, Box::new(noop)))
///   };
///   variable_node.add_route(batsignal_route);
///
///   activate_node.add_child(variable_node);
///   tree.add_child(activate_node);
///
///   // Here `a` is percent encoded in the request path
///   match tree.traverse("/%61ctiv%61te/batsignal") {
///       Some((path, segment_mapping)) => {
///         assert!(path.last().unwrap().is_routable());
///         assert_eq!(segment_mapping.get("thing").unwrap().last().unwrap(), "batsignal");
///       }
///       None => panic!(),
///   }
///
///   // These paths are not routable but could be if 1 or more `Route` were added.
///   assert!(tree.traverse("/").is_none());
///   assert!(tree.traverse("/activate").is_none());
/// # }
/// ```
pub struct Tree<'n, P> {
    root: Node<'n, P>,
}

impl<'n, P> Tree<'n, P> {
    /// Creates a new `Tree` and root `Node`.
    pub fn new() -> Self {
        Tree { root: Node::new("/", NodeSegmentType::Static) }
    }

    /// Adds a child `Node` to the root of the `Tree`.
    pub fn add_child(&mut self, child: Node<'n, P>) {
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

    /// Finalizes the Tree for use with `Requests`.
    ///
    /// **Must** be called before this Tree is used in traversal and only after all child nodes
    /// have been fully populated.
    ///
    /// TODO: Move this into a function of a `TreeBuilder` to hide modifcation from the `Router` and
    /// ensure the `Tree` must be finalized before use.
    pub fn finalize(&mut self) {
        self.root.sort();
    }

    /// Borrow the root `Node` of the `Tree`.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn borrow_root(&self) -> &Node<'n, P> {
        &self.root
    }

    /// Attempt to acquire a path from the `Tree` which matches the `Request` path
    /// and is routable.
    ///
    /// Internally ensures `Request` path is percent decoded before traversal.
    pub fn traverse<'a>(&'n self,
                    req_path: &str)
                    -> Option<(Path<'n, 'a, P>, SegmentMapping<'n>)> {
        let pd = url::percent_encoding::percent_decode(req_path.as_bytes());
        match pd.decode_utf8() {
            Ok(ref path) => self.root.traverse(self.split_request_path(path).as_slice()),
            Err(_) => None,
        }
    }

    /// Spilt a Request path into indivdual segments, leading leading "/" to represent
    /// the root of the path.
    pub fn split_request_path(&self, path: &'n str) -> Vec<&str> {
        let mut segments = vec!["/"];
        segments.extend(path.split('/').filter(|s| *s != "").collect::<Vec<&'n str>>());
        segments
    }
}
