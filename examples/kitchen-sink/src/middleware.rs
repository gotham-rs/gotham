use gotham::handler::HandlerFuture;
use gotham::state::{State, StateData};
use gotham::middleware::Middleware;
use hyper::server::Request;
use futures::{future, Future};

pub struct KitchenSinkData {
    pub header_value: String,
}

impl StateData for KitchenSinkData {}

pub struct KitchenSinkMiddleware {
    pub header_name: &'static str,
}

impl Middleware for KitchenSinkMiddleware {
    fn call<Chain>(&self, state: &mut State, request: Request, chain: Chain) -> Box<HandlerFuture>
        where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
    {
        state.put(KitchenSinkData { header_value: "default value".to_owned() });

        let result = chain(state, request);
        let header_name = self.header_name;
        let data = state.take::<KitchenSinkData>().unwrap();

        result.and_then(move |mut response| {
                            response.headers_mut().set_raw(header_name, data.header_value);
                            future::ok(response)
                        })
            .boxed()
    }
}
