//! Documentation for your crate, explaining what this Middleware does

#![warn(missing_docs, deprecated)]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

extern crate futures;
extern crate gotham;
#[macro_use]
extern crate log;

// Enable to use #[derive(StateData)] below
//#[macro_use]
//extern crate gotham_derive;

use std::io;

use futures::{future, Future};

use gotham::middleware::{Middleware, NewMiddleware};
use gotham::state::{request_id, State};
use gotham::handler::HandlerFuture;

// Example of struct that stores owned data in State
//
// n.b. There is no requirement to have a StateData struct associated with your Middleware
// instance but it is a common need hence we've shown one here to assist newcomers.
//
//#[derive(StateData)]
//pub struct MyData {
//  pub my_value: String,
//}

/// A Gotham compatible Middleware that... (your documentation here).
pub struct MyMiddleware {}

impl NewMiddleware for MyMiddleware {
    type Instance = MyMiddleware;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(MyMiddleware { ..*self })
    }
}

impl Middleware for MyMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        debug!("[{}] pre chain", request_id(&state));
        // Do things prior to passing the request on to other middleware and the eventual Handler
        // ..
        // For example store something in State
        // state.put(MyData { my_value: "abcdefg".to_owned() });

        let f = chain(state).and_then(move |(state, response)| {
            {
                debug!("[{}] post chain", request_id(&state));
                // Do things once a response has come back
                // ..
                // For example get our data back from State
                // let data = state.borrow::<MyData>().unwrap();
            }
            future::ok((state, response))
        });
        Box::new(f)
    }
}
