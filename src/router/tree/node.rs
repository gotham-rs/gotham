//! Defines `Node` and `NodeSegmentType` for `Tree`

use std::cmp::Ordering;
use std::collections::HashMap;

use router::route::Route;
use router::tree::SegmentMapping;
use router::tree::Path;

/// Indicates the type of segment which is being represented by this Node.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeSegmentType<'n> {
    /// Is matched exactly to the corresponding segment for incoming request paths. Unlike all
    /// other `NodeSegmentTypes` values determined to be associated with this segment
    /// within a `Request` path are **not** stored within `State`.
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

/// A recursive member of `Tree` representative of a segment(s) in a routable path.
///
/// Ultimately provides `0..n` `Route` instances which are further evaluated by the `Router` if
/// the `Node` is determined to be the routable end point for a single path through the tree.
///
/// # Examples
///
/// Representing the path `/activate/batsignal`.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::Method;
/// # use hyper::server::{Request, Response};
/// #
/// # use gotham::http::request_path::NoopRequestPathExtractor;
/// # use gotham::router::route::{RouteImpl, Extractors};
/// # use gotham::dispatch::Dispatcher;
/// # use gotham::state::State;
/// # use gotham::router::request_matcher::MethodOnlyRequestMatcher;
/// # use gotham::router::tree::node::Node;
/// # use gotham::router::tree::node::NodeSegmentType;
/// #
/// # fn handler(state: State, _req: Request) -> (State, Response) {
/// #   (state, Response::new())
/// # }
/// #
/// # fn main() {
///   let mut root_node:Node<()> = Node::new("/", NodeSegmentType::Static);
///   let mut activate_node = Node::new("activate", NodeSegmentType::Static);
///
///   let mut batsignal_node = Node::new("batsignal", NodeSegmentType::Static);
///   let route = {
///       // elided ..
/// #     let methods = vec![Method::Get];
/// #     let matcher = MethodOnlyRequestMatcher::new(methods);
/// #     let dispatcher = Dispatcher::new(|| Ok(handler), ());
///       let extractors: Extractors<NoopRequestPathExtractor> = Extractors::new();
///       let route = RouteImpl::new(matcher, dispatcher, extractors);
///       Box::new(route)
///   };
///   batsignal_node.add_route(route);
///
///   activate_node.add_child(batsignal_node);
///   root_node.add_child(activate_node);
///
///    match root_node.traverse(&["/", "activate", "batsignal"]) {
///        Some((path, _)) => assert!(path.last().unwrap().is_routable()),
///        None => panic!(),
///    }
/// # }
/// ```
pub struct Node<'n, P> {
    segment: &'n str,
    segment_type: NodeSegmentType<'n>,
    routes: Vec<Box<Route<P> + Send + Sync>>,

    children: Vec<Node<'n, P>>,
}

impl<'n, P> Node<'n, P> {
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

    /// Adds a `Route` be evaluated by the `Router` when acting as a leaf in a
    /// single path through the `Tree`.
    pub fn add_route(&mut self, route: Box<Route<P> + Send + Sync>) {
        self.routes.push(route);
    }

    /// Allow the `Router` to access the `Routes` for this `Node` when it is
    /// selected as the lead in a single path through the `Tree`.
    pub fn borrow_routes(&self) -> &Vec<Box<Route<P> + Send + Sync>> {
        &self.routes
    }

    /// Adds a child `Node`.
    pub fn add_child(&mut self, child: Node<'n, P>) {
        self.children.push(child);
    }

    /// Sorts all children
    ///
    /// Must be called before this Node and it's children are used in traversal, generally once
    /// the owning `Tree` has been fully constructed.
    pub fn sort(&mut self) {
        self.children.sort();

        // Recursively sort all children, if any.
        for child in &mut self.children {
            child.sort();
        }
    }

