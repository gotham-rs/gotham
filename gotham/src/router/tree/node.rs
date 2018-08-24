//! Defines `Node` for `Tree`.

use hyper::{Body, StatusCode};

use helpers::http::PercentDecoded;
use router::non_match::RouteNonMatch;
use router::route::{Delegation, Route};
use router::tree::segment::{SegmentMapping, SegmentType};
use state::{request_id, State};

use std::cmp::Ordering;
use std::collections::HashMap;

/// A recursive member of `Tree`, representative of segment(s) in a request path.
///
/// Each node includes `0..n` `Route` instances, which can be further evaluated by the `Router`
/// based on a match. Every node may also have `0..n` children to provide the recursive tree
/// representation.
pub struct Node {
    segment: String,
    segment_type: SegmentType,
    routes: Vec<Box<Route<ResBody = Body> + Send + Sync>>,
    children: Vec<Node>,
}

impl Node {
    /// Creates new `Node` for the given segment and type.
    pub fn new(segment: &str, segment_type: SegmentType) -> Self {
        Node {
            segment_type,
            segment: segment.to_string(),
            routes: vec![],
            children: vec![],
        }
    }

    /// Adds a new child `Node` instance to this `Node`.
    pub fn add_child(&mut self, node: Node) -> &mut Self {
        self.children.push(node);
        self.children.sort();
        self
    }

    /// Adds a `Route` to this `Node`, to be potentially evaluated by the `Router`.
    pub fn add_route(&mut self, route: Box<Route<ResBody = Body> + Send + Sync>) -> &mut Self {
        self.routes.push(route);
        self
    }

    /// Borrows a child `Node` based on the defined segment bounds.
    pub fn borrow_child(&self, segment: &str, segment_type: SegmentType) -> Option<&Node> {
        self.children
            .iter()
            .find(|n| n.segment_type == segment_type && n.segment == segment)
    }

    /// Borrows a mutable child `Node` based on the defined segment bounds.
    pub fn borrow_child_mut(
        &mut self,
        segment: &str,
        segment_type: SegmentType,
    ) -> Option<&mut Node> {
        self.children
            .iter_mut()
            .find(|n| n.segment_type == segment_type && n.segment == segment)
    }

    /// Determines if a child exists based on the defined segment bounds.
    pub fn has_child(&self, segment: &str, segment_type: SegmentType) -> bool {
        self.borrow_child(segment, segment_type).is_some()
    }

    /// Determines if this `Node` has any valid `Route` values attached.
    pub fn is_routable(&self) -> bool {
        !self.routes.is_empty()
    }

