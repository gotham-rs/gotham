//! Defines a hierarchial `Tree` with subtrees of `Node`.

use std::collections::HashMap;

use http::PercentDecoded;
use router::route::Route;
use router::tree::node::{Node, NodeBuilder, SegmentType};

pub mod node;
pub mod regex;

/// A depth ordered `Vec` of `Node` instances that create a routable path through the `Tree` for the
/// matched `Request` path.
type Path<'a> = Vec<&'a Node>;

/// Number of segments from a `Request` path that are considered to have been processed
/// by an `Router` traversing its `Tree`.
type SegmentsProcessed = usize;

/// Mapping of segment names into the collection of values for that segment.
pub type SegmentMapping<'r> = HashMap<&'r str, Vec<&'r PercentDecoded>>;

/// A hierarchical structure that provides a root `Node` and subtrees of linked nodes
/// that represent valid `Request` paths.
///
/// The `Tree` is created by the `gotham::router::builder` API and used internally by the `Router`
/// to determine the valid `Route` instances for a request path before dispatch.
pub struct Tree {
    root: Node,
}

impl Tree {
    /// Attempt to acquire a path from the `Tree` which matches the `Request` path and is routable.
    pub(crate) fn traverse<'r>(
        &'r self,
        req_path_segments: &'r [&PercentDecoded],
    ) -> Option<(Path<'r>, &Node, SegmentsProcessed, SegmentMapping<'r>)> {
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
        TreeBuilder {
            root: NodeBuilder::new("/", SegmentType::Static),
        }
    }

    /// Adds a direct child to the root of the `TreeBuilder`.
    pub fn add_child(&mut self, child: NodeBuilder) {
        self.root.add_child(child);
    }

    /// Determines if a child `Node` representing the exact segment provided exists at the root of
    /// the `Tree`.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn has_child(&self, segment: &str, segment_type: SegmentType) -> bool {
        self.root.has_child(segment, segment_type)
    }

    /// Borrow the root `NodeBuilder` as mutable.
    pub fn borrow_root_mut(&mut self) -> &mut NodeBuilder {
        &mut self.root
    }

    /// Adds a `Route` be evaluated by the `Router` when the root of the `Tree` is requested.
    pub fn add_route(&mut self, route: Box<Route + Send + Sync>) {
        self.root.add_route(route);
    }

    /// Finalizes and sorts all internal data and creates a Tree for use with a `Router`.
    pub fn finalize(self) -> Tree {
        Tree {
            root: self.root.finalize(),
        }
    }
}

#[cfg(test)]
mod tests {
    use hyper::{Method, Response, StatusCode};

    use extractor::{NoopPathExtractor, NoopQueryStringExtractor};
    use http::request::path::RequestPathSegments;
    use http::response::create_response;
    use router::route::matcher::MethodOnlyRouteMatcher;
    use router::route::dispatch::DispatcherImpl;
    use router::route::{Delegation, Extractors, RouteImpl};
    use state::State;
    use pipeline::set::*;

    use super::*;

    fn handler(state: State) -> (State, Response) {
        let res = create_response(&state, StatusCode::Ok, None);
        (state, res)
    }

    #[test]
    fn tree_traversal_tests() {
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut tree_builder: TreeBuilder = TreeBuilder::new();

        let mut activate_node_builder = NodeBuilder::new("activate", SegmentType::Static);

        let mut thing_node_builder = NodeBuilder::new("thing", SegmentType::Dynamic);
        let thing_route = {
            let methods = vec![Method::Get];
            let matcher = MethodOnlyRouteMatcher::new(methods);
            let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
            let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> =
                Extractors::new();
            let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
            Box::new(route)
        };
        thing_node_builder.add_route(thing_route);

        activate_node_builder.add_child(thing_node_builder);
        tree_builder.add_child(activate_node_builder);

        let tree = tree_builder.finalize();

        let request_path_segments = RequestPathSegments::new("/%61ctiv%61te/workflow5");
        match tree.traverse(request_path_segments.segments().as_slice()) {
            Some((path, leaf, segments_processed, segment_mapping)) => {
                assert!(path.last().unwrap().is_routable());
                assert_eq!(path.last().unwrap().segment(), leaf.segment());
                assert_eq!(segments_processed, 2);
                assert_eq!(
                    segment_mapping
                        .get("thing")
                        .unwrap()
                        .last()
                        .unwrap()
                        .as_ref(),
                    "workflow5"
                );
            }
            None => panic!(),
        }

        assert!(
            tree.traverse(&[&PercentDecoded::new("/").unwrap()])
                .is_none()
        );
        assert!(
            tree.traverse(&[&PercentDecoded::new("/activate").unwrap()])
                .is_none()
        );
    }
}
