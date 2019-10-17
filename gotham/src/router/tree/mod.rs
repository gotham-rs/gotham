//! Defines a hierarchial `Tree` with subtrees of `Node`.

use crate::helpers::http::PercentDecoded;
use crate::router::route::Route;
use crate::router::tree::node::Node;
use crate::router::tree::segment::{SegmentMapping, SegmentType};
use hyper::Body;
use log::trace;

pub mod node;
pub mod regex;
pub mod segment;

/// A hierarchical structure that provides a root `Node` and subtrees of linked nodes
/// that represent valid `Request` paths.
///
/// The `Tree` is created by the `gotham::router::builder` API and used internally by the `Router`
/// to determine the valid `Route` instances for a request path before dispatch.
pub struct Tree {
    root: Node,
}

impl Tree {
    /// Creates a new `Tree` and root `Node`.
    pub fn new() -> Self {
        trace!(" creating new tree");
        Tree {
            root: Node::new("/", SegmentType::Static),
        }
    }

    /// Adds a direct child to the root of the `Tree`.
    pub fn add_child(&mut self, child: Node) {
        self.root.add_child(child);
    }

    /// Adds a `Route` be evaluated by the `Router` when the root of the `Tree` is requested.
    pub fn add_route(&mut self, route: Box<dyn Route<ResBody = Body> + Send + Sync>) {
        self.root.add_route(route);
    }

    /// Borrow the root `NodeBuilder` as mutable.
    pub fn borrow_root_mut(&mut self) -> &mut Node {
        &mut self.root
    }

    /// Determines if a child `Node` representing the exact segment provided exists at the root of
    /// the `Tree`.
    ///
    /// To be used in building a `Tree` structure only.
    pub fn has_child(&self, segment: &str, segment_type: SegmentType) -> bool {
        self.root.has_child(segment, segment_type)
    }

    /// Attempt to acquire a path from the `Tree` which matches the `Request` path and is routable.
    pub(crate) fn traverse<'a>(
        &'a self,
        req_path_segments: &'a [PercentDecoded],
    ) -> Option<(&Node, SegmentMapping<'a>, usize)> {
        trace!(" starting tree traversal");
        self.root.match_node(req_path_segments)
    }
}

#[cfg(test)]
mod tests {
    use hyper::{Method, Response, StatusCode};

    use crate::extractor::{NoopPathExtractor, NoopQueryStringExtractor};
    use crate::helpers::http::request::path::RequestPathSegments;
    use crate::helpers::http::response::create_empty_response;
    use crate::pipeline::set::*;
    use crate::router::route::dispatch::DispatcherImpl;
    use crate::router::route::matcher::MethodOnlyRouteMatcher;
    use crate::router::route::{Delegation, Extractors, RouteImpl};
    use crate::state::State;

    use super::*;

    fn handler(state: State) -> (State, Response<Body>) {
        let res = create_empty_response(&state, StatusCode::OK);
        (state, res)
    }

    #[test]
    fn tree_traversal_tests() {
        let pipeline_set = finalize_pipeline_set(new_pipeline_set());
        let mut tree = Tree::new();

        let mut activate_node_builder = Node::new("activate", SegmentType::Static);

        let mut thing_node_builder = Node::new("thing", SegmentType::Dynamic);
        let thing_route = {
            let methods = vec![Method::GET];
            let matcher = MethodOnlyRouteMatcher::new(methods);
            let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (), pipeline_set));
            let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> =
                Extractors::new();
            let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
            Box::new(route)
        };
        thing_node_builder.add_route(thing_route);

        activate_node_builder.add_child(thing_node_builder);
        tree.add_child(activate_node_builder);

        let request_path_segments = RequestPathSegments::new("/%61ctiv%61te/workflow5");
        match tree.traverse(request_path_segments.segments().as_slice()) {
            Some((node, params, processed)) => {
                assert!(node.is_routable());
                assert_eq!(processed, 2);
                assert_eq!(
                    params.get("thing").unwrap().last().unwrap().as_ref(),
                    "workflow5"
                );
            }
            None => panic!(),
        }

        assert!(tree
            .traverse(&[PercentDecoded::new("/").unwrap()])
            .is_none());
        assert!(tree
            .traverse(&[PercentDecoded::new("/activate").unwrap()])
            .is_none());
    }
}
