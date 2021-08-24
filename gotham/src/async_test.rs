//! Behavior and helpers shared between [`tls::async_test::AsyncTestServer`]
//! and [`plain::async_test::AsyncTestServer`].
use crate::handler::NewHandler;
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
use std::convert::TryFrom;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

pub(crate) struct AsyncTestServerInner {
    addr: SocketAddr,
    timeout: Duration,
    handle: tokio::task::JoinHandle<()>,
}

impl AsyncTestServerInner {
    pub(crate) async fn new<NH, F, Wrapped, Wrap>(
        new_handler: NH,
        timeout: Duration,
        wrap: Wrap,
    ) -> anyhow::Result<Self>
    where
        NH: NewHandler + 'static,
        F: Future<Output = Result<Wrapped, ()>> + Unpin + Send + 'static,
        Wrapped: Unpin + AsyncRead + AsyncWrite + Send + 'static,
        Wrap: Fn(TcpStream) -> F + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0".parse::<SocketAddr>()?).await?;
        let addr = listener.local_addr()?;

        let handle = tokio::spawn(async {
            // TODO: Remove the wrapping async block once ! is stabilized, see https://github.com/rust-lang/rust/issues/35121
            super::bind_server(listener, new_handler, wrap).await;
        });

        Ok(AsyncTestServerInner {
            addr,
            timeout,
            handle,
        })
    }

    pub(crate) fn client<TestC>(self: &Arc<Self>) -> AsyncTestClient<TestC>
    where
        TestC: From<SocketAddr> + Connect + Clone + Send + Sync + 'static,
    {
        // We're creating a private TCP-based pipe here. Bind to an ephemeral port, connect to
        // it and then immediately discard the listener.
        let test_connect = TestC::from(self.addr);
        let client = Client::builder().build(test_connect);
        AsyncTestClient::new(client, self.timeout, self.clone())
    }
}

impl Drop for AsyncTestServerInner {
    fn drop(&mut self) {
        // Prevent leaking the server's main loop
        self.handle.abort();
    }
}

/// Client interface for issuing requests to an [`AsyncTestServer`].
///
/// Most methods return an [`AsyncTestRequestBuilder`] that can be used to
/// build a request.
pub struct AsyncTestClient<C: Connect> {
    client: Client<C, Body>,
    timeout: Duration,
    // keeps the test server alive as long as there is still a client.
    _test_server: Arc<AsyncTestServerInner>,
}

impl<C: Connect + Clone + Send + Sync + 'static> AsyncTestClient<C> {
    pub(crate) fn new(
        client: Client<C, Body>,
        timeout: Duration,
        test_server: Arc<AsyncTestServerInner>,
    ) -> Self {
        Self {
            client,
            timeout,
            _test_server: test_server,
        }
    }

    /// Performs the given [`Request`] using this [`AsyncTestClient`]
    pub async fn request(&self, request: Request<Body>) -> anyhow::Result<AsyncTestResponse> {
        let request_future = self.client.request(request);
        Ok(timeout(self.timeout, request_future).await??.into())
    }

    /// Begin constructing a `HEAD` request using this [`AsyncTestClient`]
    pub fn head<U>(&self, uri: U) -> AsyncTestRequestBuilder<'_, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.request_builder_with_method_and_uri(Method::HEAD, uri)
    }

    /// Begin constructing a `GET` request using this [`AsyncTestClient`]
    pub fn get<U>(&self, uri: U) -> AsyncTestRequestBuilder<'_, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.request_builder_with_method_and_uri(Method::GET, uri)
    }

    /// Begin constructing an `OPTIONS` request using this [`AsyncTestClient`]
    pub fn options<U>(&self, uri: U) -> AsyncTestRequestBuilder<'_, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.request_builder_with_method_and_uri(Method::OPTIONS, uri)
    }

    /// Begin constructing a `POST` request using this [`AsyncTestClient`]
    pub fn post<U>(&self, uri: U) -> AsyncTestRequestBuilder<'_, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.request_builder_with_method_and_uri(Method::POST, uri)
    }

    /// Begin constructing a `PUT` request using this [`AsyncTestClient`]
    pub fn put<U>(&self, uri: U) -> AsyncTestRequestBuilder<'_, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.request_builder_with_method_and_uri(Method::PUT, uri)
    }

    /// Begin constructing a `PATCH` request using this [`AsyncTestClient`]
    pub fn patch<U>(&self, uri: U) -> AsyncTestRequestBuilder<'_, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.request_builder_with_method_and_uri(Method::PATCH, uri)
    }

    /// Begin constructing a `DELETE` request using this [`AsyncTestClient`]
    pub fn delete<U>(&self, uri: U) -> AsyncTestRequestBuilder<'_, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.request_builder_with_method_and_uri(Method::DELETE, uri)
    }

    /// Begin constructing a request using this [`AsyncTestClient`]
    pub fn build_request(&self) -> AsyncTestRequestBuilder<'_, C> {
        AsyncTestRequestBuilder {
            test_client: self,
            request_builder: request::Builder::new(),
            body: None,
        }
    }

    fn request_builder_with_method_and_uri<U>(
        &self,
        method: Method,
        uri: U,
    ) -> AsyncTestRequestBuilder<'_, C>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        let request_builder = request::Builder::new().uri(uri).method(method);
        AsyncTestRequestBuilder {
            test_client: self,
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

/// Builder for a request made with an [`AsyncTestClient`].
///
/// Once a request is fully built, it can be performed using the [`perform`] method.
pub struct AsyncTestRequestBuilder<'client, C: Connect> {
    test_client: &'client AsyncTestClient<C>,
    request_builder: request::Builder,
    body: Option<Body>,
}

