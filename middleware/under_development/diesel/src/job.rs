use std::marker::PhantomData;

use gotham::state::{FromState, State};
use gotham_middleware_workers::{run_with_worker, Job, PreparedJob};

use futures::{Future, IntoFuture};
use diesel::Connection;

use state_data::Diesel;

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

/// Runs the given closure in a worker, after borrowing a connection from the pool. This requires
/// that the `State` has data populated by both the `DieselMiddleware` and `WorkersMiddleware`. If
/// one of these have not added their state data, this function will panic.
pub fn run_with_diesel<F, C, T, E, R>(
    state: State,
    f: F,
) -> Box<Future<Item = (State, T), Error = (State, E)>>
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
