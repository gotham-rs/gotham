use std::sync::Arc;

use hyper::{Method, StatusCode};
use futures::{future, Future};

use gotham::handler::NewHandler;
use gotham::middleware::pipeline::new_pipeline;
use gotham::middleware::session::{NewSessionMiddleware, MemoryBackend};
use gotham::router::Router;
use gotham::router::route::{Extractors, Route, RouteImpl, Delegation};
use gotham::router::route::dispatch::{new_pipeline_set, PipelineSet, PipelineHandleChain,
                                      DispatcherImpl};
use gotham::router::route::matcher::MethodOnlyRouteMatcher;
use gotham::router::request::path::NoopPathExtractor;
use gotham::router::request::query_string::NoopQueryStringExtractor;
use gotham::router::response::finalizer::ResponseFinalizerBuilder;
use gotham::router::response::extender::NoopResponseExtender;
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
    let matcher = MethodOnlyRouteMatcher::new(methods);
    let dispatcher = DispatcherImpl::new(new_handler, active_pipelines, pipeline_set);
    let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
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
                 .add(NewSessionMiddleware::default()
                          .insecure()
                          .with_session_type::<Session>())
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

    let mut response_finalizer_builder = ResponseFinalizerBuilder::new();
    let extender_200 = NoopResponseExtender::new();
    response_finalizer_builder.add(StatusCode::Ok, Box::new(extender_200));
    let extender_500 = NoopResponseExtender::new();
    response_finalizer_builder.add(StatusCode::InternalServerError, Box::new(extender_500));
    let response_finalizer = response_finalizer_builder.finalize();

    Router::new(tree, response_finalizer)
}
