use gotham;
use gotham::handler::HandlerFuture;
use gotham::state::State;
use gotham::middleware::Middleware;
use futures::{future, Future};

use gotham::state::request_id;

#[derive(StateData)]
pub struct KitchenSinkData {
    pub header_value: String,
}

#[derive(Clone, NewMiddleware)]
pub struct KitchenSinkMiddleware {
    pub header_name: &'static str,
}

impl Middleware for KitchenSinkMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(KitchenSinkData { header_value: "default value".to_owned() });

        let result = chain(state);
        let header_name = self.header_name;

        let f = result.and_then(move |(state, mut response)| {
            {
                let data = state.borrow::<KitchenSinkData>();
                let headers = response.headers_mut();
                headers.set_raw(header_name, data.header_value.to_owned());
                headers.set_raw("X-Request-ID", request_id(&state));
            }

            future::ok((state, response))
        });

        Box::new(f)
    }
}
