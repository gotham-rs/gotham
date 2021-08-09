//! Contains helpers for Gotham applications to use during testing.
//!
//! See the `TestServer` type for example usage.

use std::future::Future;
use std::net::{self, IpAddr, SocketAddr};
use std::panic::UnwindSafe;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::future::{self, BoxFuture};
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
use crate::test::{self, TestClient};

struct TestServerData {
    addr: SocketAddr,
    timeout: u64,
    runtime: RwLock<Runtime>,
}

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
    pub fn new<NH: NewHandler + 'static>(new_handler: NH) -> anyhow::Result<TestServer>
    where
        NH::Instance: UnwindSafe,
    {
        TestServer::with_timeout(new_handler, 10)
    }

    /// Sets the request timeout to `timeout` seconds and returns a new `TestServer`.
    pub fn with_timeout<NH: NewHandler + 'static>(
        new_handler: NH,
        timeout: u64,
    ) -> anyhow::Result<TestServer>
    where
        NH::Instance: UnwindSafe,
    {
        let runtime = Runtime::new()?;
        // TODO: Fix this into an async flow
        let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0".parse::<SocketAddr>()?))?;
        let addr = listener.local_addr()?;

        let service_stream = super::bind_server(listener, new_handler, future::ok);
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

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see a default socket address of `127.0.0.1:10000` as the source address for
    /// the connection.
    pub fn client(&self) -> TestClient<Self, TestConnect> {
        self.client_with_address(SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 10000))
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

    /// Returns a client connected to the `TestServer`. The transport is handled internally, and
    /// the server will see `client_addr` as the source address for the connection. The
    /// `client_addr` can be any valid `SocketAddr`, and need not be contactable.
    pub fn client_with_address(
        &self,
        client_addr: net::SocketAddr,
    ) -> TestClient<Self, TestConnect> {
        self.try_client_with_address(client_addr)
    }

    fn try_client_with_address(
        &self,
        _client_addr: net::SocketAddr,
    ) -> TestClient<Self, TestConnect> {
        // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
        // it and then immediately discard the listener.

        let client = Client::builder().build(TestConnect {
            addr: self.data.addr,
        });

        TestClient {
            client,
            test_server: self.clone(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::header::CONTENT_LENGTH;
    use hyper::{body, Body, StatusCode};
    use std::pin::Pin;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::handler::HandlerFuture;
    use crate::helpers::http::response::create_response;
    use crate::state::{FromState, State};
    use crate::test::helper::TestHandler;
    use http::header::CONTENT_TYPE;

    #[test]
    fn serves_requests() {
        let ticks = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let test_server = TestServer::new(TestHandler::from(format!("time: {}", ticks))).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let buf = response.read_utf8_body().unwrap();
        assert_eq!(buf, format!("time: {}", ticks));
    }

    #[test]
    #[ignore] // XXX I don't understand why this doesn't work.
              // It seems like Hyper is treating the future::empty() as an empty body...
    fn times_out() {
        let test_server = TestServer::with_timeout(TestHandler::from(""), 1).unwrap();

        let res = test_server
            .client()
            .get("http://localhost/timeout")
            .perform();

        match res {
            e @ Err(_) => {
                e.unwrap();
            }
            Ok(_) => panic!("expected timeout, but was Ok(_)"),
        }
    }

    #[test]
    #[ignore] // We trade using the mainline server setup code for this behavior.
    fn sets_client_addr() {
        let ticks = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let client_addr = "9.8.7.6:58901".parse().unwrap();
        let test_server = TestServer::new(TestHandler::from(format!("time: {}", ticks))).unwrap();

        let response = test_server
            .client_with_address(client_addr)
            .get("http://localhost/myaddr")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let buf = response.read_body().unwrap();
        let received_addr: net::SocketAddr = String::from_utf8(buf).unwrap().parse().unwrap();
        assert_eq!(received_addr, client_addr);
    }

    #[test]
    fn async_echo() {
        fn handler(mut state: State) -> Pin<Box<HandlerFuture>> {
            body::to_bytes(Body::take_from(&mut state))
                .then(move |full_body| match full_body {
                    Ok(body) => {
                        let resp_data = body.to_vec();
                        let res =
                            create_response(&state, StatusCode::OK, mime::TEXT_PLAIN, resp_data);
                        future::ok((state, res))
                    }

                    Err(e) => future::err((state, e.into())),
                })
                .boxed()
        }

        let server = TestServer::new(|| Ok(handler)).unwrap();

        let client = server.client();
        let data = "This text should get reflected back to us. Even this fancy piece of unicode: \
                    \u{3044}\u{308d}\u{306f}\u{306b}\u{307b}";

        let res = client
            .post("http://host/echo", data, mime::TEXT_PLAIN)
            .perform()
            .expect("request successful");

        assert_eq!(res.status(), StatusCode::OK);

        {
            let mime = res.headers().get(CONTENT_TYPE).expect("ContentType");
            assert_eq!(mime, mime::TEXT_PLAIN.as_ref());
        }

        let content_length = {
            let content_length = res.headers().get(CONTENT_LENGTH).expect("ContentLength");
            assert_eq!(content_length, &format!("{}", data.as_bytes().len()));
            content_length.clone()
        };

        let buf =
            String::from_utf8(res.read_body().expect("readable response")).expect("UTF8 response");

        assert_eq!(content_length, &format!("{}", buf.len()));
        assert_eq!(data, &buf);
    }
}
