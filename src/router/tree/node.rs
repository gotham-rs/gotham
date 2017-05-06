//! Defines a `Node` which is a recursive member of a `Tree` and represents a segment of a `Request`
//! path
use router::route::Route;
use router::tree::segment_matcher::SegmentMatcher;

/// A recursive member of a route `Tree`.
///
/// Stores a `segment` identifier, a `segment_matcher` and `0..n Route` instances which are
/// further evaluated by the `Router` if the `Node` is determined to be a routable for a single
/// path through the tree.
///
/// # Examples
///
/// Representing the paths `/some`, and `/some/path`.
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
/// # use gotham::router::tree::segment_matcher::StaticSegmentMatcher;
/// #
/// # fn handler(state: State, _req: Request) -> (State, Response) {
/// #   (state, Response::new())
/// # }

/// # fn basic_route() -> Box<Route> {
/// #   let methods = vec![Method::Get];
/// #   let matcher = MethodOnlyRequestMatcher::new(methods);
/// #   let dispatcher = Dispatcher::new(|| Ok(handler), ());
/// #   Box::new(RouteImpl::new(matcher, dispatcher))
/// # }
/// #
/// # fn main() {
///   // The root node is generally created by `Tree` but shown here for completeness
///   let root_segment_matcher = StaticSegmentMatcher::new();
///   let mut root = Node::new("/", Box::new(root_segment_matcher));
///
///   let some_segment_matcher = StaticSegmentMatcher::new();
///   let mut some_node = Node::new("some", Box::new(some_segment_matcher));
///   some_node.add_route(basic_route());
///
///   let path_segment_matcher = StaticSegmentMatcher::new();
///   let mut path_node = Node::new("path", Box::new(path_segment_matcher));
///   path_node.add_route(basic_route());
///
///   some_node.add_child(path_node);
///   root.add_child(some_node);
///
///   assert_eq!(root.traverse(&["some", "path"]).unwrap().last().unwrap().segment(), "path");
/// # }
/// ```
///
// This struct was originally defined as multiple types to represent the various roles a single `Node`
// can play (parent, leaf, parent+leaf) but this led to complexities in API that weren't
// considered to be a valid trade off in the long run.
pub struct Node<'n> {
    segment: &'n str,
    segment_matcher: Box<SegmentMatcher>,
    routes: Vec<Box<Route>>,

    children: Vec<Node<'n>>,
}

impl<'n> Node<'n> {
    /// Creates new Node for the given segment.
    pub fn new(segment: &'n str, segment_matcher: Box<SegmentMatcher>) -> Self {
        Node {
            segment,
            segment_matcher,
            routes: vec![],
            children: vec![],
        }
    }

    /// Provides the segment this node represents.
    pub fn segment(&self) -> &str {
        self.segment
    }

    /// Adds a `Route` be evaluated by the `Router` when acting as a leaf in a single
    /// path through the `Tree`.
    pub fn add_route(&mut self, route: Box<Route>) {
        self.routes.push(route);
    }

    /// Adds a child `Node`.
    ///
    /// e.g. for `/some/path` adding a child representing the segment `path` to an existing
    /// parent `Node` representing `some`.
    pub fn add_child(&mut self, child: Node<'n>) {
        self.children.push(child);
    }

    /// Determines if a child representing the exact segment provided exists
    ///
    /// To be used in building a `Tree` structure only.
    pub fn has_child(&self, segment: &str) -> bool {
        match self.children.iter().find(|n| n.segment == segment) {
            Some(_) => true,
            None => false,
        }
    }

