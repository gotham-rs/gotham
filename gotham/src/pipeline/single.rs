//! Defines helpers for applications that only require a single pipeline (i.e. only one set of
//! middleware for the application).

use borrow_bag::{Append, Handle};

use pipeline::set::{finalize_pipeline_set, new_pipeline_set, PipelineSet};
use pipeline::{NewMiddlewareChain, Pipeline};

/// A `PipelineSet` which contains only a single pipeline.
pub type SinglePipelineSet<C> = PipelineSet<<() as Append<Pipeline<C>>>::Output>;

/// A `Handle` for borrowing the only pipeline from a `SinglePipelineSet`.
pub type SinglePipelineHandle<C> = Handle<Pipeline<C>, <() as Append<Pipeline<C>>>::Navigator>;

/// A pipeline chain which contains only the single pipeline in a `SinglePipelineSet`.
pub type SinglePipelineChain<C> = (SinglePipelineHandle<C>, ());

/// Creates a single pipeline for use in applications with straightforward use cases for
/// middleware.
///
/// Returns instances of the required PipelineHandleChain and PipelineSet types ready for use with
/// `build_router`.
///
/// # Examples
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # use gotham::pipeline::single::single_pipeline;
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::router::builder::build_router;
/// # use gotham::middleware::session::NewSessionMiddleware;
/// #
/// # #[derive(Serialize, Deserialize, Default)]
/// # struct Session;
/// #
/// # fn main() {
/// let (chain, pipelines) = single_pipeline(
///     new_pipeline()
///         .add(NewSessionMiddleware::default().with_session_type::<Session>())
///         .build()
/// );
///
/// build_router(chain, pipelines, |route| {
///     // Implementation elided
/// #   drop(route);
/// });
/// # }
/// ```
pub fn single_pipeline<C>(c: Pipeline<C>) -> (SinglePipelineChain<C>, SinglePipelineSet<C>)
where
    C: NewMiddlewareChain,
{
    let pipelines = new_pipeline_set();
    let (pipelines, single) = pipelines.add(c);
    let pipelines = finalize_pipeline_set(pipelines);

    let chain = (single, ());

    (chain, pipelines)
}

#[cfg(test)]
mod tests {
    use super::*;

    use pipeline::new_pipeline;
    use router::builder::*;

    #[test]
    fn test_pipeline_construction() {
        let (chain, pipelines) = single_pipeline(new_pipeline().build());

        build_router(chain, pipelines, |_route| {});
    }
}
