//! Defines `Node` and `SegmentType` for `Tree`

use std::cmp::Ordering;
use std::borrow::Borrow;
use hyper::StatusCode;

use http::PercentDecoded;
use router::non_match::RouteNonMatch;
use router::route::{Delegation, Route};
use router::tree::{Path, SegmentMapping, SegmentsProcessed};
use router::tree::regex::ConstrainedSegmentRegex;
use state::{request_id, State};

/// Indicates the type of segment which is being represented by this Node.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum SegmentType {
    /// Is matched exactly (string equality) to the corresponding segment for incoming request paths.
    ///
    /// Unlike all other `NodeSegmentTypes` values determined to be associated with this segment
    /// within a `Request` path are **not** stored within `State`.
    Static,

    /// Uses the supplied regex to determine match against incoming request paths.
    Constrained {
        /// Regex used to match against a single segment of a request path.
        regex: ConstrainedSegmentRegex,
    },

    /// Matches any corresponding segment for incoming request paths.
    Dynamic,

    /// Matches multiple path segments until the end of the request path or until a child
    /// segment of the above defined types is found.
    Glob,
}

/// A recursive member of `Tree` representative of segment(s) in a routable path.
///
/// Ultimately provides `0..n` `Route` instances which are further evaluated by the `Router` if
/// the `Node` is determined to be the routable end point for a single path through the tree.
///
/// # Examples
///
/// Representing the path `/activate/workflow`.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Response, Method, StatusCode};
/// #
/// # use gotham::http::PercentDecoded;
/// # use gotham::http::response::create_response;
/// # use gotham::extractor::{NoopPathExtractor, NoopQueryStringExtractor};
/// # use gotham::pipeline::set::*;
/// # use gotham::router::route::{RouteImpl, Extractors, Delegation};
/// # use gotham::router::route::dispatch::DispatcherImpl;
/// # use gotham::state::State;
/// # use gotham::router::route::matcher::MethodOnlyRouteMatcher;
/// # use gotham::router::tree::node::{NodeBuilder, SegmentType};
/// #
/// # fn handler(state: State) -> (State, Response) {
/// #   let res = create_response(&state, StatusCode::Ok, None);
/// #   (state, res)
/// # }
/// #
/// # fn main() {
/// #  let pipeline_set = finalize_pipeline_set(new_pipeline_set());
///   let mut root_node_builder = NodeBuilder::new("/", SegmentType::Static);
///   let mut activate_node_builder = NodeBuilder::new("activate", SegmentType::Static);
///
///   let mut workflow_node = NodeBuilder::new("workflow", SegmentType::Static);
///   let route = {
///       // elided ..
/// #     let methods = vec![Method::Get];
/// #     let matcher = MethodOnlyRouteMatcher::new(methods);
/// #     let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
///       let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
///       let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
///       Box::new(route)
///   };
///   workflow_node.add_route(route);
///
///   activate_node_builder.add_child(workflow_node);
///   root_node_builder.add_child(activate_node_builder);
///
///   let root_node = root_node_builder.finalize();
///   match root_node.traverse(&[&PercentDecoded::new("/").unwrap(),
///                              &PercentDecoded::new("activate").unwrap(),
///                              &PercentDecoded::new("workflow").unwrap()])
///   {
///       Some((path, _leaf, segments_processed, _segment_mapping)) =>  {
///         assert!(path.last().unwrap().is_routable());
///         assert_eq!(segments_processed, 2);
///       }
///       None => panic!(),
///   }
/// # }
/// ```
pub struct Node {
    segment: String,
    segment_type: SegmentType,

    routes: Vec<Box<Route + Send + Sync>>,

    delegating: bool,
    children: Vec<Node>,
}

impl Node {
    /// Provides the segment this `Node` represents.
    pub fn segment(&self) -> &str {
        &self.segment
    }

    /// Provides the type of segment this `Node` represents.
    pub fn segment_type(&self) -> &SegmentType {
        &self.segment_type
    }

