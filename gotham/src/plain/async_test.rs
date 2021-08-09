use crate::async_test::AsyncTestClient;
use crate::handler::NewHandler;
use crate::plain::test::TestConnect;
use futures_util::future;
use hyper::Client;
use std::net::SocketAddr;
use std::panic::UnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

pub struct AsyncTestServer {
    data: Arc<AsyncTestServerData>,
}

#[derive(Clone)]
struct AsyncTestServerData {
    addr: SocketAddr,
    timeout: Duration,
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
        AsyncTestServer::with_timeout(new_handler, Duration::from_secs(10)).await
    }

    /// Sets the request timeout to `timeout` seconds and returns a new `AsyncTestServer`.
    pub async fn with_timeout<NH: NewHandler + 'static>(
        new_handler: NH,
        timeout: Duration,
    ) -> anyhow::Result<AsyncTestServer>
    where
        NH::Instance: UnwindSafe, // TODO: Not quite sure why it must explicitly be UnwindSafe
    {
        let listener = TcpListener::bind("127.0.0.1:0".parse::<SocketAddr>()?).await?;
        let addr = listener.local_addr()?;

        // TODO: Won't this leak the server?
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
    pub fn client(&self) -> AsyncTestClient<TestConnect> {
        // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
        // it and then immediately discard the listener.
        let client = Client::builder().build(TestConnect {
            addr: self.data.addr,
        });
        AsyncTestClient::new(client, self.data.timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helper::TestHandler;
    use http::StatusCode;

    #[tokio::test]
    async fn serves_requests() {
        let test_server = AsyncTestServer::new(TestHandler::from("response"))
            .await
            .unwrap();
        let response = test_server
            .client()
            .get("http://localhost/")
            .unwrap()
            .perform()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.read_utf8_body().await.unwrap(), "response");
    }

    #[tokio::test]
    async fn times_out() {
        let timeout = Duration::from_secs(10);
        let test_server = AsyncTestServer::with_timeout(TestHandler::from(""), timeout)
            .await
            .unwrap();

        let client = test_server.client();

        tokio::time::pause();
        // Spawning the request into the background so the time can be controlled concurrently
        let request_handle = tokio::spawn(async move {
            let builder = client.get("http://localhost/timeout").unwrap();
            builder.perform().await
        });
        // This exploits Auto-advance, see https://docs.rs/tokio/1.9.0/tokio/time/fn.pause.html#auto-advance
        // Just calling `tokio::time::advance(timeout)` directly won't have any effect here, because the spawned
        // request future hasn't been polled yet so it's timer isn't registered, meaning the advance doesn't affect
        // the request's timeout in any way
        tokio::time::sleep(timeout).await;
        tokio::time::resume();

        let request_result = request_handle.await.unwrap();
        assert!(request_result
            .unwrap_err()
            .is::<tokio::time::error::Elapsed>());
    }
}
