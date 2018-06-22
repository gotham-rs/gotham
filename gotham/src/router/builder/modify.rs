use std::panic::RefUnwindSafe;

use extractor::{PathExtractor, QueryStringExtractor};
use pipeline::chain::PipelineHandleChain;
use router::builder::single::DefineSingleRoute;
use router::builder::SingleRouteBuilder;
use router::route::matcher::{AndRouteMatcher, RouteMatcher};

/// Describes the operation of replacing a `PathExtractor` on a route. This trait exists to remove
/// type clutter from the documentation of `SingleRouteBuilder::with_path_extractor`.
pub trait ReplacePathExtractor<T, B>
where
    T: PathExtractor<B>,
{
    /// The type returned when replacing the `PathExtractor` with the target type.
    type Output: DefineSingleRoute;

    #[doc(hidden)]
    /// Replaces the `PathExtractor` in `self` with the parameterized type `T`. This is a type
    /// level operation so takes no value.
    fn replace_path_extractor(self) -> Self::Output;
}

impl<'a, M, C, P, PE, QSE, NPE, B> ReplacePathExtractor<NPE, B>
    for SingleRouteBuilder<'a, M, C, P, PE, QSE, B>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P, B> + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor<B> + Send + Sync + 'static,
    QSE: QueryStringExtractor<B> + Send + Sync + 'static,
    NPE: PathExtractor<B> + Send + Sync + 'static,
{
    type Output = SingleRouteBuilder<'a, M, C, P, NPE, QSE, B>;

    fn replace_path_extractor(self) -> Self::Output {
        self.coerce()
    }
}

/// Describes the operation of replacing a `QueryStringExtractor` on a route. This trait exists to
/// remove type clutter from the documentation of `SingleRouteBuilder::with_query_string_extractor`.
pub trait ReplaceQueryStringExtractor<T, B>
where
    T: QueryStringExtractor<B>,
{
    /// The type returned when replacing the `QueryStringExtractor` with the target type.
    type Output: DefineSingleRoute;

    #[doc(hidden)]
    /// Replaces the `QueryStringExtractor` in `self` with the parameterized type `T`. This is a
    /// type level operation so takes no value.
    fn replace_query_string_extractor(self) -> Self::Output;
}

impl<'a, M, C, P, PE, QSE, NQSE, B> ReplaceQueryStringExtractor<NQSE, B>
    for SingleRouteBuilder<'a, M, C, P, PE, QSE, B>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P, B> + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor<B> + Send + Sync + 'static,
    QSE: QueryStringExtractor<B> + Send + Sync + 'static,
    NQSE: QueryStringExtractor<B> + Send + Sync + 'static,
{
    type Output = SingleRouteBuilder<'a, M, C, P, PE, NQSE, B>;

    fn replace_query_string_extractor(self) -> Self::Output {
        self.coerce()
    }
}

/// Describes the operation of extending a `RouteMatcher` on a route. This trait exists to remove
/// type clutter from the documentation of `SingleRouteBuilder::add_route_matcher`.
pub trait ExtendRouteMatcher<NRM>
where
    NRM: RouteMatcher + Send + Sync + 'static,
{
    /// The type returned when extending the existing `RouteMatcher` with the target type.
    type Output: DefineSingleRoute;

    #[doc(hidden)]
    /// Combines the existing `RouteMatcher` using an `AndRouteMatcher` with the `RouteMatcher`
    /// defined as NM
    fn extend_route_matcher(self, matcher: NRM) -> Self::Output;
}

impl<'a, M, NRM, C, P, PE, QSE, B> ExtendRouteMatcher<NRM>
    for SingleRouteBuilder<'a, M, C, P, PE, QSE, B>
where
    M: RouteMatcher + Send + Sync + 'static,
    NRM: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P, B> + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor<B> + Send + Sync + 'static,
    QSE: QueryStringExtractor<B> + Send + Sync + 'static,
{
    /// The type returned when extending the existing `RouteMatcher` with the target type.
    type Output = SingleRouteBuilder<'a, AndRouteMatcher<M, NRM>, C, P, PE, QSE, B>;

    fn extend_route_matcher(self, matcher: NRM) -> Self::Output {
        SingleRouteBuilder {
            matcher: AndRouteMatcher::<M, NRM>::new(self.matcher, matcher),
            phantom: self.phantom,
            node_builder: self.node_builder,
            pipeline_chain: self.pipeline_chain,
            pipelines: self.pipelines,
        }
    }
}