    /// Determines if a `Route` instance associated with this `Node` is willing to `Handle` the
    /// request.
    ///
    /// Where multiple `Route` instances could possibly handle the `Request` only the first, ordered
    /// per creation, is invoked.
    ///
    /// Where no `Route` instances will accept the `Request` the resulting Error will be the
    /// erroneous status code provided by the first `Route` instance, ordered per creation.
    ///
    /// In the situation where all these avenues are exhausted an InternalServerError will be
    /// provided.
    pub fn select_route<'a>(
        &'a self,
        state: &State,
    ) -> Result<&'a Box<Route + Send + Sync>, RouteNonMatch> {
        let mut err: Result<(), RouteNonMatch> = Ok(());

        for r in self.routes.iter() {
            match r.is_match(state) {
                Ok(()) => {
                    trace!("[{}] found matching route", request_id(state));
                    return Ok(r);
                }
                Err(e) => match err {
                    Err(e0) => err = Err(e.union(e0)),
                    Ok(()) => err = Err(e),
                },
            }
        }

        match err {
            Err(e) => {
                trace!(
                    "[{}] no matching route, using error status code from route",
                    request_id(state)
                );
                Err(e)
            }

            Ok(()) => {
                trace!(
                    "[{}] invalid state, no routes. sending internal server error",
                    request_id(state)
                );
                Err(RouteNonMatch::new(StatusCode::InternalServerError))
            }
        }
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
    /// the `SegmentType` value held by the `Node`:
    ///
    /// 1. Static
    /// 2. Constrained
    /// 3. Dynamic
    /// 4. Glob
    pub fn traverse<'r>(
        &'r self,
        req_path_segments: &'r [&PercentDecoded],
    ) -> Option<(Path<'r>, &Node, SegmentsProcessed, SegmentMapping<'r>)> {
        match self.inner_traverse(req_path_segments, vec![]) {
            Some((mut path, leaf, c, sm)) => {
                path.reverse();
                Some((path, leaf, c, sm))
            }
            None => None,
        }
    }

    #[allow(unknown_lints, type_complexity)]
    fn inner_traverse<'r>(
        &'r self,
        req_path_segments: &'r [&PercentDecoded],
        mut consumed_segments: Vec<&'r PercentDecoded>,
    ) -> Option<(Vec<&Node>, &Node, SegmentsProcessed, SegmentMapping<'r>)> {
        match req_path_segments.split_first() {
            Some((x, _)) if self.is_delegating(x) => {
                // A delegated node terminates processing, start building result
                trace!(" found delegator node `{}`", self.segment);

                let mut sm = SegmentMapping::new();
                if self.segment_type != SegmentType::Static {
                    consumed_segments.push(x);
                    sm.insert(self.segment(), consumed_segments);
                };

                Some((vec![self], self, 0, sm))
            }
            Some((x, xs)) if self.is_leaf(x, xs) => {
                trace!(" found leaf node `{}`", self.segment);

                let mut sm = SegmentMapping::new();
                if self.segment_type != SegmentType::Static {
                    consumed_segments.push(x);
                    sm.insert(self.segment(), consumed_segments);
                };

                Some((vec![self], self, 0, sm))
            }
            Some((x, xs)) if self.is_match(x) => {
                trace!(" found node `{}`", self.segment);

                let child = self.children
                    .iter()
                    .filter_map(|c| c.inner_traverse(xs, vec![]))
                    .next();

                match child {
                    Some((mut path, leaf, sp, mut sm)) => {
                        if self.segment_type != SegmentType::Static {
                            consumed_segments.push(x);
                            sm.insert(&self.segment, consumed_segments);
                            path.push(self);
                        }

                        Some((path, leaf, sp + 1, sm))
                    }

                    // If we're in a Glob consume segment and continue
                    // otherwise we've failed to find a suitable way
                    // forward.
                    None if self.segment_type == SegmentType::Glob => {
                        trace!(" continuing with glob match for segment `{}`", self.segment);
                        consumed_segments.push(x);
                        match self.inner_traverse(xs, consumed_segments) {
                            Some((nodes, n, sp, sm)) => Some((nodes, n, sp + 1, sm)),
                            None => None,
                        }
                    }
                    None => None,
                }
            }
            Some(_) => None,
            None => None,
        }
    }

    fn is_delegating(&self, req_path_segment: &PercentDecoded) -> bool {
        self.is_match(req_path_segment) && self.delegating
    }

    fn is_match(&self, req_path_segment: &PercentDecoded) -> bool {
        match self.segment_type {
            SegmentType::Static => self.segment == req_path_segment.val(),
            SegmentType::Constrained { ref regex } => {
                regex.is_match(req_path_segment.val().as_ref())
            }
            SegmentType::Dynamic | SegmentType::Glob => true,
        }
    }

    fn is_leaf(&self, s: &PercentDecoded, rs: &[&PercentDecoded]) -> bool {
        rs.is_empty() && self.is_match(s) && self.is_routable()
    }
}

