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

use tokio_timer::*;
use std::time::Duration;
use futures::{future, Future, Stream};
use hyper::{Body, Response, StatusCode};
use log::LogLevelFilter;
use chrono::prelude::*;

use gotham::router::response::extender::NoopResponseExtender;
use gotham::router::Router;
use gotham::router::builder::*;
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
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

// Builds a tree that looks like:
//
// /                     --> (Get Route)
// | - echo              --> (Get + Post Routes)
// | - async             --> (Get Route)
// | - header_value      --> (Get Route)
// | - hello
//     | - :name         --> (Get Route)
//         | :from       --> (Get Route)
fn router() -> Router {
    let (pipelines, chain) = single_pipeline(
        new_pipeline()
            .add(KitchenSinkMiddleware { header_name: "X-Kitchen-Sink" })
            .build(),
    );

    build_router(chain, pipelines, |route| {
        route.get("/").to(Echo::get);
        route.get("/echo").to(Echo::get);
        route.post("/echo").to(Echo::post);
        route.get("/async").to(Echo::async);
        route.get("/wait").to(Echo::wait);
        route.get("/header_value").to(Echo::header_value);

        route
            .get("/hello/:name")
            .with_path_extractor::<SharedRequestPath>()
            .with_query_string_extractor::<SharedQueryString>()
            .to(Echo::hello);

        route
            .get("/hello/:name/:from")
            .with_path_extractor::<SharedRequestPath>()
            .with_query_string_extractor::<SharedQueryString>()
            .to(Echo::greeting);

        route.add_response_extender(StatusCode::Ok, NoopResponseExtender::new());
        route.add_response_extender(StatusCode::InternalServerError, NoopResponseExtender::new());
    })
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

    gotham::start("127.0.0.1:7878", router());
}
