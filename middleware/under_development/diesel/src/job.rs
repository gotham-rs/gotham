use std::marker::PhantomData;

use gotham::state::{FromState, State};
use gotham_middleware_workers::{run_with_worker, Job, PreparedJob};

use diesel::Connection;
use futures::{Future, IntoFuture};

use state_data::Diesel;

/// Runs the given closure in a worker, after borrowing a connection from the pool. This requires
/// that the `State` has data populated by both the `DieselMiddleware` and `WorkersMiddleware`. If
/// one of these have not added their state data, this function will panic.
pub fn run_with_diesel<F, C, T, E, R>(
    state: State,
    f: F,
) -> Box<Future<Item = (State, T), Error = (State, E)> + Send>
where
    C: Connection + 'static,
    F: FnOnce(&C) -> R + Send + 'static,
    R: IntoFuture<Item = T, Error = E> + 'static,
    R::Future: Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    let job = DieselJob {
        f,
        phantom: PhantomData,
    };

    run_with_worker(state, job)
}

struct DieselJob<F, C, T, E, R>
where
    C: Connection + 'static,
    F: FnOnce(&C) -> R + Send + 'static,
    R: IntoFuture<Item = T, Error = E> + 'static,
    R::Future: Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    f: F,
    phantom: PhantomData<FnOnce(&C) -> R + Send>,
}

impl<F, C, T, E, R> Job for DieselJob<F, C, T, E, R>
where
    C: Connection + 'static,
    F: FnOnce(&C) -> R + Send + 'static,
    R: IntoFuture<Item = T, Error = E> + 'static,
    R::Future: Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    type Item = T;
    type Error = E;
    type Prepared = PreparedDieselJob<F, C, T, E, R>;

    fn prepare(self, state: &mut State) -> Self::Prepared {
        let diesel = Diesel::borrow_from(state).clone();

        PreparedDieselJob {
            diesel,
            f: self.f,
            _phantom: self.phantom,
        }
    }
}

struct PreparedDieselJob<F, C, T, E, R>
where
    C: Connection + 'static,
    F: FnOnce(&C) -> R + Send + 'static,
    R: IntoFuture<Item = T, Error = E> + 'static,
    R::Future: Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    diesel: Diesel<C>,
    f: F,
    _phantom: PhantomData<FnOnce(&C) -> R + Send>,
}

impl<F, C, T, E, R> PreparedJob for PreparedDieselJob<F, C, T, E, R>
where
    C: Connection + 'static,
    F: FnOnce(&C) -> R + Send + 'static,
    R: IntoFuture<Item = T, Error = E> + 'static,
    R::Future: Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    type Item = T;
    type Error = E;
    type Future = R::Future;
    type Output = R;

    fn run(self) -> Self::Output {
        let conn = self.diesel
            .conn()
            .expect("Did not obtain valid Diesel connection from R2D2 pool");

        (self.f)(&*conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use diesel::sqlite::SqliteConnection;
    use diesel::{self, RunQueryDsl};
    use gotham::handler::HandlerFuture;
    use gotham::helpers::http::response::create_response;
    use gotham::pipeline::new_pipeline;
    use gotham::pipeline::single::*;
    use gotham::router::Router;
    use gotham::router::builder::*;
    use gotham::test::TestServer;
    use gotham_middleware_workers::WorkersMiddleware;
    use hyper::StatusCode;
    use mime;

    use middleware::DieselMiddleware;

    static DATABASE_URL: &'static str = ":memory:";

    fn handler(state: State) -> Box<HandlerFuture> {
        let f = run_with_diesel(state, |conn: &SqliteConnection| {
            diesel::select(diesel::dsl::sql("1"))
                .load::<i64>(conn)
                .map(|v| v.into_iter().next().expect("no results"))
        }).then(|r| {
            let (state, n) = r.unwrap_or_else(|_| panic!("query failed"));
            let body = format!("result: {}", n);
            let response = create_response(
                &state,
                StatusCode::Ok,
                Some((body.into_bytes(), mime::TEXT_PLAIN)),
            );
            Ok((state, response))
        });

        Box::new(f)
    }

    // Since we can't construct `State` ourselves, we need to test this via an actual app.
    fn router() -> Router {
        let (chain, pipelines) = single_pipeline(
            new_pipeline()
                .add(DieselMiddleware::<SqliteConnection>::new(DATABASE_URL))
                .add(WorkersMiddleware::new(1))
                .build(),
        );

        build_router(chain, pipelines, |route| {
            route.get("/").to(handler);
        })
    }

    #[test]
    fn run_with_diesel_tests() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("https://example.com/")
            .perform()
            .unwrap();
        assert_eq!(response.status(), StatusCode::Ok);
        let body = response.read_utf8_body().unwrap();
        assert_eq!(&body, "result: 1");
    }
}