    /// Determines if a child representing the exact segment provided exists.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn has_child(&self, segment: &str) -> bool {
        self.children
            .iter()
            .find(|n| n.segment == segment)
            .is_some()
    }

    /// Borrow a child that represents the exact segment provided here.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn borrow_child(&self, segment: &str) -> Option<&Node<'n, P>> {
        self.children.iter().find(|n| n.segment == segment)
    }

    /// Mutably borrow a child that represents the exact segment provided here.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn borrow_mut_child(&mut self, segment: &str) -> Option<&mut Node<'n, P>> {
        self.children.iter_mut().find(|n| n.segment == segment)
    }

    /// True if there is at least one child `Node` present
    pub fn is_parent(&self) -> bool {
        !self.children.is_empty()
    }

    /// True is there is a least one `Route` represented by this `Node`, that is it can act as a
    /// leaf in a single path through the tree.
    pub fn is_routable(&self) -> bool {
        !self.routes.is_empty()
    }

    /// Recursively traverses children attempting to locate a path of nodes which indicate they
    /// match all segments of the `Request` path and with the final `Node` of the path
    /// containing `1..n` `Route` instances for further processing by the `Router`.
    ///
    /// Only the first fully matching path is returned.
    ///
    /// Children are searched in a most to least specific order of contained segment value based on
    /// the `NodeSegmentType` value held by the `Node`:
    ///
    /// 1. Static
    /// 2. Constrained
    /// 3. Dynamic
    /// 4. Glob
    pub fn traverse<'a>(&'n self,
                        req_path_segments: &[&str])
                        -> Option<(Path<'n, 'a, P>, SegmentMapping<'n>)> {
        match self.inner_traverse(req_path_segments, vec![]) {
            Some((mut path, segment_mapping)) => {
                path.reverse();
                Some((path, segment_mapping))
            }
            None => None,
        }
    }

    #[allow(unknown_lints, type_complexity)]
    fn inner_traverse(&self,
                      req_path_segments: &[&str],
                      mut consumed_segments: Vec<String>)
                      -> Option<(Vec<&Node<'n, P>>, HashMap<&str, Vec<String>>)> {
        match req_path_segments.split_first() {
            Some((x, xs)) => {
                if self.is_match(x) {
                    if self.is_routable() && req_path_segments.len() == 1 {
                        // Leaf Node for path, start building result
                        match self.segment_type {
                            NodeSegmentType::Static => Some((vec![self], HashMap::new())),
                            _ => {
                                consumed_segments.push(String::from(*x));

                                let mut segment_mapping = HashMap::new();
                                segment_mapping.insert(self.segment(), consumed_segments);

                                Some((vec![self], segment_mapping))
                            }
                        }
                    } else {
                        // Recurse through children to continue building complete path
                        match xs.iter().peekable().peek() {
                            Some(y) => {
                                match self.children.iter().find(|c| c.is_match(y)) {
                                    Some(c) => {
                                        // Direct child, continue down tree
                                        match c.inner_traverse(xs, vec![]) {
                                            Some((mut path, mut segment_mapping)) => {
                                                match self.segment_type {
                                                    NodeSegmentType::Static => {
                                                        path.push(self);
                                                        Some((path, segment_mapping))
                                                    }
                                                    _ => {
                                                        consumed_segments.push(String::from(*x));
                                                        segment_mapping.insert(self.segment(),
                                                                               consumed_segments);
                                                        path.push(self);
                                                        Some((path, segment_mapping))
                                                    }
                                                }
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
            NodeSegmentType::Dynamic | NodeSegmentType::Glob => true,
        }
    }
}

impl<'n, P> Ord for Node<'n, P> {
    fn cmp(&self, other: &Node<'n, P>) -> Ordering {
        (&self.segment_type, &self.segment).cmp(&(&other.segment_type, &other.segment))
    }
}

impl<'n, P> PartialOrd for Node<'n, P> {
    fn partial_cmp(&self, other: &Node<'n, P>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'n, P> PartialEq for Node<'n, P> {
    fn eq(&self, other: &Node<'n, P>) -> bool {
        (&self.segment_type, &self.segment) == (&other.segment_type, &other.segment)
    }
}

impl<'n, P> Eq for Node<'n, P> {}

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::Method;
    use hyper::server::{Request, Response};

    use dispatch::Dispatcher;
    use router::request_matcher::MethodOnlyRequestMatcher;
    use router::route::{Route, RouteImpl, Extractors};
    use http::request_path::NoopRequestPathExtractor;
    use state::State;

    fn handler(state: State, _req: Request) -> (State, Response) {
        (state, Response::new())
    }

    fn handler2(state: State, _req: Request) -> (State, Response) {
        (state, Response::new())
    }

    fn get_route() -> Box<Route<()> + Send + Sync> {
        let methods = vec![Method::Get];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler), ());
        let extractors: Extractors<NoopRequestPathExtractor> = Extractors::new();
        let route = RouteImpl::new(matcher, dispatcher, extractors);
        Box::new(route)
    }

    fn test_structure<'n>() -> Node<'n, ()> {
        let mut root: Node<'n, ()> = Node::new("/", NodeSegmentType::Static);

        // Two methods, same path, same handler
        // [Get|Head]: /seg1
        let mut seg1 = Node::new("seg1", NodeSegmentType::Static);
        let methods = vec![Method::Get, Method::Head];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler), ());
        let extractors: Extractors<NoopRequestPathExtractor> = Extractors::new();
        let route = RouteImpl::new(matcher, dispatcher, extractors);
        seg1.add_route(Box::new(route));
        root.add_child(seg1);

        // Two methods, same path, different handlers
        // Post: /seg2
        let mut seg2 = Node::new("seg2", NodeSegmentType::Static);
        let methods = vec![Method::Post];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler), ());
        let extractors: Extractors<NoopRequestPathExtractor> = Extractors::new();
        let route = RouteImpl::new(matcher, dispatcher, extractors);
        seg2.add_route(Box::new(route));

        // Patch: /seg2
        let methods = vec![Method::Patch];
        let matcher = MethodOnlyRequestMatcher::new(methods);
        let dispatcher = Dispatcher::new(|| Ok(handler2), ());
        let extractors: Extractors<NoopRequestPathExtractor> = Extractors::new();
        let route = RouteImpl::new(matcher, dispatcher, extractors);
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
        match root.traverse(&["/", "seg3", "seg4"]) {
            Some((path, _)) => assert_eq!(path.last().unwrap().segment(), "seg4"),
            None => panic!("traversal should have succeeded here"),
        }

        // GET /seg3/seg4/seg5
        assert!(root.traverse(&["/", "seg3", "seg4", "seg5"]).is_none());

        // GET /seg5/seg6
        match root.traverse(&["/", "seg5", "seg6"]) {
            Some((path, _)) => assert_eq!(path.last().unwrap().segment(), "seg6"),
            None => panic!("traversal should have succeeded here"),
        }

        // GET /seg5/someval/seg7
        match root.traverse(&["/", "seg5", "someval", "seg7"]) {
            Some((path, _)) => assert_eq!(path.last().unwrap().segment(), "seg7"),
            None => panic!("traversal should have succeeded here"),
        }

        // GET /some/path/seg9/another/path
        match root.traverse(&["/", "some", "path", "seg9", "some2", "path2"]) {
            Some((path, _)) => assert_eq!(path.last().unwrap().segment(), "seg10"),
            None => panic!("traversal should have succeeded here"),
        }
    }
}
