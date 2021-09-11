//! Contains helpers for Gotham applications to use during testing.
//!
//! See the [`TestServer`] and [`AsyncTestServer`] types for example usage.

use std::future::Future;
use std::io::{self, BufReader};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::future::{BoxFuture, FutureExt};
use http::Uri;
use hyper::client::connect::{Connected, Connection};
use hyper::service::Service;
use log::info;
use pin_project::pin_project;
use rustls::Session;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio::time::Sleep;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::internal::pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{self, NoClientAuth};
use tokio_rustls::webpki::DNSNameRef;
use tokio_rustls::TlsConnector;

use crate::handler::NewHandler;
use crate::test::async_test::{AsyncTestClient, AsyncTestServerInner};
use crate::test::{self, TestClient, TestServerData};
use crate::tls::rustls_wrap;
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
/// use gotham::tls::test::TestServer;
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
        let mut cfg = rustls::ServerConfig::new(NoClientAuth::new());
        let mut cert_file = BufReader::new(&include_bytes!("cert.pem")[..]);
        let mut key_file = BufReader::new(&include_bytes!("key.pem")[..]);
        let certs = certs(&mut cert_file).unwrap();
        let mut keys = pkcs8_private_keys(&mut key_file).unwrap();
        cfg.set_single_cert(certs, keys.remove(0))?;

        let data = TestServerData::new(new_handler, timeout, rustls_wrap(cfg))?;

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
/// use gotham::tls::test::AsyncTestServer;
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
        let mut cfg = rustls::ServerConfig::new(NoClientAuth::new());
        let mut cert_file = BufReader::new(&include_bytes!("cert.pem")[..]);
        let mut key_file = BufReader::new(&include_bytes!("key.pem")[..]);
        let certs = certs(&mut cert_file).unwrap();
        let mut keys = pkcs8_private_keys(&mut key_file).unwrap();
        cfg.set_single_cert(certs, keys.remove(0))?;

        let inner = AsyncTestServerInner::new(new_handler, timeout, rustls_wrap(cfg)).await?;
        Ok(AsyncTestServer {
            inner: Arc::new(inner),
        })
    }

    /// Returns a client connected to the [`AsyncTestServer`]. It can be used to make requests against the test server.
    /// The transport is handled internally.
    pub fn client(&self) -> AsyncTestClient<crate::tls::test::TestConnect> {
        self.inner.client()
    }
}

#[allow(missing_docs)]
#[pin_project]
pub struct TlsConnectionStream<IO>(#[pin] TlsStream<IO>);

impl<IO: AsyncRead + AsyncWrite + Connection + Unpin> Connection for TlsConnectionStream<IO> {
    fn connected(&self) -> Connected {
        let (tcp, tls) = self.0.get_ref();
        if tls.get_alpn_protocol() == Some(b"h2") {
            tcp.connected().negotiated_h2()
        } else {
            tcp.connected()
        }
    }
}

impl<IO> AsyncRead for TlsConnectionStream<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<Result<(), io::Error>> {
        self.project().0.poll_read(cx, buf)
    }
}

impl<IO: AsyncRead + AsyncWrite + Unpin> AsyncWrite for TlsConnectionStream<IO> {
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().0.poll_write(cx, buf)
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().0.poll_flush(cx)
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().0.poll_shutdown(cx)
    }
}

/// `TestConnect` represents the connection between a test client and the `TestServer` instance
/// that created it. This type should never be used directly.
#[derive(Clone)]
pub struct TestConnect {
    pub(crate) addr: SocketAddr,
    pub(crate) config: Arc<rustls::ClientConfig>,
}

impl Service<Uri> for TestConnect {
    type Response = TlsConnectionStream<TcpStream>;
    type Error = tokio::io::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Uri) -> Self::Future {
        let tls = TlsConnector::from(self.config.clone());
        let address = self.addr;

        async move {
            match TcpStream::connect(address).await {
                Ok(stream) => {
                    let domain = DNSNameRef::try_from_ascii_str(req.host().unwrap()).unwrap();
                    match tls.connect(domain, stream).await {
                        Ok(tls_stream) => {
                            info!("Client TcpStream connected: {:?}", tls_stream);
                            Ok(TlsConnectionStream(tls_stream))
                        }
                        Err(error) => {
                            info!("TLS TestClient error: {:?}", error);
                            Err(error)
                        }
                    }
                }
                Err(error) => Err(error),
            }
        }
        .boxed()
    }
}

impl From<SocketAddr> for TestConnect {
    fn from(addr: SocketAddr) -> Self {
        let mut config = rustls::ClientConfig::new();
        let mut cert_file = BufReader::new(&include_bytes!("ca_cert.pem")[..]);
        config.root_store.add_pem_file(&mut cert_file).unwrap();

        Self {
            addr,
            config: Arc::new(config),
        }
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
