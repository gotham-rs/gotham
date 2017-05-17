//! Defines components of `Nodes` which live within a `Tree`.

use std::cmp::Ordering;
use std::collections::HashMap;

use router::route::Route;

/// Indicates the type of segment which is being represented by this Node.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeSegmentType<'n> {
    /// Is matched exactly to the corresponding segment for incoming request paths. Unlike all
    /// other `NodeSegmentTypes` this segment is **not** stored within `State`.
    Static,
    /// Uses the supplied regex to determine match against incoming request paths.
    Constrained {
        /// Regex used to match against a single segment of a request path.
        regex: &'n str,
    },
    /// Matches any corresponding segment for incoming request paths.
    Dynamic,
    /// Matches multiple path segments until the end of the request path or until a child
    /// segment of the above defined types is found.
    Glob,
}

/// A recursive member of [`Tree`][tree] and represents a segment of a [`Request`][request] path.
///
/// Stores a `segment`, a `segment_matcher` and `0..n` [`Route`][route] instances which
/// are further evaluated by the [`Router`][router] if the `Node` is determined to be
/// routable for a single path through the tree.
///
/// # Examples
///
/// Representing the path `/content/identifier`.
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
/// #  let mut root_node = Node::new("/", NodeSegmentType::Static);
///   let mut content_node = Node::new("content", NodeSegmentType::Static);
///
///   let mut identifier_node = Node::new("identifier", NodeSegmentType::Static);
///   identifier_node.add_route(basic_route());
///
///   content_node.add_child(identifier_node);
///   root_node.add_child(content_node);
///
///   let traversal = root_node.traverse(&["/", "content", "identifier"]);
///   assert!(traversal.unwrap().last().unwrap().is_routable());
/// # }
/// ```
///
/// [tree]: ../struct.Tree.html
/// [router]: ../../struct.Router.html
/// [route]: ../../route/trait.Route.html
/// [request]: ../../../../hyper/server/struct.Request.html
///
pub struct Node<'n> {
    segment: &'n str,
    segment_type: NodeSegmentType<'n>,
    routes: Vec<Box<Route + Send + Sync>>,

    children: Vec<Node<'n>>,
}

impl<'n> Node<'n> {
    /// Creates new `Node` for the given segment.
    pub fn new(segment: &'n str, segment_type: NodeSegmentType<'n>) -> Self {
        Node {
            segment,
            segment_type,
            routes: vec![],
            children: vec![],
        }
    }

    /// Provides the segment this `Node` represents.
    pub fn segment(&self) -> &str {
        self.segment
    }

    /// Provides the type of segment this `Node` represents.
    pub fn segment_type(&self) -> &NodeSegmentType {
        &self.segment_type
    }

    /// Adds a [`Route`][route] be evaluated by the [`Router`][router] when acting as a leaf in a
    /// single path through the [`Tree`][tree].
    ///
    /// [tree]: ../struct.Tree.html
    /// [router]: ../../struct.Router.html
    /// [route]: ../../route/trait.Route.html
    pub fn add_route(&mut self, route: Box<Route + Send + Sync>) {
        self.routes.push(route);
    }

    /// Allow the [`Router`][router] to access the [`Routes`][route] for this `Node` when it is
    /// selected as the lead in a single path through the [`Tree`][tree].
    ///
    /// [tree]: ../struct.Tree.html
    /// [router]: ../../struct.Router.html
    /// [route]: ../../route/trait.Route.html
    pub fn borrow_routes(&self) -> &Vec<Box<Route + Send + Sync>> {
        &self.routes
    }

    /// Adds a child `Node`.
    ///
    /// e.g. for `/content/identifier` adding a child representing the segment `identifier` to
    /// an existing parent `Node` representing `content`.
    pub fn add_child(&mut self, child: Node<'n>) {
        self.children.push(child);
    }

