use diesel::r2d2::ConnectionManager;
use diesel::Connection;
use futures::future;
use futures::future::{poll_fn, Future};
use gotham_derive::StateData;
use log::error;
use r2d2::{CustomizeConnection, Pool, PooledConnection};
use tokio_threadpool::blocking;

/// A database "repository", for running database workloads.
/// Manages a connection pool and running blocking tasks using
/// `tokio_threadpool::blocking` which does not block the tokio event loop.
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # use diesel::prelude::*;
/// # use diesel::Queryable;
/// # let database_url = ":memory:";
/// # use diesel::sqlite::SqliteConnection;
/// # mod schema {
/// # table! {
/// #     users {
/// #         id -> Integer,
/// #         name -> VarChar,
/// #    }
/// # }
/// # }
///
/// #[derive(Queryable, Debug)]
/// pub struct User {
///     pub id: i32,
///     pub name: String,
/// }
///
/// type Repo = gotham_middleware_diesel::Repo<SqliteConnection>;
/// let repo = Repo::new(database_url);
/// # repo.run(|conn| {
/// #     conn.execute("CREATE TABLE IF NOT EXISTS users (
/// #         id INTEGER PRIMARY KEY AUTOINCREMENT,
/// #         name VARCHAR NOT NULL
/// #         )")
/// # });
/// let result = repo.run(|conn| {
///     use schema::users::dsl::*;
///     users.load::<User>(&conn)
/// });
///
/// ```
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
    /// Creates a repo with default connection pool settings.
    /// The default connection pool is `r2d2::Builder::default()`
    ///
    /// ```rust
    /// # use diesel::sqlite::SqliteConnection;
    ///
    /// type Repo = gotham_middleware_diesel::Repo<SqliteConnection>;
    /// // Accepts a database URL, e.g. "postgres://username:password@host/database"
    /// // for a postgres connection. Here we use an Sqlite in memory connection.
    /// let repo = Repo::new(":memory:");
    /// ```
    pub fn new(database_url: &str) -> Self {
        Self::from_pool_builder(database_url, r2d2::Builder::default())
    }

    /// Creates a repo with a pool builder, allowing you to customize
    /// any connection pool configuration.
    ///
    /// ```rust
    /// # use diesel::sqlite::SqliteConnection;
    /// use r2d2::Pool;
    /// use core::time::Duration;
    ///
    /// type Repo = gotham_middleware_diesel::Repo<SqliteConnection>;
    /// let database_url = ":memory:";
    /// let repo = Repo::from_pool_builder(database_url,
    ///     Pool::builder()
    ///         .connection_timeout(Duration::from_secs(120))
    ///         .max_size(100)
    /// );
    /// ```
    pub fn from_pool_builder(
        database_url: &str,
        builder: r2d2::Builder<ConnectionManager<T>>,
    ) -> Self {
        let manager = ConnectionManager::new(database_url);
        let connection_pool = builder
            .build(manager)
            .expect("could not initiate test db pool");
        Repo { connection_pool }
    }

    /// Creates a repo for use in tests, where queries are executed
    /// with an isolated test transaction and rolled back when
    /// the connection is dropped. This allows tests to run in parallel
    /// without impacting each other.
    /// ```rust
    /// # use diesel::sqlite::SqliteConnection;
    ///
    /// type Repo = gotham_middleware_diesel::Repo<SqliteConnection>;
    /// let repo = Repo::with_test_transactions(":memory:");
    /// ```
    pub fn with_test_transactions(database_url: &str) -> Self {
        let customizer = TestConnectionCustomizer {};
        let builder = Pool::builder().connection_customizer(Box::new(customizer));
        Self::from_pool_builder(database_url, builder)
    }

    /// Runs the given closure in a way that is safe for blocking IO to the
    /// database without blocking the tokio reactor.
    /// The closure will be passed a `Connection` from the pool to use.
    pub fn run<F, R, E>(&self, f: F) -> impl Future<Item = R, Error = E>
    where
        F: FnOnce(PooledConnection<ConnectionManager<T>>) -> Result<R, E>
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
        poll_fn(move || blocking(|| (f.take().unwrap())(pool.get().unwrap()))).then(
            |future_result| match future_result {
                Ok(query_result) => match query_result {
                    Ok(result) => future::ok(result),
                    Err(error) => future::err(error),
                },
                Err(e) => panic!("Error running async database task: {:?}", e),
            },
        )
    }
}

#[derive(Debug)]
pub struct TestConnectionCustomizer;

impl<C, E> CustomizeConnection<C, E> for TestConnectionCustomizer
where
    C: diesel::connection::Connection,
    E: std::error::Error + Sync + Send,
{
    fn on_acquire(&self, conn: &mut C) -> Result<(), E> {
        if let Err(e) = conn.begin_test_transaction() {
            error!("Error beginning test transaction: {}", e);
        }
        Ok(())
    }
}
