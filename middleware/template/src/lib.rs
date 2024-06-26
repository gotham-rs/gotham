//! Documentation for your crate, explaining what this Middleware does

#![warn(missing_docs, unreachable_pub)]
#![forbid(elided_lifetimes_in_paths, unsafe_code)]
#![doc(test(no_crate_inject, attr(deny(warnings))))]

#[macro_use]
extern crate log;

use std::pin::Pin;

use futures_util::future::{self, FutureExt, TryFutureExt};

use gotham::anyhow;
use gotham::handler::HandlerFuture;
use gotham::middleware::{Middleware, NewMiddleware};
use gotham::state::{request_id, State};

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

    fn new_middleware(&self) -> anyhow::Result<Self::Instance> {
        Ok(MyMiddleware {})
    }
}

impl Middleware for MyMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Pin<Box<HandlerFuture>>
    where
        Chain: FnOnce(State) -> Pin<Box<HandlerFuture>>,
    {
        debug!("[{}] pre chain", request_id(&state));
        // Do things prior to passing the request on to other middleware and the eventual Handler
        // ..
        // For example store something in State
        // state.put(MyData { my_value: "abcdefg".to_owned() });

        chain(state)
            .and_then(move |(state, response)| {
                {
                    debug!("[{}] post chain", request_id(&state));
                    // Do things once a response has come back
                    // ..
                    // For example get our data back from State
                    // let data = state.borrow::<MyData>().unwrap();
                }
                future::ok((state, response))
            })
            .boxed()
    }
}
