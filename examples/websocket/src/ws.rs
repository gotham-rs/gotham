use base64;
use futures::prelude::*;
use gotham::hyper::header::{HeaderValue, CONNECTION, UPGRADE};
use gotham::hyper::{self, upgrade::Upgraded, Body, HeaderMap, Response, StatusCode};
use sha1::Sha1;
use tokio_tungstenite::{tungstenite, WebSocketStream};

pub use tungstenite::protocol::{Message, Role};
pub use tungstenite::Error;

const PROTO_WEBSOCKET: &str = "websocket";
const SEC_WEBSOCKET_KEY: &str = "Sec-WebSocket-Key";
const SEC_WEBSOCKET_ACCEPT: &str = "Sec-WebSocket-Accept";

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
    body: Body,
) -> Result<
    (
        Response<Body>,
        impl Future<Output = Result<WebSocketStream<Upgraded>, hyper::Error>>,
    ),
    (),
> {
    let res = response(headers)?;
    let ws = body.on_upgrade().and_then(|upgraded| {
        WebSocketStream::from_raw_socket(upgraded, Role::Server, None).map(Ok)
    });

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
