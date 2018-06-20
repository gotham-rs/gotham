//! Defines helpers for applications that only require a single pipeline (i.e. only one set of
//! middleware for the application).

use borrow_bag::{Append, Handle};

use pipeline::set::{finalize_pipeline_set, new_pipeline_set, PipelineSet};
use pipeline::{NewMiddlewareChain, Pipeline};

/// A `PipelineSet` which contains only a single pipeline.
pub type SinglePipelineSet<C, B> = PipelineSet<<() as Append<Pipeline<C, B>>>::Output>;

/// A `Handle` for borrowing the only pipeline from a `SinglePipelineSet`.
pub type SinglePipelineHandle<C, B> =
    Handle<Pipeline<C, B>, <() as Append<Pipeline<C, B>>>::Navigator>;

/// A pipeline chain which contains only the single pipeline in a `SinglePipelineSet`.
pub type SinglePipelineChain<C, B> = (SinglePipelineHandle<C, B>, ());

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
pub fn single_pipeline<C, B>(
    c: Pipeline<C, B>,
) -> (SinglePipelineChain<C, B>, SinglePipelineSet<C, B>)
where
    C: NewMiddlewareChain<B>,
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
