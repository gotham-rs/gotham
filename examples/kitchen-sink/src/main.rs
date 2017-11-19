#![deny(warnings)]
extern crate futures;
extern crate hyper;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate chrono;
#[macro_use]
extern crate log;
extern crate fern;
extern crate mime;
extern crate tokio_timer;

mod middleware;

use std::panic::RefUnwindSafe;

use tokio_timer::*;
use std::time::Duration;
use futures::{future, Future, Stream};
use hyper::{Body, Response, Method, StatusCode};
use log::LogLevelFilter;
use chrono::prelude::*;

use gotham::router::request::path::NoopPathExtractor;
use gotham::router::request::query_string::NoopQueryStringExtractor;
use gotham::router::response::finalizer::ResponseFinalizerBuilder;
use gotham::router::response::extender::NoopResponseExtender;
use gotham::router::Router;
use gotham::router::route::{Route, RouteImpl, Extractors, Delegation};
use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, PipelineSet,
                                      DispatcherImpl, PipelineHandleChain};
use gotham::router::route::matcher::MethodOnlyRouteMatcher;
use gotham::router::tree::TreeBuilder;
use gotham::router::tree::node::{NodeBuilder, SegmentType};
use gotham::handler::{NewHandler, HandlerFuture, IntoHandlerError};
use gotham::middleware::pipeline::new_pipeline;
use gotham::state::{State, FromState};
use gotham::http::response::create_response;

use self::middleware::{KitchenSinkData, KitchenSinkMiddleware};

struct Echo;

#[derive(StateData, PathExtractor, StaticResponseExtender)]
struct SharedRequestPath {
    name: String,

    // Ideally PathExtractors that are implemented by applications won't have any
    // Option fields.
    //
    // Instead have a fully specified Struct to represent every route with different segments
    // or meanings to ensure type safety.
    from: Option<String>,
}

#[derive(StateData, QueryStringExtractor, StaticResponseExtender)]
struct SharedQueryString {
    i: u8,
    q: Option<Vec<String>>,
}

static INDEX: &'static str = "Try POST /echo";
static ASYNC: &'static str = "Got async response";

fn static_route<NH, P, C>(
    methods: Vec<Method>,
    new_handler: NH,
    active_pipelines: C,
    pipeline_set: PipelineSet<P>,
) -> Box<Route + Send + Sync>
where
    NH: NewHandler + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: Send + Sync + RefUnwindSafe + 'static,
{
    let matcher = MethodOnlyRouteMatcher::new(methods);
    let dispatcher = DispatcherImpl::new(new_handler, active_pipelines, pipeline_set);
    let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
    let route = RouteImpl::new(
        matcher,
        Box::new(dispatcher),
        extractors,
        Delegation::Internal,
    );
    Box::new(route)
}

fn dynamic_route<NH, P, C>(
    methods: Vec<Method>,
    new_handler: NH,
    active_pipelines: C,
    pipeline_set: PipelineSet<P>,
) -> Box<Route + Send + Sync>
where
    NH: NewHandler + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: Send + Sync + RefUnwindSafe + 'static,
{
    let matcher = MethodOnlyRouteMatcher::new(methods);
    let dispatcher = DispatcherImpl::new(new_handler, active_pipelines, pipeline_set);
    let extractors: Extractors<SharedRequestPath, SharedQueryString> = Extractors::new();
    let route = RouteImpl::new(
        matcher,
        Box::new(dispatcher),
        extractors,
        Delegation::Internal,
    );
    Box::new(route)
}

// Builds a tree that looks like:
//
// /                     --> (Get Route)
// | - echo              --> (Get + Post Routes)
// | - async             --> (Get Route)
// | - header_value      --> (Get Route)
// | - hello
//     | - :name         --> (Get Route)
//         | :from       --> (Get Route)
fn build_router() -> Router {
    let mut tree_builder = TreeBuilder::new();

    let editable_pipeline_set = new_pipeline_set();
    let (editable_pipeline_set, global) = editable_pipeline_set.add(
        new_pipeline()
            .add(KitchenSinkMiddleware { header_name: "X-Kitchen-Sink" })
            .build(),
    );

    let pipeline_set = finalize_pipeline_set(editable_pipeline_set);

    tree_builder.add_route(static_route(
        vec![Method::Get],
        || Ok(Echo::get),
        (global, ()),
        pipeline_set.clone(),
    ));

    let mut echo = NodeBuilder::new("echo", SegmentType::Static);
    echo.add_route(static_route(
        vec![Method::Get, Method::Head],
        || Ok(Echo::get),
        (global, ()),
        pipeline_set.clone(),
    ));
    echo.add_route(static_route(
        vec![Method::Post],
        || Ok(Echo::post),
        (global, ()),
        pipeline_set.clone(),
    ));
    tree_builder.add_child(echo);

    let mut async = NodeBuilder::new("async", SegmentType::Static);
    async.add_route(static_route(
        vec![Method::Get],
        || Ok(Echo::async),
        (global, ()),
        pipeline_set.clone(),
    ));
    tree_builder.add_child(async);

    let mut wait = NodeBuilder::new("wait", SegmentType::Static);
    wait.add_route(static_route(
        vec![Method::Get],
        || Ok(Echo::wait),
        (global, ()),
        pipeline_set.clone(),
    ));
    tree_builder.add_child(wait);

    let mut header_value = NodeBuilder::new("header_value", SegmentType::Static);
    header_value.add_route(static_route(
        vec![Method::Get],
        || Ok(Echo::header_value),
        (global, ()),
        pipeline_set.clone(),
    ));
    tree_builder.add_child(header_value);

    let mut hello = NodeBuilder::new("hello", SegmentType::Static);

    let mut name = NodeBuilder::new("name", SegmentType::Dynamic);
    name.add_route(dynamic_route(
        vec![Method::Get],
        || Ok(Echo::hello),
        (global, ()),
        pipeline_set.clone(),
    ));

    let mut from = NodeBuilder::new("from", SegmentType::Dynamic);
    from.add_route(dynamic_route(
        vec![Method::Get],
        || Ok(Echo::greeting),
        (global, ()),
        pipeline_set.clone(),
    ));

    name.add_child(from);
    hello.add_child(name);
    tree_builder.add_child(hello);

    let tree = tree_builder.finalize();

    let mut response_finalizer_builder = ResponseFinalizerBuilder::new();
    let extender_200 = NoopResponseExtender::new();
    response_finalizer_builder.add(StatusCode::Ok, Box::new(extender_200));
    let extender_500 = NoopResponseExtender::new();
    response_finalizer_builder.add(StatusCode::InternalServerError, Box::new(extender_500));
    let response_finalizer = response_finalizer_builder.finalize();

    Router::new(tree, response_finalizer)
}

