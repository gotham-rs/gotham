use futures_util::{Sink, SinkExt, Stream, StreamExt};
use gotham::hyper::upgrade::OnUpgrade;
use gotham::hyper::{Body, HeaderMap, Response, StatusCode};
use gotham::prelude::*;
use gotham::state::{request_id, State};

mod ws;

fn main() {
    pretty_env_logger::init();

    let addr = "127.0.0.1:7878";
    println!("Listening on http://{}/", addr);

    gotham::start(addr, || Ok(handler)).unwrap();
}

fn handler(mut state: State) -> (State, Response<Body>) {
    let headers = HeaderMap::take_from(&mut state);
    let on_upgrade = OnUpgrade::try_take_from(&mut state);

    match on_upgrade {
        Some(on_upgrade) if ws::requested(&headers) => {
            let (response, ws) = match ws::accept(&headers, on_upgrade) {
                Ok(res) => res,
                Err(_) => return (state, bad_request()),
            };

            let req_id = request_id(&state).to_owned();

            tokio::spawn(async move {
                match ws.await {
                    Ok(ws) => connected(req_id, ws).await,
                    Err(err) => {
                        eprintln!("websocket init error: {}", err);
                        Err(())
                    }
                }
            });

            (state, response)
        }
        _ => (state, Response::new(Body::from(INDEX_HTML))),
    }
}

async fn connected<S>(req_id: String, stream: S) -> Result<(), ()>
where
    S: Stream<Item = Result<ws::Message, ws::Error>> + Sink<ws::Message, Error = ws::Error>,
{
    let (mut sink, mut stream) = stream.split();

    println!("Client {} connected", req_id);

    while let Some(message) = stream
        .next()
        .await
        .transpose()
        .map_err(|error| println!("Websocket receive error: {}", error))?
    {
        println!("{}: {:?}", req_id, message);
        match sink.send(message).await {
            Ok(()) => (),
            // this error indicates a successfully closed connection
            Err(ws::Error::ConnectionClosed) => break,
            Err(error) => {
                println!("Websocket send error: {}", error);
                return Err(());
            }
        }
    }

    println!("Client {} disconnected", req_id);
    Ok(())
}

fn bad_request() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::empty())
        .unwrap()
}

const INDEX_HTML: &str = include_str!("index.html");

#[cfg(test)]
mod test {
    use super::*;
    use crate::ws::{Message, Role};
    use gotham::hyper::header::{
        HeaderValue, CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, UPGRADE,
    };
    use gotham::hyper::upgrade;
    use gotham::plain::test::AsyncTestServer;
    use tokio_tungstenite::WebSocketStream;

    async fn create_test_server() -> AsyncTestServer {
        AsyncTestServer::new(|| Ok(handler))
            .await
            .expect("Failed to create TestServer")
    }

    #[tokio::test]
    async fn server_should_respond_with_html_if_no_websocket_was_requested() {
        let server = create_test_server().await;
        let client = server.client();

        let response = client
            .get("http://localhost:10000")
            .perform()
            .await
            .expect("Failed to request HTML");
        let body = response
            .read_utf8_body()
            .await
            .expect("Failed to read response body.");

        assert_eq!(body, INDEX_HTML);
    }

    #[tokio::test]
    async fn server_should_echo_websocket_messages() {
        let server = create_test_server().await;
        let client = server.client();

        let response = client
            .get("ws://127.0.0.1:10000")
            .header(UPGRADE, HeaderValue::from_static("websocket"))
            .header(SEC_WEBSOCKET_KEY, HeaderValue::from_static("QmF0bWFu"))
            .perform()
            .await
            .expect("Failed to request websocket upgrade");

        let connection_header = response
            .headers()
            .get(CONNECTION)
            .expect("Missing connection header");
        assert_eq!(connection_header.as_bytes(), "upgrade".as_bytes());
        let upgrade_header = response
            .headers()
            .get(UPGRADE)
            .expect("Missing upgrade header");
        assert_eq!(upgrade_header.as_bytes(), "websocket".as_bytes());
        let websocket_accept_header = response
            .headers()
            .get(SEC_WEBSOCKET_ACCEPT)
            .expect("Missing websocket accept header");
        assert_eq!(
            websocket_accept_header.as_bytes(),
            "hRHWRk+NDTj5O2GjSexJZg8ImzI=".as_bytes()
        );

        let response: Response<_> = response.into();
        let upgraded = upgrade::on(response)
            .await
            .expect("Failed to upgrade client websocket.");
        let mut websocket_stream =
            WebSocketStream::from_raw_socket(upgraded, Role::Client, None).await;

        let message = Message::Text("Hello".to_string());
        websocket_stream
            .send(message.clone())
            .await
            .expect("Failed to send text message.");

        let response = websocket_stream
            .next()
            .await
            .expect("Socket was closed")
            .expect("Failed to receive response");
        assert_eq!(message, response);

        websocket_stream
            .send(Message::Close(None))
            .await
            .expect("Failed to send close message");
    }

    #[tokio::test]
    async fn should_respond_with_bad_request_if_the_request_is_bad() {
        let server = create_test_server().await;
        let client = server.client();

        let response = client
            .get("ws://127.0.0.1:10000")
            .header(UPGRADE, HeaderValue::from_static("websocket"))
            .perform()
            .await
            .expect("Failed to perform request");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response
            .read_body()
            .await
            .expect("Failed to read response body");
        assert!(body.is_empty());
    }
}
