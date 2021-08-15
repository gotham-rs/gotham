//! Contains the [`AsyncTestServer`] for testing gotham servers from an asynchronous context.
use crate::async_test::{AsyncTestClient, AsyncTestServerInner};
use crate::handler::NewHandler;
use futures_util::future;
use std::sync::Arc;
use std::time::Duration;

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
/// use gotham::plain::async_test::AsyncTestServer;
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
        AsyncTestServer::with_timeout(new_handler, Duration::from_secs(10)).await
    }

    /// Sets the request timeout to `timeout` seconds and returns a new [`AsyncTestServer`].
    pub async fn with_timeout<NH: NewHandler + 'static>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_test::common_tests;

    #[tokio::test]
    async fn serves_requests() {
        common_tests::serves_requests(AsyncTestServer::new, AsyncTestServer::client).await;
    }

    #[tokio::test]
    async fn times_out() {
        common_tests::times_out(AsyncTestServer::with_timeout, AsyncTestServer::client).await;
    }

    #[tokio::test]
    async fn echo() {
        common_tests::echo(AsyncTestServer::new, AsyncTestServer::client).await;
    }

    #[tokio::test]
    async fn supports_multiple_servers() {
        common_tests::supports_multiple_servers(AsyncTestServer::new, AsyncTestServer::client)
            .await;
    }
}
