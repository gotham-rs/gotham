//! Contains helpers for Gotham applications to use during testing.
//!
//! See the [`TestServer`] and [`AsyncTestServer`] types for example usage.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::future::{self, BoxFuture};
use futures_util::FutureExt;
use http::Uri;
use hyper::service::Service;
use log::info;
use tokio::net::TcpStream;
use tokio::time::Sleep;

use crate::handler::NewHandler;
use crate::test::async_test::{AsyncTestClient, AsyncTestServerInner};
use crate::test::{self, TestClient, TestServerData};
use std::time::Duration;

/// The `TestServer` type, which is used as a harness when writing test cases for Hyper services
/// (which Gotham's `Router` is). An instance of `TestServer` is run asynchronously within the
/// current thread, and is only accessible by a client returned from the `TestServer`.
///
/// # Examples
///
/// ```rust
/// # extern crate hyper;
/// # extern crate gotham;
/// #
/// # use gotham::state::State;
/// # use hyper::{Body, Response, StatusCode};
/// #
/// # fn my_handler(state: State) -> (State, Response<Body>) {
/// #   (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
/// # }
/// #
/// # fn main() {
/// use gotham::test::TestServer;
///
/// let test_server = TestServer::new(|| Ok(my_handler)).unwrap();
///
/// let response = test_server.client().get("http://localhost/").perform().unwrap();
/// assert_eq!(response.status(), StatusCode::ACCEPTED);
/// # }
/// ```
#[derive(Clone)]
pub struct TestServer {
    data: Arc<TestServerData>,
}

impl test::Server for TestServer {
    fn run_future<F, O>(&self, future: F) -> O
    where
        F: Future<Output = O>,
    {
        self.data.run_future(future)
    }

    fn request_expiry(&self) -> Sleep {
        self.data.request_expiry()
    }
}

impl TestServer {
    /// Creates a `TestServer` instance for the `Handler` spawned by `new_handler`. This server has
    /// the same guarantee given by `hyper::server::Http::bind`, that a new service will be spawned
    /// for each connection.
    ///
    /// Timeout will be set to 10 seconds.
    pub fn new<NH: NewHandler + 'static>(new_handler: NH) -> anyhow::Result<TestServer> {
        TestServer::with_timeout(new_handler, 10)
    }

    /// Sets the request timeout to `timeout` seconds and returns a new `TestServer`.
    pub fn with_timeout<NH: NewHandler + 'static>(
        new_handler: NH,
        timeout: u64,
    ) -> anyhow::Result<TestServer> {
        let data = TestServerData::new(new_handler, timeout, future::ok)?;

        Ok(TestServer {
            data: Arc::new(data),
        })
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally.
    pub fn client(&self) -> TestClient<Self, TestConnect> {
        self.data.client(self)
    }

    /// Spawns the given future on the `TestServer`'s internal runtime.
    /// This allows you to spawn more futures ontop of the `TestServer` in your
    /// tests.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.data.spawn(future)
    }
}

/// An [`AsyncTestServer`], that can be used for testing requests against a server in asynchronous contexts.
/// The [`AsyncTestServer`] runs in the runtime where it is created and an [`AsyncTestClient`] can be
/// created to make asynchronous requests to it.
///
/// This differs from [`crate::plain::test::TestServer`] in that it doesn't come with it's own runtime and therefore
/// doesn't crash when used inside of another runtime.
///
/// # Example
///
/// ```rust
/// # use gotham::state::State;
/// # use hyper::{Response, Body};
/// # use http::StatusCode;
/// #
/// # fn my_handler(state: State) -> (State, Response<Body>) {
/// #     (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
/// # }
/// #
/// # #[tokio::main]
/// # async fn main() {
/// use gotham::plain::test::AsyncTestServer;
///
/// let test_server = AsyncTestServer::new(|| Ok(my_handler)).await.unwrap();
///
/// let response = test_server.client().get("http://localhost/").perform().await.unwrap();
/// assert_eq!(response.status(), StatusCode::ACCEPTED);
/// # }
/// ```
#[derive(Clone)]
pub struct AsyncTestServer {
    inner: Arc<AsyncTestServerInner>,
}

