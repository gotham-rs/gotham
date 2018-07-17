use std::io;

use futures_cpupool::{Builder, CpuPool};

use handler::HandlerFuture;
use middleware::workers::pool::WorkersPool;
use middleware::{Middleware, NewMiddleware};
use state::State;

/// A middleware which manages a pool of background threads, and allows work to be executed outside
/// of the event loop by passing `Job` types.
#[derive(Clone)]
pub struct WorkersMiddleware {
    pool: CpuPool,
}

impl WorkersMiddleware {
    /// Creates a new WorkersMiddleware with `n` threads.
    pub fn new(n: usize) -> WorkersMiddleware {
        let mut builder = Builder::new();
        builder.pool_size(n);
        builder.name_prefix("gotham-worker-");
        WorkersMiddleware::from_builder(&mut builder)
    }

    /// Creates a new WorkersMiddleware with thread pool parameters defined by the provided
    /// `Builder`.
    pub fn from_builder(builder: &mut Builder) -> WorkersMiddleware {
        let pool = builder.create();
        WorkersMiddleware { pool }
    }
}

impl NewMiddleware for WorkersMiddleware {
    type Instance = Self;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
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

    use helpers::http::response::create_response;
    use middleware::workers::job::*;
    use middleware::workers::pool::*;
    use pipeline::single::*;
    use pipeline::*;
    use router::Router;
    use router::builder::*;
    use test::TestServer;

    fn router() -> Router {
        let (chain, pipelines) =
            single_pipeline(new_pipeline().add(WorkersMiddleware::new(1)).build());

        build_router(chain, pipelines, |route| {
            route.get("/").to(handler);
        })
    }

    fn handler(state: State) -> Box<HandlerFuture> {
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
