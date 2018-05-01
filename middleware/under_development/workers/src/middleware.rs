use futures_cpupool::{Builder, CpuPool};
use gotham::handler::HandlerFuture;
use gotham::middleware::Middleware;
use gotham::state::State;

use pool::WorkersPool;

/// A middleware which manages a pool of background threads, and allows work to be executed outside
/// of the event loop by passing `Job` types.
#[derive(Clone, NewMiddleware)]
pub struct WorkersMiddleware {
    pool: CpuPool,
}

impl WorkersMiddleware {
    /// Creates a new WorkersMiddleware with `n` threads.
    pub fn new(n: usize) -> WorkersMiddleware {
        let mut builder = Builder::new();
        builder.pool_size(n);
        WorkersMiddleware::from_builder(&mut builder)
    }

    /// Creates a new WorkersMiddleware with thread pool parameters defined by the provided
    /// `Builder`.
    pub fn from_builder(builder: &mut Builder) -> WorkersMiddleware {
        let pool = builder.create();
        WorkersMiddleware { pool }
    }
}

impl Middleware for WorkersMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture> + 'static,
        Self: Sized,
    {
        state.put(WorkersPool { pool: self.pool });
        chain(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::Future;
    use hyper::StatusCode;
    use mime;
    use gotham::http::response::create_response;
    use gotham::router::Router;
    use gotham::router::builder::*;
    use gotham::pipeline::single::*;
    use gotham::pipeline::*;
    use gotham::test::TestServer;

    use job::*;
    use pool::*;

    fn router() -> Router {
        let (chain, pipelines) =
            single_pipeline(new_pipeline().add(WorkersMiddleware::new(1)).build());

        build_router(chain, pipelines, |route| {
            route.get("/").to(handler);
        })
    }

    fn handler(mut state: State) -> Box<HandlerFuture> {
        let f = run_with_worker(state, |_state: &mut State| {
            let x = 41;
            move || Ok(x + 1)
        }).then(|r: Result<(State, usize), (State, ())>| {
            let (state, t) = r.unwrap_or_else(|_| panic!("not ok"));
            let response = create_response(
                &state,
                StatusCode::Ok,
                Some((format!("{}", t).into_bytes(), mime::TEXT_PLAIN)),
            );
            Ok((state, response))
        });

        Box::new(f)
    }

    #[test]
    fn middleware_tests() {
        let test_server = TestServer::new(router()).unwrap();
        let client = test_server.client();
        let response = client.get("https://example.com/").perform().unwrap();
        assert_eq!(response.status(), StatusCode::Ok);
        let body = response.read_utf8_body().unwrap();
        assert_eq!(&body, "42");
    }
}
