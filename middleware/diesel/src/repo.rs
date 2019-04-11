use diesel::r2d2::ConnectionManager;
use diesel::Connection;
use futures::future::{poll_fn, Future};
use gotham_derive::StateData;
use r2d2::{Pool, PooledConnection};
use tokio_threadpool::{blocking, BlockingError};

/// A database "repository", for running database workloads.
/// Manages a connection pool and running blocking tasks using
/// `tokio_threadpool::blocking` which does not block the tokio event loop.
#[derive(StateData)]
pub struct Repo<T>
where
    T: Connection + 'static,
{
    connection_pool: Pool<ConnectionManager<T>>,
}

impl<T> Clone for Repo<T>
where
    T: Connection + 'static,
{
    fn clone(&self) -> Repo<T> {
        Repo {
            connection_pool: self.connection_pool.clone(),
        }
    }
}

impl<T> Repo<T>
where
    T: Connection + 'static,
{
    pub fn new(database_url: &str) -> Self {
        Repo {
            connection_pool: Repo::connection_pool(database_url),
        }
    }

    pub fn connection_pool(database_url: &str) -> Pool<ConnectionManager<T>> {
        let manager = ConnectionManager::new(database_url);
        Repo::configure_pool(manager)
    }

    #[cfg(test)]
    fn configure_pool(manager: ConnectionManager<T>) -> Pool<ConnectionManager<T>> {
        let customizer = TestConnectionCustomizer {};

        Pool::builder()
            .connection_customizer(Box::new(customizer))
            .build(manager)
            .expect("could not initiate test db pool")
    }

    #[cfg(not(test))]
    fn configure_pool(manager: ConnectionManager<T>) -> Pool<ConnectionManager<T>> {
        Pool::new(manager).expect("could not initiate db pool")
    }
    /// Runs the given closure in a way that is safe for blocking IO to the database.
    /// The closure will be passed a `Connection` from the pool to use.
    pub fn run<F, R>(&self, f: F) -> impl Future<Item = R, Error = BlockingError>
    where
        F: FnOnce(PooledConnection<ConnectionManager<T>>) -> R
            + Send
            + std::marker::Unpin
            + 'static,
        T: Send + 'static,
    {
        let pool = self.connection_pool.clone();
        // `tokio_threadpool::blocking` returns a `Poll` which can be converted into a future
        // using `poll_fn`.
        // `f.take()` allows the borrow checker to be sure `f` is not moved into the inner closure
        // multiple times if `poll_fn` is called multple times.
        let mut f = Some(f);
        poll_fn(move || blocking(|| (f.take().unwrap())(pool.get().unwrap())))
    }
}

#[cfg(test)]
use r2d2::CustomizeConnection;

#[cfg(test)]
#[derive(Debug)]
pub struct TestConnectionCustomizer;

#[cfg(test)]
impl<C, E> CustomizeConnection<C, E> for TestConnectionCustomizer
where
    C: diesel::connection::Connection,
    E: std::error::Error + Sync + Send,
{
    fn on_acquire(&self, conn: &mut C) -> Result<(), E> {
        match conn.begin_test_transaction() {
            Ok(_) => Ok(()),
            Err(_) => Ok(()),
        }
    }
}