impl<'client, C: Connect + Clone + Send + Sync + 'static> AsyncTestRequestBuilder<'client, C> {
    /// Perform the built request.
    pub async fn perform(self) -> anyhow::Result<AsyncTestResponse> {
        let Self {
            test_client,
            request_builder,
            body,
        } = self;

        let request = request_builder.body(body.unwrap_or_default())?;
        test_client.request(request).await
    }

    /// Convenience method to append a `content-type` header for the given MIME type.
    pub fn mime(self, mime: Mime) -> Self {
        self.header(
            CONTENT_TYPE,
            mime.to_string().parse::<HeaderValue>().unwrap(),
        )
    }

    /// Set a Body for this request. See [`http::request::Builder::body`].
    /// Other than the [`http::request::Builder::body`] it doesn't finish building
    /// the request though, instead if called multiple times, only the last one is kept.
    /// Defaults to [`Body::empty`] if never called.
    pub fn body<B: Into<Body>>(mut self, body: B) -> Self {
        self.body.replace(body.into());
        self
    }

    /// Add a custom value to this request. See [`http::request::Builder::extension`]
    pub fn extension<T>(self, extension: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        self.replace_request_builder(|builder| builder.extension(extension))
    }

    /// Add a header to this request. See [`http::request::Builder::header`]
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        self.replace_request_builder(|builder| builder.header(key, value))
    }

    /// Set the method of this request. See [`http::request::Builder::method`]
    pub fn method<M>(self, method: M) -> Self
    where
        Method: TryFrom<M>,
        <Method as TryFrom<M>>::Error: Into<http::Error>,
    {
        self.replace_request_builder(|builder| builder.method(method))
    }

    /// Set the [`Uri`] of this request. See [`http::request::Builder::uri`]
    pub fn uri<U>(self, uri: U) -> Self
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.replace_request_builder(|builder| builder.uri(uri))
    }

    /// Set the [`Version`](HTTP Version) of this Request. [`http::request::Builder::version`].
    pub fn version(self, version: Version) -> Self {
        self.replace_request_builder(|builder| builder.version(version))
    }

    fn replace_request_builder(
        mut self,
        replacer: impl FnOnce(request::Builder) -> request::Builder,
    ) -> Self {
        self.request_builder = replacer(self.request_builder);
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

/// Wrapper around a [`Response`] with some helper methods.
/// `Response::from(test_response)` can be used to get the underlying [`Response`]
pub struct AsyncTestResponse {
    response: Response<Body>,
}

impl AsyncTestResponse {
    /// Awaits the body of the underlying [`Response`] and returns it. This will run until
    /// all data has been received.
    pub async fn read_body(self) -> anyhow::Result<Vec<u8>> {
        let bytes = hyper::body::to_bytes(self.response.into_body()).await?;
        Ok(bytes.to_vec())
    }

    /// Awaits the UTF-8 encoded body of the underlying [`Response`] and returns it as a [`String`].
    /// This will run until all data has been received.
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
pub(crate) mod common_tests {
    use super::*;
    use crate::test::helper::TestHandler;
    use http::StatusCode;

    pub(crate) async fn serves_requests<TS, F, C>(
        server_factory: fn(TestHandler) -> F,
        client_factory: fn(&TS) -> AsyncTestClient<C>,
    ) where
        F: Future<Output = anyhow::Result<TS>>,
        C: Connect + Clone + Send + Sync + 'static,
    {
        let test_server = server_factory(TestHandler::from("response")).await.unwrap();
        let response = client_factory(&test_server)
            .get("http://localhost/")
            .perform()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.read_utf8_body().await.unwrap(), "response");
    }

    pub(crate) async fn times_out<TS, F, C>(
        server_factory: fn(TestHandler, Duration) -> F,
        client_factory: fn(&TS) -> AsyncTestClient<C>,
    ) where
        F: Future<Output = anyhow::Result<TS>>,
        C: Connect + Clone + Send + Sync + 'static,
    {
        let timeout = Duration::from_secs(10);
        let test_server = server_factory(TestHandler::default(), timeout)
            .await
            .unwrap();

        let client = client_factory(&test_server);

        tokio::time::pause();
        // Spawning the request into the background so the time can be controlled concurrently
        let request_handle =
            tokio::spawn(async move { client.get("http://localhost/timeout").perform().await });
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

    pub(crate) async fn echo<TS, F, C>(
        server_factory: fn(TestHandler) -> F,
        client_factory: fn(&TS) -> AsyncTestClient<C>,
    ) where
        F: Future<Output = anyhow::Result<TS>>,
        C: Connect + Clone + Send + Sync + 'static,
    {
        let server = server_factory(TestHandler::default()).await.unwrap();

        let data = "This text should get reflected back to us. Even this fancy piece of unicode: \
                    \u{3044}\u{308d}\u{306f}\u{306b}\u{307b}";

        let response = client_factory(&server)
            .post("http://localhost/echo")
            .body(data)
            .perform()
            .await
            .unwrap();
        let response_text = response.read_utf8_body().await.unwrap();
        assert_eq!(response_text, data);
    }

    pub(crate) async fn supports_multiple_servers<TS, F, C>(
        server_factory: fn(TestHandler) -> F,
        client_factory: fn(&TS) -> AsyncTestClient<C>,
    ) where
        F: Future<Output = anyhow::Result<TS>>,
        C: Connect + Clone + Send + Sync + 'static,
    {
        let server_a = server_factory(TestHandler::from("A")).await.unwrap();
        let server_b = server_factory(TestHandler::from("B")).await.unwrap();

        let client_a = client_factory(&server_a);
        let client_b = client_factory(&server_b);

        let response_a = client_a
            .get("http://localhost/")
            .perform()
            .await
            .unwrap()
            .read_utf8_body()
            .await
            .unwrap();
        let response_b = client_b
            .get("http://localhost/")
            .perform()
            .await
            .unwrap()
            .read_utf8_body()
            .await
            .unwrap();

        assert_eq!(response_a, "A");
        assert_eq!(response_b, "B");
    }

    pub(crate) async fn adds_client_address_to_state<TS, F, C>(
        server_factory: fn(TestHandler) -> F,
        client_factory: fn(&TS) -> AsyncTestClient<C>,
    ) where
        F: Future<Output = anyhow::Result<TS>>,
        C: Connect + Clone + Send + Sync + 'static,
    {
        let server = server_factory(TestHandler::default()).await.unwrap();
        let client = client_factory(&server);

        let client_address = client
            .get("http://localhost/myaddr")
            .perform()
            .await
            .unwrap()
            .read_utf8_body()
            .await
            .unwrap();
        assert!(client_address.starts_with("127.0.0.1"));
    }
}
