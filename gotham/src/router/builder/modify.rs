use std::panic::RefUnwindSafe;

use extractor::{PathExtractor, QueryStringExtractor};
use hyper::Body;
use pipeline::chain::PipelineHandleChain;
use router::builder::single::DefineSingleRoute;
use router::builder::SingleRouteBuilder;
use router::route::matcher::{AndRouteMatcher, RouteMatcher};

/// Describes the operation of replacing a `PathExtractor` on a route. This trait exists to remove
/// type clutter from the documentation of `SingleRouteBuilder::with_path_extractor`.
pub trait ReplacePathExtractor<T>
where
    T: PathExtractor<Body>,
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
    PE: PathExtractor<Body> + Send + Sync + 'static,
    QSE: QueryStringExtractor<Body> + Send + Sync + 'static,
    NPE: PathExtractor<Body> + Send + Sync + 'static,
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
    T: QueryStringExtractor<Body>,
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
    PE: PathExtractor<Body> + Send + Sync + 'static,
    QSE: QueryStringExtractor<Body> + Send + Sync + 'static,
    NQSE: QueryStringExtractor<Body> + Send + Sync + 'static,
{
    type Output = SingleRouteBuilder<'a, M, C, P, PE, NQSE>;

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

impl<'a, M, NRM, C, P, PE, QSE> ExtendRouteMatcher<NRM> for SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    NRM: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
    PE: PathExtractor<Body> + Send + Sync + 'static,
    QSE: QueryStringExtractor<Body> + Send + Sync + 'static,
{
    /// The type returned when extending the existing `RouteMatcher` with the target type.
    type Output = SingleRouteBuilder<'a, AndRouteMatcher<M, NRM>, C, P, PE, QSE>;

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
