use std::panic::RefUnwindSafe;

use extractor::{PathExtractor, QueryStringExtractor};
use router::route::matcher::RouteMatcher;
use pipeline::chain::PipelineHandleChain;
use router::builder::SingleRouteBuilder;
use router::builder::single::DefineSingleRoute;

/// Describes the operation of replacing a `PathExtractor` on a route. This trait exists to remove
/// type clutter from the documentation of `SingleRouteBuilder::with_path_extractor`.
pub trait ReplacePathExtractor<T>
where
    T: PathExtractor,
{
    /// The type returned when replacing the `PathExtractor` with the target type.
    type Output: DefineSingleRoute;

    #[doc(hidden)]
    /// Replaces the `PathExtractor` in `self` with the parameterized type `T`. This is a type
    /// level operation so takes no value.
    fn replace_path_extractor(self) -> Self::Output;
}

impl<'a, M, C, P, PE, QSE, NPE> ReplacePathExtractor<NPE>
    for SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
    NPE: PathExtractor + Send + Sync + 'static,
{
    type Output = SingleRouteBuilder<'a, M, C, P, NPE, QSE>;

    fn replace_path_extractor(self) -> Self::Output {
        self.coerce()
    }
}

/// Describes the operation of replacing a `QueryStringExtractor` on a route. This trait exists to
/// remove type clutter from the documentation of `SingleRouteBuilder::with_query_string_extractor`.
pub trait ReplaceQueryStringExtractor<T>
where
    T: QueryStringExtractor,
{
    /// The type returned when replacing the `QueryStringExtractor` with the target type.
    type Output: DefineSingleRoute;

    #[doc(hidden)]
    /// Replaces the `QueryStringExtractor` in `self` with the parameterized type `T`. This is a
    /// type level operation so takes no value.
    fn replace_query_string_extractor(self) -> Self::Output;
}

impl<'a, M, C, P, PE, QSE, NQSE> ReplaceQueryStringExtractor<NQSE>
    for SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
    NQSE: QueryStringExtractor + Send + Sync + 'static,
{
    type Output = SingleRouteBuilder<'a, M, C, P, PE, NQSE>;

    fn replace_query_string_extractor(self) -> Self::Output {
        self.coerce()
    }
}
