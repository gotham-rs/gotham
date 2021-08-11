//! Contains helpers for Gotham applications to use during testing.
//!
//! See the `TestServer` type for example usage.

use std::future::Future;
use std::io::{self, BufReader};
use std::net::{self, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::future::{BoxFuture, FutureExt};
use http::Uri;
use hyper::client::connect::{Connected, Connection};
use hyper::client::Client;
use hyper::service::Service;
use log::info;
use pin_project::pin_project;
use rustls::Session;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::time::{sleep, Sleep};
use tokio_rustls::client::TlsStream;
use tokio_rustls::{
    rustls::{
        self,
        internal::pemfile::{certs, pkcs8_private_keys},
        NoClientAuth,
    },
    webpki::DNSNameRef,
    TlsConnector,
};

use crate::handler::NewHandler;
use crate::test::{self, TestClient, TestServerData};
use crate::tls::rustls_wrap;

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
    fn request_expiry(&self) -> Sleep {
        let runtime = self.data.runtime.write().unwrap();
        let _guard = runtime.enter();
        sleep(Duration::from_secs(self.data.timeout))
    }

    fn run_future<F, O>(&self, future: F) -> O
    where
        F: Future<Output = O>,
    {
        self.data
            .runtime
            .write()
            .expect("unable to acquire write lock")
            .block_on(future)
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
        let runtime = Runtime::new()?;
        // TODO: Fix this into an async flow
        let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0".parse::<SocketAddr>()?))?;
        let addr = listener.local_addr()?;

        let mut cfg = rustls::ServerConfig::new(NoClientAuth::new());
        let mut cert_file = BufReader::new(&include_bytes!("cert.pem")[..]);
        let mut key_file = BufReader::new(&include_bytes!("key.pem")[..]);
        let certs = certs(&mut cert_file).unwrap();
        let mut keys = pkcs8_private_keys(&mut key_file).unwrap();
        cfg.set_single_cert(certs, keys.remove(0))?;

        let wrap = rustls_wrap(cfg);
        let service_stream = super::bind_server(listener, new_handler, wrap);
        runtime.spawn(service_stream); // Ignore the result

        let data = TestServerData {
            addr,
            timeout,
            runtime: RwLock::new(runtime),
        };

        Ok(TestServer {
            data: Arc::new(data),
        })
    }

    /// Returns a client connected to the `TestServer`. The transport is handled internally.
    pub fn client(&self) -> TestClient<Self, TestConnect> {
        // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
        // it and then immediately discard the listener.
        let mut config = rustls::ClientConfig::new();
        let mut cert_file = BufReader::new(&include_bytes!("ca_cert.pem")[..]);
        config.root_store.add_pem_file(&mut cert_file).unwrap();

        let client = Client::builder().build(TestConnect {
            addr: self.data.addr,
            config: Arc::new(config),
        });

        TestClient {
            client,
            test_server: self.clone(),
        }
    }

    /// Spawns the given future on the `TestServer`'s internal runtime.
    /// This allows you to spawn more futures ontop of the `TestServer` in your
    /// tests.
    pub fn spawn<F>(&self, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.data
            .runtime
            .write()
            .expect("unable to acquire read lock")
            .spawn(fut);
    }

    /// Exactly the same as [`TestServer::client`].
    #[deprecated(note = "does the same as client")]
    pub fn client_with_address(
        &self,
        _client_addr: net::SocketAddr,
    ) -> TestClient<Self, TestConnect> {
        self.client()
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
    config: Arc<rustls::ClientConfig>,
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
