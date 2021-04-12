use futures::prelude::*;
use gotham::hyper::header::{
    HeaderValue, CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, UPGRADE,
};
use gotham::hyper::{
    self,
    upgrade::{OnUpgrade, Upgraded},
    Body, HeaderMap, Response, StatusCode,
};
use sha1::Sha1;
use tokio_tungstenite::{tungstenite, WebSocketStream};

pub use tungstenite::protocol::{Message, Role};
pub use tungstenite::Error;

const PROTO_WEBSOCKET: &str = "websocket";

/// Check if a WebSocket upgrade was requested.
pub fn requested(headers: &HeaderMap) -> bool {
    headers.get(UPGRADE) == Some(&HeaderValue::from_static(PROTO_WEBSOCKET))
}

/// Accept a WebSocket upgrade request.
///
/// Returns HTTP response, and a future that eventually resolves
/// into websocket object.
pub fn accept(
    headers: &HeaderMap,
    on_upgrade: OnUpgrade,
) -> Result<
    (
        Response<Body>,
        impl Future<Output = Result<WebSocketStream<Upgraded>, hyper::Error>>,
    ),
    (),
> {
    let res = response(headers)?;
    let ws = async move {
        let upgraded = on_upgrade.await?;
        Ok(WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await)
    };

    Ok((res, ws))
}

fn response(headers: &HeaderMap) -> Result<Response<Body>, ()> {
    let key = headers.get(SEC_WEBSOCKET_KEY).ok_or(())?;

    Ok(Response::builder()
        .header(UPGRADE, PROTO_WEBSOCKET)
        .header(CONNECTION, "upgrade")
        .header(SEC_WEBSOCKET_ACCEPT, accept_key(key.as_bytes()))
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .body(Body::empty())
        .unwrap())
}

fn accept_key(key: &[u8]) -> String {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::default();
    sha1.update(key);
    sha1.update(WS_GUID);
    base64::encode(&sha1.digest().bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_accept_key_from_rfc6455() {
        // From https://tools.ietf.org/html/rfc6455#section-1.2
        let key = accept_key("dGhlIHNhbXBsZSBub25jZQ==".as_bytes());
        assert_eq!(key, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
