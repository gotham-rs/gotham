use crate::handler::NewHandler;
use futures_util::future;
use http::header::CONTENT_TYPE;
use http::header::{HeaderName, HeaderValue};
use http::request;
use http::Version;
use http::{Method, Request, Uri};
use hyper::client::connect::Connect;
use hyper::Client;
use hyper::{Body, Response};
use mime::Mime;
use std::any::Any;
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::panic::UnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;

#[derive(Clone)]
pub struct AsyncTestServer {
    inner: Arc<AsyncTestServerInner>,
}

struct AsyncTestServerInner {
    addr: SocketAddr,
    timeout: Duration,
    handle: tokio::task::JoinHandle<()>,
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

        let handle = tokio::spawn(async {
            // TODO: Remove the wrapping async block once ! is stabilized, see https://github.com/rust-lang/rust/issues/35121
            super::bind_server(listener, new_handler, future::ok).await;
        });

        Ok(AsyncTestServer {
            inner: Arc::new(AsyncTestServerInner {
                addr,
                timeout,
                handle,
            }),
        })
    }

    /// Returns a client connected to the `AsyncTestServer`. The transport is handled internally, and
    /// the server will see a default socket address of `127.0.0.1:10000` as the source address for
    /// the connection.
    pub fn client(&self) -> AsyncTestClient<super::plain::test::TestConnect> {
        // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
        // it and then immediately discard the listener.
        let client = Client::builder().build(super::plain::test::TestConnect {
            addr: self.inner.addr,
        });
        AsyncTestClient::new(client, self.inner.timeout)
    }
}

impl Drop for AsyncTestServer {
    fn drop(&mut self) {
        // Prevent leaking the server's main loop
        self.inner.handle.abort();
    }
}

pub struct AsyncTestClient<C: Connect> {
    client: Client<C, Body>,
    timeout: Duration,
}

impl<C: Connect + Clone + Send + Sync + 'static> AsyncTestClient<C> {
    pub(crate) fn new(client: Client<C, Body>, timeout: Duration) -> Self {
        Self { client, timeout }
    }

    pub async fn request(&self, request: Request<Body>) -> anyhow::Result<AsyncTestResponse> {
        let request_future = self.client.request(request);
        Ok(timeout(self.timeout, request_future).await??.into())
    }

    pub fn head<U>(&self, uri: U) -> anyhow::Result<AsyncTestRequestBuilder<'_, C>>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Error + Send + Sync + 'static,
    {
        Ok(self.build_request(Method::HEAD, uri.try_into()?))
    }

    pub fn get<U>(&self, uri: U) -> anyhow::Result<AsyncTestRequestBuilder<'_, C>>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Error + Send + Sync + 'static,
    {
        Ok(self.build_request(Method::GET, uri.try_into()?))
    }

    pub fn options<U>(&self, uri: U) -> anyhow::Result<AsyncTestRequestBuilder<'_, C>>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Error + Send + Sync + 'static,
    {
        Ok(self.build_request(Method::OPTIONS, uri.try_into()?))
    }

    pub fn post<U>(&self, uri: U) -> anyhow::Result<AsyncTestRequestBuilder<'_, C>>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Error + Send + Sync + 'static,
    {
        Ok(self.build_request(Method::POST, uri.try_into()?))
    }

    pub fn put<U>(&self, uri: U) -> anyhow::Result<AsyncTestRequestBuilder<'_, C>>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Error + Send + Sync + 'static,
    {
        Ok(self.build_request(Method::PUT, uri.try_into()?))
    }

    pub fn patch<U>(&self, uri: U) -> anyhow::Result<AsyncTestRequestBuilder<'_, C>>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Error + Send + Sync + 'static,
    {
        Ok(self.build_request(Method::PATCH, uri.try_into()?))
    }

    pub fn delete<U>(&self, uri: U) -> anyhow::Result<AsyncTestRequestBuilder<'_, C>>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Error + Send + Sync + 'static,
    {
        Ok(self.build_request(Method::DELETE, uri.try_into()?))
    }

    pub fn build_request(&self, method: Method, uri: Uri) -> AsyncTestRequestBuilder<'_, C> {
        let request_builder = request::Builder::new().uri(uri).method(method);
        AsyncTestRequestBuilder {
            test_client: &self,
            request_builder,
            body: None,
        }
    }
}

