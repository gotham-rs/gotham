use router::request::path::PathExtractor;
use router::request::query_string::QueryStringExtractor;
use router::route::matcher::RouteMatcher;
use router::route::dispatch::PipelineHandleChain;
use router::builder::SingleRouteBuilder;
use router::builder::single::DefineSingleRoute;

pub trait ReplacePathExtractor<T>
where
    T: PathExtractor,
{
    type Output: DefineSingleRoute;

    fn replace_path_extractor(self) -> Self::Output;
}

impl<'a, M, C, P, PE, QSE, NPE> ReplacePathExtractor<NPE>
    for SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
    NPE: PathExtractor + Send + Sync + 'static,
{
    type Output = SingleRouteBuilder<'a, M, C, P, NPE, QSE>;

    fn replace_path_extractor(self) -> Self::Output {
        self.coerce()
    }
}

pub trait ReplaceQueryStringExtractor<T>
where
    T: QueryStringExtractor,
{
    type Output: DefineSingleRoute;

    fn replace_query_string_extractor(self) -> Self::Output;
}

impl<'a, M, C, P, PE, QSE, NQSE> ReplaceQueryStringExtractor<NQSE>
    for SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: Send + Sync + 'static,
    PE: PathExtractor + Send + Sync + 'static,
    QSE: QueryStringExtractor + Send + Sync + 'static,
    NQSE: QueryStringExtractor + Send + Sync + 'static,
{
    type Output = SingleRouteBuilder<'a, M, C, P, PE, NQSE>;

    fn replace_query_string_extractor(self) -> Self::Output {
        self.coerce()
    }
}
