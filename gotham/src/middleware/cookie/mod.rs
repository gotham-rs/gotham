//! Defines a cookie parsing middleware to be attach cookies on requests.
use std::pin::Pin;

use cookie::{Cookie, CookieJar};
use hyper::header::{HeaderMap, HeaderValue, COOKIE};

use super::{Middleware, NewMiddleware};
use crate::handler::HandlerFuture;
use crate::state::{FromState, State};

/// A struct that can act as a cookie parsing middleware for Gotham.
///
/// We implement `NewMiddleware` here for Gotham to allow us to work with the request
/// lifecycle correctly. This trait requires `Clone`, so that is also included. Cookies
/// become availabe on the request state as the `CookieJar` type.
#[derive(Copy, Clone)]
pub struct CookieParser;

/// Public API for external re-use.
impl CookieParser {
    /// Parses a `CookieJar` from a `State`.
    pub fn from_state(state: &State) -> CookieJar {
        HeaderMap::borrow_from(&state)
            .get_all(COOKIE)
            .iter()
            .flat_map(HeaderValue::to_str)
            .flat_map(|cs| cs.split("; "))
            .flat_map(|cs| Cookie::parse(cs.to_owned()))
            .fold(CookieJar::new(), |mut jar, cookie| {
                jar.add_original(cookie);
                jar
            })
    }
}

/// `Middleware` trait implementation.
impl Middleware for CookieParser {
    /// Attaches a set of parsed cookies to the request state.
    fn call<Chain>(self, mut state: State, chain: Chain) -> Pin<Box<HandlerFuture>>
    where
        Chain: FnOnce(State) -> Pin<Box<HandlerFuture>>,
    {
        let cookies = { CookieParser::from_state(&state) };
        state.put(cookies);
        chain(state)
    }
}

/// `NewMiddleware` trait implementation.
impl NewMiddleware for CookieParser {
    type Instance = Self;

    /// Clones the current middleware to a new instance.
    fn new_middleware(&self) -> anyhow::Result<Self::Instance> {
        Ok(*self)
    }
}
