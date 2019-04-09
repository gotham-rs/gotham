use diesel::Connection;
use futures::future::{self, Future};
use log::{error, trace};
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process;

use gotham::handler::HandlerFuture;
use gotham::middleware::{Middleware, NewMiddleware};
use gotham::state::{request_id, State};

mod repo;

use repo::Repo;

pub struct DieselMiddleware<T>
where
    T: Connection + 'static,
{
    repo: AssertUnwindSafe<Repo<T>>,
}

impl<T> DieselMiddleware<T>
where
    T: Connection,
{
    pub fn new(repo: Repo<T>) -> Self {
        DieselMiddleware {
            repo: AssertUnwindSafe(repo),
        }
    }
}

impl<T> Clone for DieselMiddleware<T>
where
    T: Connection + 'static,
{
    fn clone(&self) -> Self {
        match catch_unwind(|| self.repo.clone()) {
            Ok(repo) => DieselMiddleware {
                repo: AssertUnwindSafe(repo),
            },
            Err(_) => {
                error!("PANIC: r2d2::Pool::clone caused a panic");
                eprintln!("PANIC: r2d2::Pool::clone caused a panic");
                process::abort()
            }
        }
    }
}

impl<T> NewMiddleware for DieselMiddleware<T>
where
    T: Connection + 'static,
{
    type Instance = DieselMiddleware<T>;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        match catch_unwind(|| self.repo.clone()) {
            Ok(repo) => Ok(DieselMiddleware {
                repo: AssertUnwindSafe(repo),
            }),
            Err(_) => {
                error!(
                    "PANIC: r2d2::Pool::clone caused a panic, unable to rescue with a HTTP error"
                );
                eprintln!(
                    "PANIC: r2d2::Pool::clone caused a panic, unable to rescue with a HTTP error"
                );
                process::abort()
            }
        }
    }
}

impl<T> Middleware for DieselMiddleware<T>
where
    T: Connection + 'static,
{
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture> + 'static,
        Self: Sized,
    {
        trace!("[{}] pre chain", request_id(&state));
        state.put(self.repo.clone());

        let f = chain(state).and_then(move |(state, response)| {
            {
                trace!("[{}] post chain", request_id(&state));
            }
            future::ok((state, response))
        });
        Box::new(f)
    }
}