    /// Sorts all children
    ///
    /// Must be called before this Node and it's children are used in traversal, generally once
    /// the owning [`Tree`][tree] has been fully constructed.
    ///
    /// [tree]: ../struct.Tree.html
    pub fn sort(&mut self) {
        self.children.sort();

        // Recursively sort all children, if any.
        for child in &mut self.children {
            child.sort();
        }
    }

    /// Determines if a child representing the exact segment provided exists.
    ///
    /// To be used in building a [`Tree`][tree] structure only.
    ///
    /// [tree]: ../struct.Tree.html
    pub fn has_child(&self, segment: &str) -> bool {
        match self.children.iter().find(|n| n.segment == segment) {
            Some(_) => true,
            None => false,
        }
    }

    /// Borrow a child that represents the exact segment provided here.
    ///
    /// To be used in building a [`Tree`][tree] structure only.
    ///
    /// [tree]: ../struct.Tree.html
    pub fn borrow_child(&self, segment: &str) -> Option<&Node<'n>> {
        match self.children.iter().find(|n| n.segment == segment) {
            Some(node) => Some(node),
            None => None,
        }
    }

    /// Mutably borrow a child that represents the exact segment provided here.
    ///
    /// To be used in building a [`Tree`][tree] structure only.
    ///
    /// [tree]: ../struct.Tree.html
    pub fn borrow_mut_child(&mut self, segment: &str) -> Option<&mut Node<'n>> {
        match self.children.iter_mut().find(|n| n.segment == segment) {
            Some(node) => Some(node),
            None => None,
        }
    }

    /// True if there is at least one child `Node` present
    pub fn is_parent(&self) -> bool {
        self.children.len() > 0
    }

    /// True is there is a least one [`Route`][route] represented by this `Node`, that is it can act as a
    /// leaf in a single path through the tree.
    ///
    /// [route]: ../../route/trait.Route.html
    pub fn is_routable(&self) -> bool {
        self.routes.len() > 0
    }

    /// Recursively traverses children attempting to locate a path of nodes which indicate they
    /// match all segments of the [`Request`][request] path and with the final `Node` of the path
    /// containing `1..n` [`Route`][route] instances for further processing by the
    /// [`Router`][router].
    ///
    /// **Only the first fully matching path is returned.**
    ///
    /// # Matching Nuances
    ///
    /// ```text
    ///    /
    ///    |--static1
    ///       |--dynamic     -> (Route)
    ///       |--static2     -> (Route)
    /// ```
    ///
    /// For the [`Request`][request] path `/static1/static2` the returned path is
    /// `[Node("static1"), Node("dynamic")]`, **NOT** `[Node("static1"), Node("static2")]`.
    ///
    /// In this case a static matcher for `dynamic` that is restricted by a regular expression or
    /// a re-ordering of the `Tree` to:
    ///
    /// ```text
    ///    /
    ///    |--static1
    ///       |--static2  -> (Route)
    ///       |--dynamic     -> (Route)
    /// ```
    ///
    /// would ensure the [`Request`][request] path `/static1/static2` is represented by the
    /// returned path `[Node("static1"), Node("static2")]`.
    ///
    /// Finally if the `Tree` structure is:
    ///
    /// ```text
    ///    /
    ///    |--static1
    ///       |--dynamic
    ///          |-- static3 -> (Route)
    ///       |--static2     -> (Route)
    /// ```
    ///
    /// then for the [`Request`][request] path `/static1/static2` the  match against `dynamic` would be
    /// discounted as that `Node` itself is not routable. The algorithm then backtracks eventually
    /// returning the path `[Node("static1"), Node("static2")]`.
    ///
    /// [router]: ../../struct.Router.html
    /// [route]: ../../route/trait.Route.html
    /// [request]: ../../../../hyper/server/struct.Request.html
    pub fn traverse(&'n self, req_path_segments: &[&str]) -> Option<Vec<&Node<'n>>> {
        match self.inner_traverse(req_path_segments, vec![]) {
            Some((mut path, segment_mapping)) => {
                path.reverse();
                Some(path)
            }
            None => None,
        }
    }

    fn inner_traverse(&self,
                      req_path_segments: &[&str],
                      mut consumed_segments: Vec<String>)
                      -> Option<(Vec<&Node<'n>>, HashMap<&str, Vec<String>>)> {
        match req_path_segments.split_first() {
            Some((x, xs)) => {
                if self.is_match(x) {
                    if self.is_routable() && req_path_segments.len() == 1 {
                        // Leaf Node for Route Path, start building result
                        consumed_segments.push(String::from(*x));

                        let mut segment_mapping = HashMap::new();
                        segment_mapping.insert(self.segment(), consumed_segments);

                        Some((vec![self], segment_mapping))
                    } else {
                        match xs.iter().peekable().peek() {
                            Some(y) => {
                                match self.children.iter().find(|c| c.is_match(y)) {
                                    Some(c) => {
                                        // Direct child, continue down tree
                                        match c.inner_traverse(xs, vec![]) {
                                            Some((mut path, mut segment_mapping)) => {
                                                consumed_segments.push(String::from(*x));
                                                segment_mapping.insert(self.segment(),
                                                                       consumed_segments);
                                                path.push(self);
                                                Some((path, segment_mapping))
                                            }
                                            None => None,
                                        }
                                    }
                                    None => {
                                        match self.segment_type {
                                            // If we're in a Glob consume segment and continue
                                            // otherwise we've failed to find a suitable way
                                            // forward.
                                            NodeSegmentType::Glob => {
                                                // Prepare for use within State
                                                consumed_segments.push(String::from(*x));
                                                self.inner_traverse(xs, consumed_segments)
                                            }
                                            _ => None,
                                        }
                                    }
                                }
                            }
                            None => None,
                        }
                    }
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn is_match(&self, request_path_segment: &str) -> bool {
        match self.segment_type {
            NodeSegmentType::Static => self.segment == request_path_segment,
            NodeSegmentType::Constrained { regex: _ } => unimplemented!(), // TODO
            NodeSegmentType::Dynamic => true,
            NodeSegmentType::Glob => true,
        }
    }
}

impl<'n> Ord for Node<'n> {
    fn cmp(&self, other: &Node<'n>) -> Ordering {
        (&self.segment_type, &self.segment).cmp(&(&other.segment_type, &other.segment))
    }
}

impl<'n> PartialOrd for Node<'n> {
    fn partial_cmp(&self, other: &Node<'n>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'n> PartialEq for Node<'n> {
    fn eq(&self, other: &Node<'n>) -> bool {
        (&self.segment_type, &self.segment) == (&other.segment_type, &other.segment)
    }
}

impl<'n> Eq for Node<'n> {}

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::Method;
    use hyper::server::{Request, Response};

    use dispatch::Dispatcher;
    use state::State;

    use router::request_matcher::MethodOnlyRequestMatcher;
    use router::route::{Route, RouteImpl};

    fn handler(state: State, _req: Request) -> (State, Response) {
        (state, Response::new())
    }

    fn handler2(state: State, _req: Request) -> (State, Response) {
        (state, Response::new())
    }

    fn get_route() -> Box<Route + Send + Sync> {
        let methods = vec![Method::Get];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler), ());
        Box::new(RouteImpl::new(matcher, dispatcher))
    }

    fn test_structure<'n>() -> Node<'n> {
        let mut root = Node::new("/", NodeSegmentType::Static);

        // Two methods, same path, same handler
        // [Get|Head]: /seg1
        let mut seg1 = Node::new("seg1", NodeSegmentType::Static);
        let methods = vec![Method::Get, Method::Head];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler), ());
        let route = RouteImpl::new(matcher, dispatcher);
        seg1.add_route(Box::new(route));
        root.add_child(seg1);

        // Two methods, same path, different handlers
        // Post: /seg2
        let mut seg2 = Node::new("seg2", NodeSegmentType::Static);
        let methods = vec![Method::Post];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler), ());
        let route = RouteImpl::new(matcher, dispatcher);
        seg2.add_route(Box::new(route));

        // Patch: /seg2
        let methods = vec![Method::Patch];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler2), ());
        let route = RouteImpl::new(matcher, dispatcher);
        seg2.add_route(Box::new(route));
        root.add_child(seg2);

        // Ensure basic traversal
        // Get: /seg3/seg4
        let mut seg3 = Node::new("seg3", NodeSegmentType::Static);
        let mut seg4 = Node::new("seg4", NodeSegmentType::Static);
        seg4.add_route(get_route());
        seg3.add_child(seg4);
        root.add_child(seg3);

        // Ensure traversal will backtrack and find the correct path if it goes down an ultimately
        // invalid branch, in this case seg6 initially being matched by the dynamic handler segdyn1
        // which matches every segment it sees.
        //
        // Get /seg5/:segdyn1/seg7
        // Get /seg5/seg6
        let mut seg5 = Node::new("seg5", NodeSegmentType::Static);
        let mut seg6 = Node::new("seg6", NodeSegmentType::Static);
        seg6.add_route(get_route());

        let mut segdyn1 = Node::new(":segdyn1", NodeSegmentType::Dynamic);
        let mut seg7 = Node::new("seg7", NodeSegmentType::Static);
        seg7.add_route(get_route());

        // Ensure traversal will respect Globs
        let mut seg8 = Node::new("seg8", NodeSegmentType::Glob);
        let mut seg9 = Node::new("seg9", NodeSegmentType::Static);
        let mut seg10 = Node::new("seg10", NodeSegmentType::Glob);
        seg10.add_route(get_route());
        seg9.add_child(seg10);
        seg8.add_child(seg9);
        root.add_child(seg8);

        segdyn1.add_child(seg7);
        seg5.add_child(segdyn1);
        seg5.add_child(seg6);
        root.add_child(seg5);

        root.sort();
        root
    }

    #[test]
    fn assigns_segment() {
        let node = Node::new("seg1", NodeSegmentType::Static);
        assert_eq!("seg1", node.segment());
    }

    #[test]
    fn manages_children() {
        let root = test_structure();
        assert!(root.has_child("seg1"));
        assert!(root.has_child("seg2"));

        assert!(root.is_parent());
        assert!(root.borrow_child("seg1").is_some());
        assert!(root.borrow_child("seg2").is_some());
        assert!(root.borrow_child("seg0").is_none());

        let node = root.borrow_child("seg1").unwrap();
        assert!(!node.is_parent());
    }

    #[test]
    fn traverses_children() {
        let root = test_structure();

        // GET /seg3/seg4
        assert_eq!(root.traverse(&["/", "seg3", "seg4"])
                       .unwrap()
                       .last()
                       .unwrap()
                       .segment(),
                   "seg4");

        // GET /seg3/seg4/seg5
        assert!(root.traverse(&["/", "seg3", "seg4", "seg5"]).is_none());

        // GET /seg5/seg6
        assert_eq!(root.traverse(&["/", "seg5", "seg6"])
                       .unwrap()
                       .last()
                       .unwrap()
                       .segment(),
                   "seg6");

        // GET /seg5/someval/seg7
        assert_eq!(root.traverse(&["/", "seg5", "someval", "seg7"])
                       .unwrap()
                       .last()
                       .unwrap()
                       .segment(),
                   "seg7");

        println!("~~~");
        // GET /some/path/seg9/another/path
        assert_eq!(root.traverse(&["/", "some", "path", "seg9", "some2", "path2"])
                       .unwrap()
                       .last()
                       .unwrap()
                       .segment(),
                   "seg10");
    }
}