/// Constructs a `Node` which is sorted and immutable.
pub struct NodeBuilder {
    segment: String,
    segment_type: SegmentType,
    routes: Vec<Box<Route + Send + Sync>>,

    delegating: bool,
    children: Vec<NodeBuilder>,
}

impl NodeBuilder {
    /// Creates new `NodeBuilder` for the given segment.
    pub fn new<S>(segment: S, segment_type: SegmentType) -> Self
    where
        S: Borrow<str>,
    {
        let segment = segment.borrow().to_owned();
        NodeBuilder {
            segment,
            segment_type,
            routes: vec![],
            children: vec![],
            delegating: false,
        }
    }

    /// Access the segment name of the `Node` under construction
    pub fn segment(&self) -> &str {
        &self.segment
    }

    /// Adds a `Route` be evaluated by the `Router` when the built `Node` is acting as a leaf in a
    /// single path through the `Tree`.
    pub fn add_route(&mut self, route: Box<Route + Send + Sync>) {
        if route.delegation() == Delegation::External {
            if !self.routes.is_empty() {
                panic!("Node which is externally delegating must have single Route");
            }

            if !self.children.is_empty() {
                panic!("Node which is externally delegating must not have existing children");
            }

            self.delegating = true;
        };

        trace!(" adding route to `{}`", self.segment());
        self.routes.push(route);
    }

    /// Adds a new child to this sub-tree structure
    pub fn add_child(&mut self, child: NodeBuilder) {
        if self.delegating {
            panic!("Node which is externally delegating must not have existing children")
        }

        trace!(
            " adding child `{}` to `{}`",
            child.segment(),
            self.segment()
        );
        self.children.push(child);
    }

    /// Determines if a child representing the exact segment provided exists.
    pub fn has_child(&self, segment: &str, segment_type: SegmentType) -> bool {
        self.children
            .iter()
            .find(|n| n.segment_type == segment_type && n.segment == segment)
            .is_some()
    }

    /// Borrow a child that represents the exact segment provided here.
    pub fn borrow_child(&self, segment: &str, segment_type: SegmentType) -> Option<&NodeBuilder> {
        self.children
            .iter()
            .find(|n| n.segment_type == segment_type && n.segment == segment)
    }

    /// Mutably borrow a child that represents the exact segment provided here.
    pub fn borrow_mut_child(
        &mut self,
        segment: &str,
        segment_type: SegmentType,
    ) -> Option<&mut NodeBuilder> {
        self.children
            .iter_mut()
            .find(|n| n.segment_type == segment_type && n.segment == segment)
    }

    /// Finalizes and sorts all internal data, including all children.
    pub fn finalize(mut self) -> Node {
        self.sort();

        let mut children = self.children
            .drain(..)
            .map(|c| c.finalize())
            .collect::<Vec<Node>>();

        children.shrink_to_fit();
        self.routes.shrink_to_fit();

        Node {
            segment: self.segment,
            segment_type: self.segment_type,
            routes: self.routes,
            delegating: self.delegating,
            children,
        }
    }

