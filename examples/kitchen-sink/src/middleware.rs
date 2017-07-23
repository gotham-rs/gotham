use std::io;

use gotham;
use gotham::handler::HandlerFuture;
use gotham::state::State;
use gotham::middleware::{Middleware, NewMiddleware};
use hyper::server::Request;
use futures::{future, Future};

use gotham::state::request_id;

#[derive(StateData)]
pub struct KitchenSinkData {
    pub header_value: String,
}

pub struct KitchenSinkMiddleware {
    pub header_name: &'static str,
}

impl NewMiddleware for KitchenSinkMiddleware {
    type Instance = KitchenSinkMiddleware;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(KitchenSinkMiddleware { ..*self })
    }
}

impl Middleware for KitchenSinkMiddleware {
    fn call<Chain>(self, mut state: State, request: Request, chain: Chain) -> Box<HandlerFuture>
        where Chain: FnOnce(State, Request) -> Box<HandlerFuture>
    {
        state.put(KitchenSinkData { header_value: "default value".to_owned() });

        let result = chain(state, request);
        let header_name = self.header_name;

        result
            .and_then(move |(state, mut response)| {
                {
                    let data = state.borrow::<KitchenSinkData>().unwrap();
                    let headers = response.headers_mut();
                    headers.set_raw(header_name, data.header_value.to_owned());
                    headers.set_raw("X-Request-ID", request_id(&state));
                }

                future::ok((state, response))
            })
            .boxed()
    }
}
