//! Defines helpers for applications that only require a single pipeline (i.e. only one set of
//! middleware for the application).

use borrow_bag::{Append, Handle};

use router::route::dispatch::{finalize_pipeline_set, new_pipeline_set, PipelineSet};
use pipeline::{NewMiddlewareChain, Pipeline};

/// A `PipelineSet` which contains only a single pipeline.
pub type SinglePipelineSet<C> = PipelineSet<<() as Append<Pipeline<C>>>::Output>;

/// A `Handle` for borrowing the only pipeline from a `SinglePipelineSet`.
pub type SinglePipelineHandle<C> = Handle<Pipeline<C>, <() as Append<Pipeline<C>>>::Navigator>;

/// A pipeline chain which contains only the single pipeline in a `SinglePipelineSet`.
pub type SinglePipelineChain<C> = (SinglePipelineHandle<C>, ());

/// Returns a set of pipelines containing a single pipeline, and a chain containing only that
/// single pipeline.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # use gotham::pipeline::single::single_pipeline;
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::router::builder::build_router;
/// #
/// # fn main() {
/// let (pipelines, chain) = single_pipeline(
///     new_pipeline().build()
/// );
///
/// build_router(chain, pipelines, |route| {
///     // Implementation elided
/// #   drop(route);
/// });
/// # }
/// ```
pub fn single_pipeline<C>(c: Pipeline<C>) -> (SinglePipelineSet<C>, SinglePipelineChain<C>)
where
    C: NewMiddlewareChain,
{
    let pipelines = new_pipeline_set();
    let (pipelines, single) = pipelines.add(c);
    let pipelines = finalize_pipeline_set(pipelines);

    let chain = (single, ());

    (pipelines, chain)
}

#[cfg(test)]
mod tests {
    use super::*;

    use router::builder::*;
    use pipeline::new_pipeline;

    #[test]
    fn test_pipeline_construction() {
        let (pipelines, chain) = single_pipeline(new_pipeline().build());

        build_router(chain, pipelines, |_route| {});
    }
}