impl AsyncTestServer {
    /// Creates an [`AsyncTestServer`] instance for the [`crate::handler::Handler`](`Handler`) spawned by `new_handler`. This server has
    /// the same guarantee given by [`hyper::server::Server::bind`], that a new service will be spawned
    /// for each connection.
    ///
    /// Requests will time out after 10 seconds by default. Use [`AsyncTestServer::with_timeout`] for a different timeout.
    pub async fn new<NH: NewHandler + 'static>(new_handler: NH) -> anyhow::Result<AsyncTestServer> {
        AsyncTestServer::new_with_timeout(new_handler, Duration::from_secs(10)).await
    }

    /// Sets the request timeout to `timeout` seconds and returns a new [`AsyncTestServer`].
    pub async fn new_with_timeout<NH: NewHandler + 'static>(
        new_handler: NH,
        timeout: Duration,
    ) -> anyhow::Result<AsyncTestServer> {
        let inner = AsyncTestServerInner::new(new_handler, timeout, future::ok).await?;

        Ok(AsyncTestServer {
            inner: Arc::new(inner),
        })
    }

    /// Returns a client connected to the [`AsyncTestServer`]. It can be used to make requests against the test server.
    /// The transport is handled internally.
    pub fn client(&self) -> AsyncTestClient<super::test::TestConnect> {
        self.inner.client()
    }
}

/// `TestConnect` represents the connection between a test client and the `TestServer` instance
/// that created it. This type should never be used directly.
#[derive(Clone)]
pub struct TestConnect {
    pub(crate) addr: SocketAddr,
}

impl Service<Uri> for TestConnect {
    type Response = TcpStream;
    type Error = tokio::io::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _req: Uri) -> Self::Future {
        TcpStream::connect(self.addr)
            .inspect(|s| info!("Client TcpStream connected: {:?}", s))
            .boxed()
    }
}

impl From<SocketAddr> for TestConnect {
    fn from(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helper::TestHandler;
    use crate::test::{self, async_test, Server};
    use tokio::sync::oneshot;

    #[test]
    fn test_server_serves_requests() {
        test::common_tests::serves_requests(TestServer::new, TestServer::client)
    }

    #[test]
    fn test_server_times_out() {
        test::common_tests::times_out(TestServer::with_timeout, TestServer::client)
    }

    #[test]
    fn test_server_async_echo() {
        test::common_tests::async_echo(TestServer::new, TestServer::client)
    }

    #[test]
    fn test_server_supports_multiple_servers() {
        test::common_tests::supports_multiple_servers(TestServer::new, TestServer::client)
    }

    #[test]
    fn test_server_spawns_and_runs_futures() {
        let server = TestServer::new(TestHandler::default()).unwrap();

        let (sender, spawn_receiver) = oneshot::channel();
        let (spawn_sender, run_receiver) = oneshot::channel();
        sender.send(1).unwrap();
        server.spawn(async move {
            assert_eq!(1, spawn_receiver.await.unwrap());
            spawn_sender.send(42).unwrap();
        });
        assert_eq!(42, server.run_future(run_receiver).unwrap());
    }

    #[test]
    fn test_server_adds_client_address_to_state() {
        test::common_tests::adds_client_address_to_state(TestServer::new, TestServer::client);
    }

    #[tokio::test]
    async fn async_test_server_serves_requests() {
        async_test::common_tests::serves_requests(AsyncTestServer::new, AsyncTestServer::client)
            .await;
    }

    #[tokio::test]
    async fn async_test_server_times_out() {
        async_test::common_tests::times_out(
            AsyncTestServer::new_with_timeout,
            AsyncTestServer::client,
        )
        .await;
    }

    #[tokio::test]
    async fn async_test_server_echo() {
        async_test::common_tests::echo(AsyncTestServer::new, AsyncTestServer::client).await;
    }

    #[tokio::test]
    async fn async_test_server_supports_multiple_servers() {
        async_test::common_tests::supports_multiple_servers(
            AsyncTestServer::new,
            AsyncTestServer::client,
        )
        .await;
    }

    #[tokio::test]
    async fn async_test_server_adds_client_address_to_state() {
        async_test::common_tests::adds_client_address_to_state(
            AsyncTestServer::new,
            AsyncTestServer::client,
        )
        .await;
    }
}