    /// Traverses this `Node` and its children, attempting to a locate a path of `Node` instances
    /// which match all segments of the provided `Request` path. The final `Node` must have at
    /// least a single `Route` attached in order to be returned.
    ///
    /// Only the first matching path is returned from this method, and the value is wrapped in
    /// an `Option` as there may be no matching node.
    ///
    /// Children are searched in a most to least specific order of segments, based on the node
    /// `SegmentType` value:
    ///
    /// 1. Static
    /// 2. Constrained
    /// 3. Dynamic
    /// 4. Glob
    ///
    /// This method is a wrapping of an internal recursive implementation to mask the required
    /// types needed for the recursion.
    pub fn match_node<'a>(
        &'a self,
        segments: &'a [PercentDecoded],
    ) -> Option<(&'a Node, SegmentMapping<'a>, usize)> {
        // accumulators for recursion
        let mut params = HashMap::new();
        let mut processed = 0;

        // process and map the results through to the required form
        self.inner_match_node(segments, &mut params, &mut processed)
            .map(|node| (node, params, processed))
    }

    /// Retrieves a reference to the contained segment value.
    ///
    /// This is required for lifetime related annotations.
    pub fn segment<'a>(&'a self) -> &'a str {
        &self.segment
    }

    /// Determines if a `Route` instance associated with this `Node` is willing to `Handle` the
    /// request.
    ///
    /// Where multiple `Route` instances could possibly handle the `Request` only the first, ordered
    /// per creation, is invoked.
    ///
    /// Where no `Route` instances will accept the `Request` the resulting Error will be the
    /// union of the `RouteNonMatch` values returned from each `Route`.
    ///
    /// In the situation where all these avenues are exhausted an InternalServerError will be
    /// provided.
    pub fn select_route(
        &self,
        state: &State,
    ) -> Result<&Box<Route<ResBody = Body> + Send + Sync>, RouteNonMatch> {
        let mut err = Ok(());

        // check for matching routes
        for r in self.routes.iter() {
            match r.is_match(state) {
                Ok(()) => {
                    trace!("[{}] found matching route", request_id(state));
                    return Ok(r);
                }
                Err(e) => {
                    // concat errors
                    err = match err {
                        Err(e0) => Err(e.union(e0)),
                        Ok(()) => Err(e),
                    }
                }
            }
        }

        // unpack required for types
        if let Err(e) = err {
            trace!(
                "[{}] no matching route, using error status code from route",
                request_id(state)
            );
            return Err(e);
        }

        trace!(
            "[{}] invalid state, no routes. sending internal server error",
            request_id(state)
        );

        // error because we shouldn't arrive here due to match_node/1
        Err(RouteNonMatch::new(StatusCode::INTERNAL_SERVER_ERROR))
    }

    /// Recursive implementation of `match_route` to populate parameters and keep
    /// track of the number of visited nodes.
    ///
    /// There's space for optimizations in here (perhaps), but it seems to perform
    /// faster than the previous implementation of the router, so all is well for now.
    fn inner_match_node<'a>(
        &'a self,
        segments: &'a [PercentDecoded],
        params: &mut SegmentMapping<'a>,
        processed: &mut usize,
    ) -> Option<&'a Node> {
        let next_segment = segments.split_first();

        // stop if we're done
        if let None = next_segment {
            if !self.is_routable() {
                return None;
            }
            return Some(self);
        }

        // check for external delegates, and stop
        if let Some(route) = self.routes.first() {
            if route.delegation() == Delegation::External {
                return Some(self);
            }
        }

        let (segment, remaining) = next_segment.unwrap();

        *processed += 1;

        // check all children first
        for child in &self.children {
            match child.segment_type {
                // Globbing matches everything, so we append the segment value
                // to the parameters against the child segment name.
                SegmentType::Glob => {
                    params
                        .entry(&child.segment)
                        .or_insert_with(|| vec![])
                        .push(&segment);
                }

                // Static matches based on a raw string match, so we simply
                // compare the value of the current segment with that of the
                // child node we're currently iterating.
                SegmentType::Static => {
                    // check for raw string match
                    if child.segment != segment.as_ref() {
                        continue;
                    }
                }

                // Constrained matches are based on a contained pattern the
                // segment value must match. If the segment matches, we need
                // to make sure to store the value inside the parameters map.
                SegmentType::Constrained { ref regex } => {
                    // check for regex matching
                    if !regex.is_match(&segment.as_ref()) {
                        continue;
                    }
                    // if there's a match, store the value
                    params.insert(&child.segment, vec![&segment]);
                }

                // Dynamic matches match every value, so we just attach the
                // segment value to the parameters list (just like with the
                // constrained type).
                SegmentType::Dynamic => {
                    // if there's a match, store the value
                    params.insert(&child.segment, vec![&segment]);
                }
            };

            // If we hit this point, we've determined that the child node is
            // the correct node to delegate to, so we continue the recursion
            // on the child node, passing in the same parameters.
            return child.inner_match_node(remaining, params, processed);
        }

        // If there are no children, but this is a globbing node, then we can
        // continue the nesting by just shifting the path segments and calling
        // `inner_match_node` on ourself again (to simulate wildcards).
        if let SegmentType::Glob = self.segment_type {
            // push the segment to the parameters of the glob
            if let Some(path) = params.get_mut(self.segment()) {
                path.push(&segment);
            }
            // call again, but after shifting the segments to the next
            return self.inner_match_node(remaining, params, processed);
        }

        None
    }
}