    // Sorts all children per `PartialEq` and `PartialOrd` implementations.
    //
    // Final ordering of Children is based on most to least specific SegmentType as follows:
    //
    // 1. Static
    // 2. Constrained
    // 3. Dynamic
    // 4. Glob
    fn sort(&mut self) {
        self.children.sort();

        for child in &mut self.children {
            child.sort();
        }
    }
}

impl Ord for NodeBuilder {
    fn cmp(&self, other: &NodeBuilder) -> Ordering {
        (&self.segment_type, &self.segment).cmp(&(&other.segment_type, &other.segment))
    }
}

impl PartialOrd for NodeBuilder {
    fn partial_cmp(&self, other: &NodeBuilder) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NodeBuilder {
    fn eq(&self, other: &NodeBuilder) -> bool {
        (&self.segment_type, &self.segment) == (&other.segment_type, &other.segment)
    }
}

impl Eq for NodeBuilder {}

#[cfg(test)]
mod tests {
    use super::*;

    use std::panic::RefUnwindSafe;

    use hyper::{Headers, Method, Response};

    use pipeline::set::*;
    use router::route::dispatch::DispatcherImpl;
    use router::route::matcher::MethodOnlyRouteMatcher;
    use router::route::{Extractors, Route, RouteImpl};
    use extractor::{NoopPathExtractor, NoopQueryStringExtractor};
    use http::request::path::RequestPathSegments;
    use state::{set_request_id, State};

    fn handler(state: State) -> (State, Response) {
        (state, Response::new())
    }

    fn get_route<P>(pipeline_set: PipelineSet<P>) -> Box<Route + Send + Sync>
    where
        P: Send + Sync + RefUnwindSafe + 'static,
    {
        let methods = vec![Method::Get];
        let matcher = MethodOnlyRouteMatcher::new(methods);
        let dispatcher = DispatcherImpl::new(|| Ok(handler), (), pipeline_set);
        let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
        let route = RouteImpl::new(
            matcher,
            Box::new(dispatcher),
            extractors,
            Delegation::Internal,
        );
        Box::new(route)
    }

    fn get_delegated_route<P>(pipeline_set: PipelineSet<P>) -> Box<Route + Send + Sync>
    where
        P: Send + Sync + RefUnwindSafe + 'static,
    {
        let methods = vec![Method::Get];
        let matcher = MethodOnlyRouteMatcher::new(methods);
        let dispatcher = DispatcherImpl::new(|| Ok(handler), (), pipeline_set);
        let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
        let route = RouteImpl::new(
            matcher,
            Box::new(dispatcher),
            extractors,
            Delegation::External,
        );
        Box::new(route)
    }

    fn test_structure() -> NodeBuilder {
        let mut root: NodeBuilder = NodeBuilder::new("/", SegmentType::Static);
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());

        // Two methods, same path, same handler
        // [Get|Head]: /seg1
        let mut seg1 = NodeBuilder::new("seg1", SegmentType::Static);
        let methods = vec![Method::Get, Method::Head];
        let matcher = MethodOnlyRouteMatcher::new(methods);
        let dispatcher = DispatcherImpl::new(|| Ok(handler), (), pipeline_set.clone());
        let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
        let route = RouteImpl::new(
            matcher,
            Box::new(dispatcher),
            extractors,
            Delegation::Internal,
        );
        seg1.add_route(Box::new(route));
        root.add_child(seg1);

        // Two methods, same path, different handlers
        // Post: /seg2
        let mut seg2 = NodeBuilder::new("seg2", SegmentType::Static);
        let methods = vec![Method::Post];
        let matcher = MethodOnlyRouteMatcher::new(methods);
        let dispatcher = DispatcherImpl::new(|| Ok(handler), (), pipeline_set.clone());
        let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
        let route = RouteImpl::new(
            matcher,
            Box::new(dispatcher),
            extractors,
            Delegation::Internal,
        );
        seg2.add_route(Box::new(route));

