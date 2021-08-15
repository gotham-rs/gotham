//! Contains helpers for Gotham applications to use during testing.
//!
//! See the `TestServer` type for example usage.

use std::future::Future;
use std::net::{self, SocketAddr};
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
use crate::test::{self, TestClient, TestServerData};

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

    /// Exactly the same as [`TestServer::client`].
    #[deprecated(since = "0.3.0", note = "does the same as client")]
    pub fn client_with_address(
        &self,
        _client_addr: net::SocketAddr,
    ) -> TestClient<Self, TestConnect> {
        self.client()
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
    use crate::test::{common_tests, Server};
    use tokio::sync::oneshot;

    #[test]
    fn serves_requests() {
        common_tests::serves_requests(TestServer::new, TestServer::client)
    }

    #[test]
    fn times_out() {
        common_tests::times_out(TestServer::with_timeout, TestServer::client)
    }

    #[test]
    fn async_echo() {
        common_tests::async_echo(TestServer::new, TestServer::client)
    }

    #[test]
    fn supports_multiple_servers() {
        common_tests::supports_multiple_servers(TestServer::new, TestServer::client)
    }

    #[test]
    fn spawns_and_runs_futures() {
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
    fn adds_client_address_to_state() {
        common_tests::adds_client_address_to_state(TestServer::new, TestServer::client);
    }
}