impl Eq for Node {}
impl PartialEq for Node {
    /// Compares two `Node` values for equality based on the segments they represent.
    fn eq(&self, other: &Node) -> bool {
        self.segment == other.segment && self.segment_type == other.segment_type
    }
}

impl Ord for Node {
    /// Compares two `Node` values to determine an appropriate `Ordering`.
    fn cmp(&self, other: &Node) -> Ordering {
        (&self.segment_type, &self.segment).cmp(&(&other.segment_type, &other.segment))
    }
}

impl PartialOrd for Node {
    /// Compares two `Node` values to determine an appropriate `Ordering`.
    fn partial_cmp(&self, other: &Node) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::panic::RefUnwindSafe;

    use hyper::{HeaderMap, Method, Response};

    use extractor::{NoopPathExtractor, NoopQueryStringExtractor};
    use helpers::http::request::path::RequestPathSegments;
    use helpers::http::PercentDecoded;
    use pipeline::set::*;
    use router::route::dispatch::DispatcherImpl;
    use router::route::matcher::MethodOnlyRouteMatcher;
    use router::route::{Delegation, Extractors, Route, RouteImpl};
    use router::tree::regex::ConstrainedSegmentRegex;
    use state::{set_request_id, State};

    fn handler(state: State) -> (State, Response<Body>) {
        (state, Response::new(Body::empty()))
    }

