#![deny(warnings)]

#[macro_use]
extern crate log;
extern crate futures;
extern crate hyper;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate diesel;

use std::io;

use futures::{Future, future};
use hyper::Request;

use gotham::middleware::{NewMiddleware, Middleware};
use gotham::state::{State, request_id};
use gotham::handler::HandlerFuture;

/// Provides access to a Diesel controller within an R2D2 pool via Gotham State
/// for Middleware and Handlers.
#[derive(StateData)]
pub struct DieselData {}

pub struct DieselMiddleware {}

impl NewMiddleware for DieselMiddleware {
    type Instance = DieselMiddleware;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(DieselMiddleware { ..*self })
    }
}

impl Middleware for DieselMiddleware {
    fn call<Chain>(self, state: State, request: Request, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State, Request) -> Box<HandlerFuture>,
    {
        trace!("[{}] pre chain", request_id(&state));

        chain(state, request)
            .and_then(move |(state, response)| {
                {
                    trace!("[{}] post chain", request_id(&state));
                }
                future::ok((state, response))
            })
            .boxed()
    }
}
