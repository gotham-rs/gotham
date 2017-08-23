#![deny(warnings)]

#[macro_use]
extern crate log;
extern crate futures;
extern crate hyper;
extern crate gotham;
extern crate diesel;

// Enable to use #[derive(StateData)] below
//#[macro_use]
//extern crate gotham_derive;

use std::io;

use futures::{Future, future};
use hyper::Request;

use gotham::middleware::{NewMiddleware, Middleware};
use gotham::state::{State, request_id};
use gotham::handler::HandlerFuture;

// Example of struct that stores owned data in State
//
// n.b. There is no requirement to have a StateData struct associated with your Middleware
// instance but it is a common need hence we've shown one here to assist newcomers.
//
//#[derive(StateData)]
//pub struct DieselData {
//  pub my_value: String,
//}

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
        // Do things prior to passing the request on to other middleware and the eventual Handler
        // ..
        // For example store something in State
        // state.put(DieselData { my_value: "abcdefg".to_owned() });

        chain(state, request)
            .and_then(move |(state, response)| {
                {
                    trace!("[{}] post chain", request_id(&state));
                    // Do things once a response has come back
                    // ..
                    // For example get our data back from State
                    // let data = state.borrow::<DieselData>().unwrap();
                }
                future::ok((state, response))
            })
            .boxed()
    }
}
