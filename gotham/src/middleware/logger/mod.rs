//! Middleware for the Gotham framework to log on requests made to the server.
//!
//! This implementation is quite bare at the moment and will log out using the
//! [Common Log Format](https://en.wikipedia.org/wiki/Common_Log_Format) (CLF).
use chrono::prelude::*;
use futures::{future, Future};
use hyper::{header::ContentLength, HttpVersion, Method, Uri};
use log::Level;
use std::io;

use handler::HandlerFuture;
use middleware::{Middleware, NewMiddleware};
use state::{client_addr, FromState, State};

/// A struct that can act as a logging middleware for Gotham.
///
/// We implement `NewMiddleware` here for Gotham to allow us to work with the request
/// lifecycle correctly. This trait requires `Clone`, so that is also included.
#[derive(Copy, Clone)]
pub struct LoggingMiddleware {
    duration: bool,
    level: Level,
}

/// Main implementation for `LoggingMiddleware` to enable various configuration.
impl LoggingMiddleware {
    /// Creates a new `LoggingMiddleware` using the provided log level.
    pub fn new(level: Level) -> LoggingMiddleware {
        LoggingMiddleware {
            level,
            duration: false,
        }
    }

    /// Creates a new `LoggingMiddleware` using the provided log level, with duration
    /// attached to the end of log messages.
    pub fn with_duration(level: Level) -> LoggingMiddleware {
        LoggingMiddleware {
            level,
            duration: true,
        }
    }
}

/// Implementation of `NewMiddleware` is required for Gotham middleware.
///
/// This will simply dereference the internal state, rather than deriving `NewMiddleware`
/// which will clone the structure - should be cheaper for repeated calls.
impl NewMiddleware for LoggingMiddleware {
    type Instance = Self;

    /// Returns a new middleware to be used to serve a request.
    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(*self)
    }
}

/// Implementing `gotham::middleware::Middleware` allows us to hook into the request chain
/// in order to correctly log out after a request has executed.
impl Middleware for LoggingMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        // skip everything if logging is disabled
        if !log_enabled!(self.level) {
            return chain(state);
        }

        // extract the current time
        let start_time = Utc::now();

        // hook onto the end of the request to log the access
        let f = chain(state).and_then(move |(state, response)| {
            // format the start time to the CLF formats
            let datetime = start_time.format("%d/%b/%Y:%H:%M:%S %z");

            // grab the ip address from the state
            let ip = client_addr(&state).unwrap().ip();

            // calculate duration
            let duration = {
                // disabled, so skip
                if !self.duration {
                    "".to_owned()
                } else {
                    // calculate microsecond offset from start
                    let micros_offset = Utc::now()
                        .signed_duration_since(start_time)
                        .num_microseconds()
                        .unwrap();

                    // format into a more readable format
                    if micros_offset < 1000 {
                        format!(" - {}µs", micros_offset)
                    } else if micros_offset < 1000000 {
                        format!(" - {:.2}ms", (micros_offset as f32) / 1000.0)
                    } else {
                        format!(" - {:.2}s", (micros_offset as f32) / 1000000.0)
                    }
                }
            };

            {
                // borrows from the state
                let path = Uri::borrow_from(&state);
                let method = Method::borrow_from(&state);
                let version = HttpVersion::borrow_from(&state);

                // take references based on the response
                let status = response.status().as_u16();
                let length = response.headers().get::<ContentLength>().unwrap();

                // log out
                log!(
                    self.level,
                    "{} - - [{}] \"{} {} {}\" {} {} {}",
                    ip,
                    datetime,
                    method,
                    path,
                    version,
                    status,
                    length,
                    duration
                );
            }

            // continue the response chain
            future::ok((state, response))
        });

        // box it up
        Box::new(f)
    }
}