impl Echo {
    fn get(state: State) -> (State, Response) {
        let res = create_response(
            &state,
            StatusCode::Ok,
            Some((String::from(INDEX).into_bytes(), mime::TEXT_PLAIN)),
        );
        (state, res)
    }

    fn post(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state).concat2().then(
            move |full_body| {
                match full_body {
                    Ok(valid_body) => {
                        let res = create_response(
                            &state,
                            StatusCode::Ok,
                            Some((valid_body.to_vec(), mime::TEXT_PLAIN)),
                        );
                        future::ok((state, res))
                    }
                    Err(e) => future::err((state, e.into_handler_error())),
                }
            },
        );

        Box::new(f)
    }

    fn async(state: State) -> Box<HandlerFuture> {
        let res = create_response(
            &state,
            StatusCode::Ok,
            Some((String::from(ASYNC).into_bytes(), mime::TEXT_PLAIN)),
        );
        Box::new(future::lazy(move || future::ok((state, res))))
    }

    pub fn wait(state: State) -> Box<HandlerFuture> {
        let timeout = Timer::default();
        let sleep = timeout.sleep(Duration::from_secs(2));

        let result = sleep.then(|res| match res {
            Ok(_) => {
                let res = create_response(
                    &state,
                    StatusCode::Ok,
                    Some((
                        String::from("delayed hello").into_bytes(),
                        mime::TEXT_PLAIN,
                    )),
                );
                future::ok((state, res))
            }
            Err(e) => {
                let err = e.into_handler_error();
                future::err((state, err))
            }
        });

        Box::new(result)
    }

    fn header_value(mut state: State) -> (State, Response) {
        state.borrow_mut::<KitchenSinkData>().header_value = "different value!".to_owned();

        let res = create_response(
            &state,
            StatusCode::Ok,
            Some((String::from(INDEX).into_bytes(), mime::TEXT_PLAIN)),
        );
        (state, res)
    }

    fn hello(mut state: State) -> (State, Response) {
        let hello = format!("Hello, {}\n", SharedRequestPath::take_from(&mut state).name);

        let res = create_response(
            &state,
            StatusCode::Ok,
            Some((hello.into_bytes(), mime::TEXT_PLAIN)),
        );
        (state, res)
    }

    fn greeting(state: State) -> (State, Response) {
        let g = {
            let srp = SharedRequestPath::borrow_from(&state);
            let name = srp.name.as_str();
            let from = match srp.from {
                Some(ref s) => &s,
                None => "",
            };

            if let Some(srq) = state.try_borrow::<SharedQueryString>() {
                format!(
                    "Greetings, {} from {}. [i: {}, q: {:?}]\n",
                    name,
                    from,
                    srq.i,
                    srq.q
                )
            } else {
                format!("Greetings, {} from {}.\n", name, from)
            }
        };

        let res = create_response(
            &state,
            StatusCode::Ok,
            Some((g.into_bytes(), mime::TEXT_PLAIN)),
        );
        (state, res)
    }
}

fn main() {
    fern::Dispatch::new()
        .level(LogLevelFilter::Error)
        .level_for("gotham", log::LogLevelFilter::Error)
        .level_for("gotham::state", log::LogLevelFilter::Error)
        .level_for("gotham::start", log::LogLevelFilter::Info)
        .level_for("kitchen_sink", log::LogLevelFilter::Error)
        .chain(std::io::stdout())
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}]{}",
                Utc::now().format("[%Y-%m-%d %H:%M:%S%.9f]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .apply()
        .unwrap();

    gotham::start("127.0.0.1:7878", build_router());
}
