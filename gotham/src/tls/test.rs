//! Contains helpers for Gotham applications to use during testing.
//!
//! See the `TestServer` type for example usage.

use http::Uri;
use hyper::client::connect::{Connected, Connection};
use std::io::{self, BufReader};
use std::net::{self, IpAddr, SocketAddr};
use std::panic::UnwindSafe;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::Duration;

use log::info;

use futures::prelude::*;
use hyper::client::Client;
use hyper::service::Service;
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
/// use gotham::tls::test::TestServer;
///
/// let test_server = TestServer::new(|| Ok(my_handler)).unwrap();
///
/// let response = test_server.client().get("http://localhost/").perform().unwrap();
/// assert_eq!(response.status(), StatusCode::ACCEPTED);
/// # }
/// ```
pub struct TestServer {
    data: Arc<TestServerData>,
}

impl Clone for TestServer {
    fn clone(&self) -> TestServer {
        TestServer {
            data: self.data.clone(),
        }
    }
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

        let mut cfg = rustls::ServerConfig::new(NoClientAuth::new());
        let mut cert_file = BufReader::new(&include_bytes!("cert.pem")[..]);
        let mut key_file = BufReader::new(&include_bytes!("key.pem")[..]);
        let certs = certs(&mut cert_file).unwrap();
        let mut keys = pkcs8_private_keys(&mut key_file).unwrap();
        cfg.set_single_cert(certs, keys.remove(0))?;

        let service_stream = super::bind_server_rustls(listener, new_handler, cfg);
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
            .expect("TestServer: unable to spawn client")
    }

    fn try_client_with_address(
        &self,
        _client_addr: net::SocketAddr,
    ) -> anyhow::Result<TestClient<Self, TestConnect>> {
        // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
        // it and then immediately discard the listener.
        let mut config = rustls::ClientConfig::new();
        let mut cert_file = BufReader::new(&include_bytes!("ca_cert.pem")[..]);
        config.root_store.add_pem_file(&mut cert_file).unwrap();

        let client = Client::builder().build(TestConnect {
            addr: self.data.addr,
            config: Arc::new(config),
        });

        Ok(TestClient {
            client,
            test_server: self.clone(),
        })
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
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Uri) -> Self::Future {
        let tls = TlsConnector::from(self.config.clone());

        TcpStream::connect(self.addr)
            .and_then(move |stream| {
                let domain = DNSNameRef::try_from_ascii_str(req.host().unwrap()).unwrap();
                tls.connect(domain, stream)
                    .inspect(|s| info!("Client TcpStream connected: {:?}", s))
                    .map_ok(TlsConnectionStream)
                    .map_err(|e| {
                        info!("TLS TestClient error: {:?}", e);
                        e
                    })
            })
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::{SystemTime, UNIX_EPOCH};

    use hyper::header::CONTENT_LENGTH;
    use hyper::{body, Body, Response, StatusCode, Uri};
    use mime;

    use crate::handler::{Handler, HandlerFuture, NewHandler};
    use crate::helpers::http::response::create_response;
    use crate::state::{client_addr, FromState, State};
    use http::header::CONTENT_TYPE;
    use log::info;

    #[derive(Clone)]
    struct TestHandler {
        response: String,
    }

    impl Handler for TestHandler {
        fn handle(self, state: State) -> Pin<Box<HandlerFuture>> {
            let path = Uri::borrow_from(&state).path().to_owned();
            match path.as_str() {
                "/" => {
                    info!("TestHandler responding to /");
                    let response = Response::builder()
                        .status(StatusCode::OK)
                        .body(self.response.clone().into())
                        .unwrap();

                    future::ok((state, response)).boxed()
                }
                "/timeout" => {
                    // TODO: What is this supposed to return?  It previously returned nothing which isn't a timeout
                    let response = Response::builder()
                        .status(StatusCode::REQUEST_TIMEOUT)
                        .body(Body::default())
                        .unwrap();

                    info!("TestHandler responding to /timeout");
                    future::ok((state, response)).boxed()
                }

                "/myaddr" => {
                    info!("TestHandler responding to /myaddr");
                    let response = Response::builder()
                        .status(StatusCode::OK)
                        .body(format!("{}", client_addr(&state).unwrap()).into())
                        .unwrap();

                    future::ok((state, response)).boxed()
                }
                _ => unreachable!(),
            }
        }
    }

    impl NewHandler for TestHandler {
        type Instance = Self;

        fn new_handler(&self) -> anyhow::Result<Self> {
            Ok(self.clone())
        }
    }

    #[test]
    fn serves_requests() {
        let ticks = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_service = move || {
            Ok(TestHandler {
                response: format!("time: {}", ticks),
            })
        };

        let test_server = TestServer::new(new_service).unwrap();

        let response = test_server
            .client()
            .get("https://example.com/")
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
        let new_service = || {
            Ok(TestHandler {
                response: "".to_owned(),
            })
        };

        let test_server = TestServer::with_timeout(new_service, 1).unwrap();

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

        let new_service = move || {
            Ok(TestHandler {
                response: format!("time: {}", ticks),
            })
        };

        let client_addr = "9.8.7.6:58901".parse().unwrap();
        let test_server = TestServer::new(new_service).unwrap();

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
            let f =
                body::to_bytes(Body::take_from(&mut state)).then(
                    move |full_body| match full_body {
                        Ok(body) => {
                            let resp_data = body.to_vec();
                            let res = create_response(
                                &state,
                                StatusCode::OK,
                                mime::TEXT_PLAIN,
                                resp_data,
                            );
                            future::ok((state, res))
                        }

                        Err(e) => future::err((state, e.into())),
                    },
                );

            f.boxed()
        }

        let server = TestServer::new(|| Ok(handler)).unwrap();

        let client = server.client();
        let data = "This text should get reflected back to us. Even this fancy piece of unicode: \
                    \u{3044}\u{308d}\u{306f}\u{306b}\u{307b}";

        let res = client
            .post("https://example.com/echo", data, mime::TEXT_PLAIN)
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
