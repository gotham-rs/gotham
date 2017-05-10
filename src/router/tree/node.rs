//! Defines a `Node` which is a recursive member of a `Tree` and represents a segment of a `Request`
//! path
use router::route::Route;
use router::tree::segment_matcher::SegmentMatcher;

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
/// # use gotham::router::tree::segment_matcher::StaticSegmentMatcher;
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
/// #  let mut root_node = Node::new("/", Box::new(StaticSegmentMatcher::new()));
///   let mut content_node = Node::new("content", Box::new(StaticSegmentMatcher::new()));
///
///   let mut identifier_node = Node::new("identifier", Box::new(StaticSegmentMatcher::new()));
///   identifier_node.add_route(basic_route());
///
///   content_node.add_child(identifier_node);
///   root_node.add_child(content_node);
///
///   let traversal = root_node.traverse(&["content", "identifier"]);
///   assert!(traversal.unwrap().last().unwrap().is_routable());
/// # }
/// ```
///
/// [tree]: ../struct.Tree.html
/// [router]: ../../struct.Router.html
/// [route]: ../../route/trait.Route.html
/// [request]: ../../../../hyper/server/struct.Request.html
///

// This struct was originally defined as multiple types to represent the various roles a single `Node`
// can play (parent, leaf, parent+leaf) but this led to complexities in API that weren't
// considered to be a valid trade off in the long run.
pub struct Node<'n> {
    segment: &'n str,
    segment_matcher: Box<SegmentMatcher + Send + Sync>,
    routes: Vec<Box<Route + Send + Sync>>,

    children: Vec<Node<'n>>,
}

impl<'n> Node<'n> {
    /// Creates new `Node` for the given segment.
    pub fn new(segment: &'n str, segment_matcher: Box<SegmentMatcher + Send + Sync>) -> Self {
        Node {
            segment,
            segment_matcher,
            routes: vec![],
            children: vec![],
        }
    }

    /// Provides the segment this `Node` represents.
    pub fn segment(&self) -> &str {
        self.segment
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

    /// Determines if a child representing the exact segment provided exists
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
    /// **Only the first fully matching path is returned.** Child nodes are simply stored in
    /// the order of calls made to `add_child` with no further sorting applied.
    ///
    /// # Matching Nuances
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
    /// For the [`Request`][request] path `/segment1/segment2`
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
    /// would ensure the [`Request`][request] path `/segment1/segment2` is represented by the
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
    /// then for the [`Request`][request] path `/segment1/segment2` the  match against `:var1` would be
    /// discounted as that `Node` itself is not routable. The algorithm then backtracks eventually
    /// returning the path `[Node("segment1"), Node("segment2")]`.
    ///
    /// [router]: ../../struct.Router.html
    /// [route]: ../../route/trait.Route.html
    /// [request]: ../../../../hyper/server/struct.Request.html
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

    fn get_route() -> Box<Route + Send + Sync> {
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
