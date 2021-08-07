use crate::handler::NewHandler;
use crate::plain::test::TestConnect;
use futures_util::future;
use hyper::Client;
use std::net::SocketAddr;
use std::panic::UnwindSafe;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct AsyncTestServer {
    data: Arc<AsyncTestServerData>,
}

#[derive(Clone)]
struct AsyncTestServerData {
    addr: SocketAddr,
    timeout: u64,
}

impl AsyncTestServer {
    /// Creates an `AsyncTestServer` instance for the `Handler` spawned by `new_handler`. This server has
    /// the same guarantee given by `hyper::server::Http::bind`, that a new service will be spawned
    /// for each connection.
    ///
    /// Timeout will be set to 10 seconds.
    pub async fn new<NH: NewHandler + 'static>(new_handler: NH) -> anyhow::Result<AsyncTestServer>
    where
        NH::Instance: UnwindSafe,
    {
        AsyncTestServer::with_timeout(new_handler, 10).await
    }

    /// Sets the request timeout to `timeout` seconds and returns a new `AsyncTestServer`.
    pub async fn with_timeout<NH: NewHandler + 'static>(
        new_handler: NH,
        timeout: u64,
    ) -> anyhow::Result<AsyncTestServer>
    where
        NH::Instance: UnwindSafe, // TODO: Not quite sure why it must explicitly be UnwindSafe
    {
        let listener = TcpListener::bind("127.0.0.1:0".parse::<SocketAddr>()?).await?;
        let addr = listener.local_addr()?;

        let service_stream = super::bind_server(listener, new_handler, future::ok);
        let _ = tokio::spawn(service_stream);

        let data = AsyncTestServerData { addr, timeout };

        Ok(AsyncTestServer {
            data: Arc::new(data),
        })
    }

    /// Returns a client connected to the `AsyncTestServer`. The transport is handled internally, and
    /// the server will see a default socket address of `127.0.0.1:10000` as the source address for
    /// the connection.
    pub fn client(&self) -> Client<TestConnect> {
        // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
        // it and then immediately discard the listener.
        Client::builder().build(TestConnect {
            addr: self.data.addr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_test::read_utf8_body;
    use crate::handler::{Handler, HandlerFuture};
    use crate::state::{client_addr, FromState, State};
    use futures_util::FutureExt;
    use http::{StatusCode, Uri};
    use hyper::{Body, Response};
    use log::info;
    use std::pin::Pin;
    use std::time::{SystemTime, UNIX_EPOCH};

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
                        .body(self.response.into())
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

    #[tokio::test]
    async fn serves_requests() {
        let ticks = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_service = move || {
            Ok(TestHandler {
                response: format!("time: {}", ticks),
            })
        };

        let test_server = AsyncTestServer::new(new_service).await.unwrap();
        let response = test_server
            .client()
            .get(Uri::from_static("http://localhost/"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let buf = read_utf8_body(response).await.unwrap();
        assert_eq!(buf, format!("time: {}", ticks));
    }
}
