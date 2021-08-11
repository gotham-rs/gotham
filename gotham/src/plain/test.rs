//! Contains helpers for Gotham applications to use during testing.
//!
//! See the `TestServer` type for example usage.

use std::future::Future;
use std::net::{self, SocketAddr};
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::future::{self, BoxFuture, Ready};
use futures_util::FutureExt;
use http::Uri;
use hyper::client::Client;
use hyper::service::Service;
use log::info;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::time::{sleep, Sleep};

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

        let wrap = create_wrap()?;
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
        let test_connect = TestConnect::from(self.data.addr);
        let client = Client::builder().build(test_connect);

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

fn create_wrap() -> anyhow::Result<fn(TcpStream) -> Ready<Result<TcpStream, ()>>> {
    Ok(future::ok)
}
