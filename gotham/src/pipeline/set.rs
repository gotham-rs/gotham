//! Defines the types for adding multiple pipelines into a `PipelineSet` and retaining a handle to
//! each pipeline for constructing a `PipelineHandleChain`.

use borrow_bag::BorrowBag;
use std::sync::Arc;

/// Represents the set of all `Pipeline` instances that are available for use when building a
/// `Router`. A `PipelineSet` is "frozen".
pub type PipelineSet<P> = Arc<BorrowBag<P>>;

/// A set of `Pipeline` instances that is currently being defined, and can have more `Pipeline`
/// instances added.
pub type EditablePipelineSet<P> = BorrowBag<P>;

/// Create an empty set of `Pipeline` instances.
///
/// See BorrowBag#add to insert new `Pipeline` instances.
pub fn new_pipeline_set() -> EditablePipelineSet<()> {
    BorrowBag::new()
}

/// Wraps the current set of `Pipeline` instances into a thread-safe reference counting pointer for
/// use with the `Router`.
pub fn finalize_pipeline_set<P>(eps: EditablePipelineSet<P>) -> PipelineSet<P> {
    Arc::new(eps)
}
