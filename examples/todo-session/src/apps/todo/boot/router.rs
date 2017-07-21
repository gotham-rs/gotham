use std::sync::Arc;

use hyper::{Method, StatusCode};
use futures::{future, Future};

use gotham::dispatch::{new_pipeline_set, PipelineSet, PipelineHandleChain, DispatcherImpl};
use gotham::http::request_path::NoopRequestPathExtractor;
use gotham::http::query_string::NoopQueryStringExtractor;
use gotham::handler::NewHandler;
use gotham::middleware::pipeline::new_pipeline;
use gotham::middleware::session::NewSessionMiddleware;
use gotham::router::Router;
use gotham::router::route::{Extractors, Route, RouteImpl, Delegation};
use gotham::router::request_matcher::MethodOnlyRequestMatcher;
use gotham::router::response_extender::{ResponseExtenderBuilder, NoopExtender};
use gotham::router::tree::TreeBuilder;

use apps::todo::Session;
use apps::todo::controllers::todo;

fn static_route<NH, P, C>(methods: Vec<Method>,
                          new_handler: NH,
                          active_pipelines: C,
                          pipeline_set: PipelineSet<P>)
                          -> Box<Route + Send + Sync>
    where NH: NewHandler + 'static,
          C: PipelineHandleChain<P> + Send + Sync + 'static,
          P: Send + Sync + 'static
{
    let matcher = MethodOnlyRequestMatcher::new(methods);
    let dispatcher = DispatcherImpl::new(new_handler, active_pipelines, pipeline_set);
    let extractors: Extractors<NoopRequestPathExtractor, NoopQueryStringExtractor> =
        Extractors::new();
    let route = RouteImpl::new(matcher,
                               Box::new(dispatcher),
                               extractors,
                               Delegation::Internal);
    Box::new(route)
}

pub fn router() -> Router {
    let mut tree_builder = TreeBuilder::new();

    let editable_pipeline_set = new_pipeline_set();
    let (editable_pipeline_set, global) = editable_pipeline_set
        .add(new_pipeline()
                 .add(NewSessionMiddleware::<_, Session>::default())
                 .build());

    let pipeline_set = Arc::new(editable_pipeline_set);

    tree_builder.add_route(static_route(vec![Method::Get],
                                        || Ok(todo::index),
                                        (global, ()),
                                        pipeline_set.clone()));


    tree_builder.add_route(static_route(vec![Method::Post],
                                        || Ok(todo::add),
                                        (global, ()),
                                        pipeline_set.clone()));

    let tree = tree_builder.finalize();

    let mut response_extender_builder = ResponseExtenderBuilder::new();
    let extender_200 = NoopExtender::new();
    response_extender_builder.add(StatusCode::Ok, Box::new(extender_200));
    let extender_500 = NoopExtender::new();
    response_extender_builder.add(StatusCode::InternalServerError, Box::new(extender_500));
    let response_extender = response_extender_builder.finalize();

    Router::new(tree, response_extender)
}
