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
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use tokio::time::timeout;

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
    pub fn mime(&mut self, mime: Mime) -> &mut Self {
        self.header(
            CONTENT_TYPE,
            mime.to_string().parse::<HeaderValue>().unwrap(),
        );
        self
    }

    pub fn body(&mut self, body: Body) -> &mut Self {
        self.body.replace(body);
        self
    }

    pub async fn perform(self) -> anyhow::Result<AsyncTestResponse> {
        let Self {
            test_client,
            request_builder,
            body,
        } = self;

        let request = request_builder.body(body.unwrap_or_default())?;
        test_client.request(request).await
    }

    pub fn extension<T>(&mut self, extension: T) -> &mut Self
    where
        T: Any + Send + Sync + 'static,
    {
        self.replace_request_builder(|builder| builder.extension(extension))
    }

    pub fn header<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        self.replace_request_builder(|builder| builder.header(key, value))
    }

    pub fn method<M>(&mut self, method: M) -> &mut Self
    where
        Method: TryFrom<M>,
        <Method as TryFrom<M>>::Error: Into<http::Error>,
    {
        self.replace_request_builder(|builder| builder.method(method))
    }

    pub fn uri<U>(&mut self, uri: U) -> &mut Self
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        self.replace_request_builder(|builder| builder.uri(uri))
    }

    pub fn version(&mut self, version: Version) -> &mut Self {
        self.replace_request_builder(|builder| builder.version(version))
    }

    fn replace_request_builder(
        &mut self,
        replacer: impl FnOnce(request::Builder) -> request::Builder,
    ) -> &mut Self {
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