    /// Borrow a child that represents the exact segment provided here.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn borrow_child(&self, segment: &str) -> Option<&Node<'n>> {
        match self.children.iter().find(|n| n.segment == segment) {
            Some(node) => Some(node),
            None => None,
        }
    }

    /// Mutably borrow a child that represents the exact segment provided here.
    ///
    /// To be used in building a `Tree` structure only.
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

    /// True is there is a least one `Route` represented by this `Node`, that is it can act as a
    /// leaf in a single path through the tree.
    pub fn is_routable(&self) -> bool {
        self.routes.len() > 0
    }

    /// Recursively traverses children attempting to locate a path of nodes which indicate they
    /// match all segments of the `Request` path and with the final `Node` of the path  containing `1..n Route`
    /// instances for further processing by the `Router`.
    ///
    /// **Only the first fully matching path is returned.** Child nodes are simply stored in
    /// the order of calls made to `add_child` with no further sorting applied.
    ///
    /// # Matching nuances
    ///
    /// ```text
    ///    /
    ///    |--segment1
    ///       |--:var1     -> (Route)
    ///       |--segment2  -> (Route)
    /// ```
    ///
    /// Assume that `:var1` uses a segment matcher that simply returns true for any provided segment.
    ///
    /// For the `Request` path `/segment1/segment2`
    /// the returned path is `[Node("segment1"), Node(":var1")]`, **NOT** `[Node("segment1"),
    /// Node("segment2")]`.
    ///
    /// In this case a segment matcher for `:var1` that is restricted by a regular expression or
    /// a re-ordering of the `Tree` to:
    ///
    /// ```text
    ///    /
    ///    |--segment1
    ///       |--segment2  -> (Route)
    ///       |--:var1     -> (Route)
    /// ```
    ///
    /// would ensure the `Request` path `/segment1/segment2` is represented by the
    /// returned path `[Node("segment1"), Node("segment2")]`.
    ///
    /// Finally if the `Tree` structure is:
    ///
    /// ```text
    ///    /
    ///    |--segment1
    ///       |--:var1
    ///          |-- segment3 -> (Route)
    ///       |--segment2     -> (Route)
    /// ```
    ///
    /// then for the `Request` path `/segment1/segment2` the  match against `:var1` would be
    /// discounted as that `Node` itself is not routable. The algorithm then backtracks eventually
    /// returning the path `[Node("segment1"), Node("segment2")]`.
    pub fn traverse(&'n self, req_segments: &[&str]) -> Option<Vec<&'n Node<'n>>> {
        match self.inner_traverse(req_segments) {
            Some(mut path) => {
                path.reverse();
                Some(path)
            }
            None => None,
        }
    }

    fn inner_traverse(&'n self, req_segments: &[&str]) -> Option<Vec<&'n Node<'n>>> {
        match req_segments.split_first() {
            Some((req_segment, req_segments)) => {
                self.children
                    .iter()
                    .filter(|ref c| c.segment_matcher.is_match(c.segment, req_segment))
                    .flat_map(|ref c| match c.inner_traverse(req_segments) {
                                  Some(mut path) => {
                        path.push(self);
                        Some(path)
                    }
                                  None => None,
                              })
                    .nth(0)
            }
            None => {
                if self.is_routable() {
                    Some(vec![self])
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::Method;
    use hyper::server::{Request, Response};

    use dispatch::Dispatcher;
    use state::State;

    use router::request_matcher::MethodOnlyRequestMatcher;
    use router::route::{Route, RouteImpl};
    use router::tree::segment_matcher::{StaticSegmentMatcher, DynamicSegmentMatcher};

    fn handler(state: State, _req: Request) -> (State, Response) {
        (state, Response::new())
    }

    fn handler2(state: State, _req: Request) -> (State, Response) {
        (state, Response::new())
    }

    fn get_route() -> Box<Route> {
        let methods = vec![Method::Get];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler), ());
        Box::new(RouteImpl::new(matcher, dispatcher))
    }

    fn test_structure<'n>() -> Node<'n> {
        let sm = StaticSegmentMatcher::new();
        let dm = DynamicSegmentMatcher::new();

        let mut root = Node::new("/", Box::new(sm.clone()));

        // Two methods, same path, same handler
        // [Get|Head]: /seg1
        let mut seg1 = Node::new("seg1", Box::new(sm.clone()));
        let methods = vec![Method::Get, Method::Head];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler), ());
        let route = RouteImpl::new(matcher, dispatcher);
        seg1.add_route(Box::new(route));
        root.add_child(seg1);

        // Two methods, same path, different handlers
        // Post: /seg2
        let mut seg2 = Node::new("seg2", Box::new(sm.clone()));
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
        let mut seg3 = Node::new("seg3", Box::new(sm.clone()));
        let mut seg4 = Node::new("seg4", Box::new(sm.clone()));
        seg4.add_route(get_route());
        seg3.add_child(seg4);
        root.add_child(seg3);

        // Ensure traversal will backtrack and find the correct path if it goes down an ultimately
        // invalid branch, in this case seg6 initially being matched by the dynamic handler segdyn1
        // which matches every segment it sees.
        //
        // Get /seg5/:segdyn1/seg7
        // Get /seg5/seg6
        let mut seg5 = Node::new("seg5", Box::new(sm.clone()));
        let mut seg6 = Node::new("seg6", Box::new(sm.clone()));
        seg6.add_route(get_route());

        let mut segdyn1 = Node::new(":segdyn1", Box::new(dm.clone()));
        let mut seg7 = Node::new("seg7", Box::new(sm.clone()));
        seg7.add_route(get_route());

        segdyn1.add_child(seg7);
        seg5.add_child(segdyn1);
        seg5.add_child(seg6);
        root.add_child(seg5);

        root
    }

    #[test]
    fn assigns_segment() {
        let sm = StaticSegmentMatcher::new();
        let node = Node::new("seg1", Box::new(sm));
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
        assert_eq!(root.traverse(&["seg3", "seg4"])
                       .unwrap()
                       .last()
                       .unwrap()
                       .segment(),
                   "seg4");

        // GET /seg3/seg4/seg5
        assert!(root.traverse(&["seg3", "seg4", "seg5"]).is_none());

        // GET /seg5/seg6
        assert_eq!(root.traverse(&["seg5", "seg6"])
                       .unwrap()
                       .last()
                       .unwrap()
                       .segment(),
                   "seg6");

        // GET /seg5/someval/seg7
        assert_eq!(root.traverse(&["seg5", "someval", "seg7"])
                       .unwrap()
                       .last()
                       .unwrap()
                       .segment(),
                   "seg7");
    }
}