impl<C: Connect> From<AsyncTestClient<C>> for Client<C> {
    fn from(test_client: AsyncTestClient<C>) -> Self {
        test_client.client
    }
}

pub struct AsyncTestRequestBuilder<'client, C: Connect> {
    test_client: &'client AsyncTestClient<C>,
    request_builder: request::Builder,
    body: Option<Body>,
}

impl<'client, C: Connect + Clone + Send + Sync + 'static> AsyncTestRequestBuilder<'client, C> {
    pub async fn perform(self) -> anyhow::Result<AsyncTestResponse> {
        let Self {
            test_client,
            request_builder,
            body,
        } = self;

        let request = request_builder.body(body.unwrap_or_default())?;
        test_client.request(request).await
    }

    pub fn mime(self, mime: Mime) -> Self {
        self.header(
            CONTENT_TYPE,
            mime.to_string().parse::<HeaderValue>().unwrap(),
        )
    }

    pub fn body<B: Into<Body>>(mut self, body: B) -> Self {
        self.body.replace(body.into());
        self
    }

    pub fn extension<T>(self, extension: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        self.replace_request_builder(|builder| builder.extension(extension))
    }

    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        self.replace_request_builder(|builder| builder.header(key, value))
    }

    pub fn method<M>(self, method: M) -> Self
    where
        Method: TryFrom<M>,
        <Method as TryFrom<M>>::Error: Into<http::Error>,
    {
        self.replace_request_builder(|builder| builder.method(method))
    }

    pub fn uri<U>(self, uri: U) -> Self
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.replace_request_builder(|builder| builder.uri(uri))
    }

    pub fn version(self, version: Version) -> Self {
        self.replace_request_builder(|builder| builder.version(version))
    }

    fn replace_request_builder(
        mut self,
        replacer: impl FnOnce(request::Builder) -> request::Builder,
    ) -> Self {
        let mut intermediary = request::Builder::new();
        // swap out request_builder so it can be modified
        std::mem::swap(&mut intermediary, &mut self.request_builder);

        intermediary = replacer(intermediary);

        // place it back after modification
        std::mem::swap(&mut intermediary, &mut self.request_builder);

        self
    }
}

impl<'client, C: Connect> Deref for AsyncTestRequestBuilder<'client, C> {
    type Target = request::Builder;

    fn deref(&self) -> &Self::Target {
        &self.request_builder
    }
}

impl<'client, C: Connect> DerefMut for AsyncTestRequestBuilder<'client, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.request_builder
    }
}

pub struct AsyncTestResponse {
    response: Response<Body>,
}

impl AsyncTestResponse {
    /// Awaits the body of the underlying `Response`, and returns it. This will cause the event
    /// loop to execute until the `Response` body has been fully read into the `Vec<u8>`.
    pub async fn read_body(self) -> anyhow::Result<Vec<u8>> {
        let bytes = hyper::body::to_bytes(self.response.into_body()).await?;
        Ok(bytes.to_vec())
    }

    /// Awaits the UTF-8 encoded body of the underlying `Response`, and returns the `String`. This
    /// will cause the event loop to execute until the `Response` body has been fully read and the
    /// `String` created.
    pub async fn read_utf8_body(self) -> anyhow::Result<String> {
        let bytes = self.read_body().await?;
        Ok(String::from_utf8(bytes)?)
    }
}

impl From<Response<Body>> for AsyncTestResponse {
    fn from(response: Response<Body>) -> Self {
        Self { response }
    }
}

impl From<AsyncTestResponse> for Response<Body> {
    fn from(test_response: AsyncTestResponse) -> Self {
        test_response.response
    }
}

impl Deref for AsyncTestResponse {
    type Target = Response<Body>;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

impl DerefMut for AsyncTestResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.response
    }
}

impl Debug for AsyncTestResponse {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("AsyncTestResponse")
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
        let test_server = AsyncTestServer::with_timeout(TestHandler::default(), timeout)
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

    #[tokio::test]
    async fn echo() {
        let server = AsyncTestServer::new(TestHandler::default()).await.unwrap();

        let data = "This text should get reflected back to us. Even this fancy piece of unicode: \
                    \u{3044}\u{308d}\u{306f}\u{306b}\u{307b}";

        let response = server
            .client()
            .post("http://localhost/echo")
            .unwrap()
            .body(data)
            .perform()
            .await
            .unwrap();
        let response_text = response.read_utf8_body().await.unwrap();
        assert_eq!(response_text, data);
    }
}
