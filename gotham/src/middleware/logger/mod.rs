//! Middlewares for the Gotham framework to log on requests made to the server.
//!
//! This module contains several logging implementations, with varying degrees
//! of complexity. The default `RequestLogger` will log out using the standard
//! [Common Log Format](https://en.wikipedia.org/wiki/Common_Log_Format) (CLF).
//!
//! There is also a `SimpleLogger` which emits only basic request logs.
use futures::{future, Future};
use hyper::{header::CONTENT_LENGTH, Method, Uri, Version};
use log::Level;
use std::io;

use handler::HandlerFuture;
use helpers::timing::Timer;
use middleware::{Middleware, NewMiddleware};
use state::request_id::request_id;
use state::{client_addr, FromState, State};

/// A struct that can act as a logging middleware for Gotham.
///
/// We implement `NewMiddleware` here for Gotham to allow us to work with the request
/// lifecycle correctly. This trait requires `Clone`, so that is also included.
#[derive(Copy, Clone)]
pub struct RequestLogger {
    level: Level,
}

impl RequestLogger {
    /// Constructs a new `RequestLogger` instance.
    pub fn new(level: Level) -> Self {
        RequestLogger { level }
    }
}

/// Implementation of `NewMiddleware` is required for Gotham middleware.
///
/// This will simply dereference the internal state, rather than deriving `NewMiddleware`
/// which will clone the structure - should be cheaper for repeated calls.
impl NewMiddleware for RequestLogger {
    type Instance = Self;

    /// Returns a new middleware to be used to serve a request.
    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(*self)
    }
}

/// Implementing `gotham::middleware::Middleware` allows us to hook into the request chain
/// in order to correctly log out after a request has executed.
impl Middleware for RequestLogger {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        // skip everything if logging is disabled
        if !log_enabled!(self.level) {
            return chain(state);
        }

        // extract the current time
        let timer = Timer::new();

        // hook onto the end of the request to log the access
        let f = chain(state).and_then(move |(state, response)| {
            // format the start time to the CLF formats
            let datetime = timer.start_time().format("%d/%b/%Y:%H:%M:%S %z");

            // grab the ip address from the state
            let ip = client_addr(&state).unwrap().ip();

            {
                // borrows from the state
                let path = Uri::borrow_from(&state);
                let method = Method::borrow_from(&state);
                let version = Version::borrow_from(&state);

                // take references based on the response
                let status = response.status().as_u16();
                let length = response
                    .headers()
                    .get(CONTENT_LENGTH)
                    .map(|len| len.to_str().unwrap())
                    .unwrap_or("0");

                // log out
                log!(
                    self.level,
                    "{} - - [{}] \"{} {} {:?}\" {} {} - {}",
                    ip,
                    datetime,
                    method,
                    path,
                    version,
                    status,
                    length,
                    timer.elapsed()
                );
            }

            // continue the response chain
            future::ok((state, response))
        });

        // box it up
        Box::new(f)
    }
}

/// A struct that can act as a simple logging middleware for Gotham.
///
/// We implement `NewMiddleware` here for Gotham to allow us to work with the request
/// lifecycle correctly. This trait requires `Clone`, so that is also included.
#[derive(Copy, Clone)]
pub struct SimpleLogger {
    level: Level,
}

impl SimpleLogger {
    /// Constructs a new `SimpleLogger` instance.
    pub fn new(level: Level) -> Self {
        SimpleLogger { level }
    }
}

/// Implementation of `NewMiddleware` is required for Gotham middleware.
///
/// This will simply dereference the internal state, rather than deriving `NewMiddleware`
/// which will clone the structure - should be cheaper for repeated calls.
impl NewMiddleware for SimpleLogger {
    type Instance = Self;

    /// Returns a new middleware to be used to serve a request.
    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(*self)
    }
}

/// Implementing `gotham::middleware::Middleware` allows us to hook into the request chain
/// in order to correctly log out after a request has executed.
impl Middleware for SimpleLogger {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        // skip everything if logging is disabled
        if !log_enabled!(self.level) {
            return chain(state);
        }

        // extract the current time
        let timer = Timer::new();

        // execute the request and chain the logging call
        let f = chain(state).and_then(move |(state, response)| {
            log!(
                self.level,
                "[RESPONSE][{}][{:?}][{}][{}]",
                request_id(&state),
                response.version(),
                response.status(),
                timer.elapsed()
            );

            future::ok((state, response))
        });

        Box::new(f)
    }
}
