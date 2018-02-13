//! Defines the types for adding multiple pipelines into a `PipelineSet` and retaining a handle to
//! each pipeline for constructing a `PipelineHandleChain`.

use std::sync::Arc;
use borrow_bag::BorrowBag;

/// Represents the set of all `Pipeline` instances that are available for use with `Routes`.
pub type PipelineSet<P> = Arc<BorrowBag<P>>;

/// A set of `Pipeline` instances that may continue to grow
pub type EditablePipelineSet<P> = BorrowBag<P>;

/// Create an empty set of `Pipeline` instances.
///
/// See BorrowBag#add to insert new `Pipeline` instances.
pub fn new_pipeline_set() -> EditablePipelineSet<()> {
    BorrowBag::new()
}

/// Wraps the current set of `Pipeline` instances into a thread-safe reference counting pointer for
/// use with `DispatcherImpl` instances.
pub fn finalize_pipeline_set<P>(eps: EditablePipelineSet<P>) -> PipelineSet<P> {
    Arc::new(eps)
}