    fn get_route<P>(pipeline_set: PipelineSet<P>) -> Box<Route<ResBody = Body> + Send + Sync>
    where
        P: Send + Sync + RefUnwindSafe + 'static,
    {
        let methods = vec![Method::GET];
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

    fn test_structure() -> Node {
        let mut root = Node::new("/", SegmentType::Static);
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());

        // Two methods, same path, same handler
        // [Get|Head]: /seg1
        let mut seg1 = Node::new("seg1", SegmentType::Static);
        let methods = vec![Method::GET, Method::HEAD];
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
        let mut seg2 = Node::new("seg2", SegmentType::Static);
        let methods = vec![Method::POST];
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
        let methods = vec![Method::PATCH];
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
        let mut seg3 = Node::new("seg3", SegmentType::Static);
        let mut seg4 = Node::new("seg4", SegmentType::Static);
        seg4.add_route(get_route(pipeline_set.clone()));
        seg3.add_child(seg4);
        root.add_child(seg3);

        // Ensure regex matching works and that it's anchored to the segment and does not allow for
        // overzealous matching
        // GET: /resource/<id> where id: [0-9]+
        let mut seg_resource = Node::new("resource", SegmentType::Static);
        let mut seg_id = Node::new(
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
        let mut seg5 = Node::new("seg5", SegmentType::Static);
        let mut seg6 = Node::new("seg6", SegmentType::Static);
        seg6.add_route(get_route(pipeline_set.clone()));

        let mut segdyn1 = Node::new(":segdyn1", SegmentType::Dynamic);
        let mut seg7 = Node::new("seg7", SegmentType::Static);
        seg7.add_route(get_route(pipeline_set.clone()));

        // Ensure traversal will respect Globs
        let mut seg8 = Node::new("seg8", SegmentType::Glob);
        let mut seg9 = Node::new("seg9", SegmentType::Static);

        let mut seg10 = Node::new("seg10", SegmentType::Glob);
        seg10.add_route(get_route(pipeline_set.clone()));

        segdyn1.add_child(seg7);
        seg5.add_child(seg6);
        seg5.add_child(segdyn1);
        root.add_child(seg5);

        seg9.add_child(seg10);
        seg8.add_child(seg9);
        root.add_child(seg8);

        root
    }

    #[test]
    fn manages_children() {
        let root = test_structure();

        assert!(root.borrow_child("seg1", SegmentType::Static).is_some());
        assert!(root.borrow_child("seg2", SegmentType::Static).is_some());
        assert!(root.borrow_child("seg1", SegmentType::Dynamic).is_none());
        assert!(root.borrow_child("seg0", SegmentType::Static).is_none());
    }

    #[test]
    fn traverses_children() {
        let root = test_structure();

        // GET /seg3/seg4
        let rs = RequestPathSegments::new("/seg3/seg4");
        match root.match_node(&rs.segments()) {
            Some((node, _params, processed)) => {
                assert_eq!(node.segment, "seg4");
                assert_eq!(processed, 2);
            }
            None => panic!("traversal should have succeeded here"),
        }

        // GET /seg3/seg4/seg5
        let rs = RequestPathSegments::new("/seg3/seg4/seg5");
        assert!(root.match_node(&rs.segments()).is_none());

        // GET /seg5/seg6
        let rs = RequestPathSegments::new("/seg5/seg6");
        match root.match_node(&rs.segments()) {
            Some((node, _params, processed)) => {
                assert_eq!(node.segment, "seg6");
                assert_eq!(processed, 2);
            }
            None => panic!("traversal should have succeeded here"),
        }

        // GET /seg5/someval/seg7
        let rs = RequestPathSegments::new("/seg5/someval/seg7");
        match root.match_node(&rs.segments()) {
            Some((node, _params, processed)) => {
                assert_eq!(node.segment, "seg7");
                assert_eq!(processed, 3);
            }
            None => panic!("traversal should have succeeded here"),
        }

        // GET /some/path/seg9/another/path
        let rs = RequestPathSegments::new("/some/path/seg9/another/branch");
        match root.match_node(&rs.segments()) {
            Some((node, _params, processed)) => {
                assert_eq!(node.segment, "seg10");
                assert_eq!(processed, 5);
            }
            None => panic!("traversal should have succeeded here"),
        }

        let rs = RequestPathSegments::new("/resource/5001");
        let expected_segment = "id";
        match root.match_node(&rs.segments()) {
            Some((node, _params, processed)) => {
                assert_eq!(node.segment, expected_segment);
                assert_eq!(processed, 2);
            }
            None => panic!("traversal should have succeeded here"),
        }
    }

    #[test]
    fn non_matching_routes_allow_list_tests() {
        let root = test_structure();

        let mut state = State::new();
        state.put(Method::OPTIONS);
        state.put(HeaderMap::new());
        set_request_id(&mut state);

        let rs = RequestPathSegments::new("/seg2");
        match root.match_node(&rs.segments()) {
            Some((node, _params, _processed)) => match node.select_route(&state) {
                Err(e) => {
                    let (status, mut allow_list) = e.deconstruct();
                    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
                    allow_list.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
                    assert_eq!(allow_list, vec![Method::PATCH, Method::POST]);
                }
                Ok(_) => panic!("expected mismatched route to test allow header"),
            },
            None => panic!("traversal should have succeeded here"),
        }

        let rs = RequestPathSegments::new("/resource/100");
        match root.match_node(&rs.segments()) {
            Some((node, _params, _processed)) => match node.select_route(&state) {
                Err(e) => {
                    let (status, mut allow_list) = e.deconstruct();
                    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
                    allow_list.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
                    assert_eq!(allow_list, vec![Method::GET]);
                }
                Ok(_) => panic!("expected mismatched route to test allow header"),
            },
            None => panic!("traversal should have succeeded here"),
        }
    }

    #[test]
    fn node_traversal_tests() {
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut root_node_builder = Node::new("/", SegmentType::Static);
        let mut activate_node_builder = Node::new("activate", SegmentType::Static);

        let mut workflow_node = Node::new("workflow", SegmentType::Static);
        let route = {
            let methods = vec![Method::GET];
            let matcher = MethodOnlyRouteMatcher::new(methods);
            let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
            let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> =
                Extractors::new();
            let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
            Box::new(route)
        };
        workflow_node.add_route(route);

        activate_node_builder.add_child(workflow_node);
        root_node_builder.add_child(activate_node_builder);

        let root_node = root_node_builder;
        match root_node.match_node(&[
            PercentDecoded::new("activate").unwrap(),
            PercentDecoded::new("workflow").unwrap(),
        ]) {
            Some((node, _params, processed)) => {
                assert!(node.is_routable());
                assert_eq!(processed, 2)
            }
            None => panic!(),
        }
    }
}