        // Patch: /seg2
        let methods = vec![Method::Patch];
        let matcher = MethodOnlyRouteMatcher::new(methods);
        let dispatcher = DispatcherImpl::new(|| Ok(handler), (), pipeline_set.clone());
        let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
        let route = RouteImpl::new(
            matcher,
            Box::new(dispatcher),
            extractors,
            Delegation::Internal,
        );
        seg2.add_route(Box::new(route));
        root.add_child(seg2);

        // Ensure basic traversal
        // Get: /seg3/seg4
        let mut seg3 = NodeBuilder::new("seg3", SegmentType::Static);
        let mut seg4 = NodeBuilder::new("seg4", SegmentType::Static);
        seg4.add_route(get_route(pipeline_set.clone()));
        seg3.add_child(seg4);
        root.add_child(seg3);

        // Ensure regex matching works and that it's anchored to the segment and does not allow for
        // overzealous matching
        // GET: /resource/<id> where id: [0-9]+
        let mut seg_resource = NodeBuilder::new("resource", SegmentType::Static);
        let mut seg_id = NodeBuilder::new(
            "id",
            SegmentType::Constrained {
                regex: ConstrainedSegmentRegex::new("[0-9]+"),
            },
        );
        seg_id.add_route(get_route(pipeline_set.clone()));
        seg_resource.add_child(seg_id);
        root.add_child(seg_resource);

        // Ensure traversal will backtrack and find the correct path if it goes down an ultimately
        // invalid branch, in this case seg6 initially being matched by the dynamic handler segdyn1
        // which matches every segment it sees.
        //
        // Get /seg5/:segdyn1/seg7
        // Get /seg5/seg6
        let mut seg5 = NodeBuilder::new("seg5", SegmentType::Static);
        let mut seg6 = NodeBuilder::new("seg6", SegmentType::Static);
        seg6.add_route(get_route(pipeline_set.clone()));

        let mut segdyn1 = NodeBuilder::new(":segdyn1", SegmentType::Dynamic);
        let mut seg7 = NodeBuilder::new("seg7", SegmentType::Static);
        seg7.add_route(get_route(pipeline_set.clone()));

        // Ensure traversal will respect Globs
        let mut seg8 = NodeBuilder::new("seg8", SegmentType::Glob);
        let mut seg9 = NodeBuilder::new("seg9", SegmentType::Static);

        let mut seg10 = NodeBuilder::new(String::from("seg10"), SegmentType::Glob);
        seg10.add_route(get_route(pipeline_set.clone()));

        seg9.add_child(seg10);
        seg8.add_child(seg9);
        root.add_child(seg8);

        segdyn1.add_child(seg7);
        seg5.add_child(segdyn1);
        seg5.add_child(seg6);
        root.add_child(seg5);

        root
    }

    #[test]
    fn manages_children() {
        let root_node_builder = test_structure();

        assert!(
            root_node_builder
                .borrow_child("seg1", SegmentType::Static)
                .is_some()
        );
        assert!(
            root_node_builder
                .borrow_child("seg2", SegmentType::Static)
                .is_some()
        );
        assert!(
            root_node_builder
                .borrow_child("seg1", SegmentType::Dynamic)
                .is_none()
        );
        assert!(
            root_node_builder
                .borrow_child("seg0", SegmentType::Static)
                .is_none()
        );
    }

