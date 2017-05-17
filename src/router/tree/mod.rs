//! Defines an unordered `Tree` holding a collection of recursive `Node` instances.
//!
//! Valid paths are located by recursively matching HTTP request path segments, resulting in a `Node`
//! that has one or more `Route` instances which can be futher considered for dispatch.

use router::route::Route;
use router::tree::node::Node;
use router::tree::node::NodeSegmentType;

pub mod node;
pub mod segment_matcher;

/// A hierarchical tree structure that provides a root [`Node`][node] and subtrees of linked nodes
/// that represent valid [`Request`][request] paths.
///
/// Allows the [`Router`][router] to supply a [`Request`][request] path and obtain `[0..n]` valid
/// [`Route`][route] instances for that path for further evaluation.
///
/// # Examples
///
/// Representing routable the paths `/`, and `/content/identifier`.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::Method;
/// # use hyper::server::{Request, Response};
/// # use gotham::router::route::{Route, RouteImpl};
/// # use gotham::dispatch::Dispatcher;
/// # use gotham::state::State;
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::router::tree::Tree;
/// # use gotham::router::tree::node::Node;
/// # use gotham::router::tree::node::NodeSegmentType;
/// #
/// # fn handler(state: State, _req: Request) -> (State, Response) {
/// #   (state, Response::new())
/// # }

/// # fn basic_route() -> Box<Route + Send + Sync> {
/// #   let methods = vec![Method::Get];
/// #   let matcher = MethodOnlyRequestMatcher::new(methods);
/// #   let dispatcher = Dispatcher::new(|| Ok(handler), ());
/// #   Box::new(RouteImpl::new(matcher, dispatcher))
/// # }
/// #
/// # fn main() {
///   let mut tree = Tree::new();
///   tree.add_route(basic_route());
///
///   let mut content_node = Node::new("content", NodeSegmentType::Static);
///
///   let mut identifier_node = Node::new("identifier", NodeSegmentType::Static);
///   identifier_node.add_route(basic_route());
///
///   content_node.add_child(identifier_node);
///   tree.add_child(content_node);
///
///   assert!(tree.traverse("/").unwrap().last().unwrap().is_routable());
///   assert!(tree.traverse("/content").is_none()); // This path is not routable
///   assert!(tree.traverse("/content/identifier").unwrap().last().unwrap().is_routable());
/// # }
/// ```
///
/// [node]: node/struct.Node.html
/// [router]: ../struct.Router.html
/// [route]: ../route/trait.Route.html
/// [request]: ../../../hyper/server/struct.Request.html
pub struct Tree<'n> {
    root: Node<'n>,
}

impl<'n> Tree<'n> {
    /// Creates a new `Tree` and root [`Node`][node].
    ///
    /// [node]: node/struct.Node.html
    /// [ssm]: segment_matcher/struct.StaticSegmentMatcher.html
    pub fn new() -> Self {
        Tree { root: Node::new("/", NodeSegmentType::Static) }
    }

    /// Adds a child [`Node`][node] to the root of the `Tree`.
    ///
    /// [node]: node/struct.Node.html
    pub fn add_child(&mut self, child: Node<'n>) {
        self.root.add_child(child);
    }

    /// Determines if a child [`Node`][node] representing the exact segment provided
    /// exists at the root of the `Tree`.
    ///
    /// To be used in building a `Tree` structure only.
    ///
    /// [node]: node/struct.Node.html
    pub fn has_child(&self, segment: &str) -> bool {
        self.root.has_child(segment)
    }

    /// Adds a `Route` be evaluated by the `Router` when the root of the `Tree` is requested
    ///
    pub fn add_route(&mut self, route: Box<Route + Send + Sync>) {
        self.root.add_route(route);
    }

    /// Finalizes the Tree for use with [`Requests`][request].
    ///
    /// **Must** be called before this Tree is used in traversal and only after all child nodes
    /// have been fully populated.
    ///
    /// [request]: ../../../hyper/server/struct.Request.html
    ///
    /// TODO: Move this into a function of a `TreeBuilder` to hide modifcation from the `Router` and
    /// ensure the `Tree` must be finalized before use.
    pub fn finalize(&mut self) {
        self.root.sort();
    }

    /// Borrow the root [`Node`][node] of the `Tree`.
    ///
    /// To be used in building a `Tree` structure only.
    ///
    /// [node]: node/struct.Node.html
    pub fn borrow_root(&self) -> &Node<'n> {
        &self.root
    }

    /// Attempt to acquire a path from the `Tree` which matches the `Request` path
    /// and is routable.
    ///
    /// The traversal algorithm has unique properties. Refer to the description of
    /// [`traverse`][node-traverse] within [`Node`][node] for full details.
    ///
    /// [node-traverse]: node/struct.Node.html#method.traverse
    /// [node]: node/struct.Node.html
    pub fn traverse(&'n self, req_path: &str) -> Option<Vec<&Node<'n>>> {
        // TODO: Percent Decode Request Path here.
        self.root.traverse(self.split_request_path(req_path).as_slice())
    }

    /// Spilt a Request path into indivdual segments, leading leading "/" to represent
    /// the root of the path.
    pub fn split_request_path(&self, path: &'n str) -> Vec<&str> {
        let mut segments = vec!["/"];
        segments.extend(path.split('/').filter(|s| *s != "").collect::<Vec<&'n str>>());
        segments
    }
}