    #[test]
    fn traverses_children() {
        let root = test_structure().finalize();

        // GET /seg3/seg4
        let rs = RequestPathSegments::new("/seg3/seg4");
        match root.traverse(&rs.segments()) {
            Some((path, leaf, sp, _)) => {
                assert_eq!(path.last().unwrap().segment(), "seg4");
                assert_eq!(path.last().unwrap().segment(), leaf.segment());
                assert_eq!(sp, 2);
            }
            None => panic!("traversal should have succeeded here"),
        }

        // GET /seg3/seg4/seg5
        let rs = RequestPathSegments::new("/seg3/seg4/seg5");
        assert!(root.traverse(&rs.segments()).is_none());

        // GET /seg5/seg6
        let rs = RequestPathSegments::new("/seg5/seg6");
        match root.traverse(&rs.segments()) {
            Some((path, _, sp, _)) => {
                assert_eq!(path.last().unwrap().segment(), "seg6");
                assert_eq!(sp, 2);
            }
            None => panic!("traversal should have succeeded here"),
        }

        // GET /seg5/someval/seg7
        let rs = RequestPathSegments::new("/seg5/someval/seg7");
        match root.traverse(&rs.segments()) {
            Some((path, _, sp, _)) => {
                assert_eq!(path.last().unwrap().segment(), "seg7");
                assert_eq!(sp, 3);
            }
            None => panic!("traversal should have succeeded here"),
        }

        // GET /some/path/seg9/another/path
        let rs = RequestPathSegments::new("/some/path/seg9/another/branch");
        match root.traverse(&rs.segments()) {
            Some((path, _, sp, _)) => {
                assert_eq!(path.last().unwrap().segment(), "seg10");
                assert_eq!(sp, 5);
            }
            None => panic!("traversal should have succeeded here"),
        }

        let rs = RequestPathSegments::new("/resource/5001");
        let expected_segment = "id";
        match root.traverse(&rs.segments()) {
            Some((path, _, sp, _)) => {
                assert_eq!(path.last().unwrap().segment(), expected_segment);
                assert_eq!(sp, 2);
            }
            None => panic!("traversal should have succeeded here"),
        }
    }

    #[test]
    fn non_matching_routes_allow_list_tests() {
        let root = test_structure().finalize();

        let mut state = State::new();
        state.put(Method::Options);
        state.put(Headers::new());
        set_request_id(&mut state);

        let rs = RequestPathSegments::new("/seg2");
        match root.traverse(&rs.segments()) {
            Some((_, node, _, _)) => match node.select_route(&state) {
                Err(e) => {
                    let (status, mut allow_list) = e.deconstruct();
                    assert_eq!(status, StatusCode::MethodNotAllowed);
                    allow_list.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
                    assert_eq!(allow_list, vec![Method::Patch, Method::Post]);
                }
                Ok(_) => panic!("expected mismatched route to test allow header"),
            },
            None => panic!("traversal should have succeeded here"),
        }

        let rs = RequestPathSegments::new("/resource/100");
        match root.traverse(&rs.segments()) {
            Some((_, node, _, _)) => match node.select_route(&state) {
                Err(e) => {
                    let (status, mut allow_list) = e.deconstruct();
                    assert_eq!(status, StatusCode::MethodNotAllowed);
                    allow_list.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
                    assert_eq!(allow_list, vec![Method::Get]);
                }
                Ok(_) => panic!("expected mismatched route to test allow header"),
            },
            None => panic!("traversal should have succeeded here"),
        }
    }

    #[test]
    #[should_panic(expected = "Node which is externally delegating must not have existing children")]
    fn panics_when_delegated_node_adds_children() {
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut seg1 = NodeBuilder::new("seg1", SegmentType::Static);
        let seg2 = NodeBuilder::new("seg2", SegmentType::Static);

        seg1.add_route(get_delegated_route(pipeline_set));
        seg1.add_child(seg2);
    }

    #[test]
    #[should_panic(expected = "Node which is externally delegating must not have existing children")]
    fn panics_when_node_with_children_is_provided_delegated_route() {
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut seg1 = NodeBuilder::new("seg1", SegmentType::Static);
        let seg2 = NodeBuilder::new("seg2", SegmentType::Static);

        seg1.add_child(seg2);
        seg1.add_route(get_delegated_route(pipeline_set));
    }

    #[test]
    #[should_panic(expected = "Node which is externally delegating must have single Route")]
    fn panics_when_node_with_a_route_adds_another() {
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut seg1 = NodeBuilder::new("seg1", SegmentType::Static);

        seg1.add_route(get_delegated_route(pipeline_set.clone()));
        seg1.add_route(get_delegated_route(pipeline_set));
    }
}
