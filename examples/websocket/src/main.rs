use futures::prelude::*;
use gotham::hyper::{Body, HeaderMap, Response, StatusCode};
use gotham::hyper::header::{HeaderValue, UPGRADE, SEC_WEBSOCKET_KEY};
use gotham::state::{request_id, FromState, State};

mod ws;

fn main() {
    pretty_env_logger::init();

    let addr = "127.0.0.1:7878";
    println!("Listening on http://{}/", addr);

    gotham::start(addr, || Ok(handler));
}

fn handler(mut state: State) -> (State, Response<Body>) {
    let body = Body::take_from(&mut state);
    let headers = HeaderMap::take_from(&mut state);

    if ws::requested(&headers) {
        let (response, ws) = match ws::accept(&headers, body) {
            Ok(res) => res,
            Err(_) => return bad_request(state),
        };

        let req_id = request_id(&state).to_owned();
        let ws = ws
            .map_err(|err| eprintln!("websocket init error: {}", err))
            .and_then(move |ws| connected(req_id, ws));

        tokio::spawn(ws);

        (state, response)
    } else {
        (state, Response::new(Body::from(INDEX_HTML)))
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

fn bad_request(state: State) -> (State, Response<Body>) {
    let response = Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::empty())
        .unwrap();
    (state, response)
}

const INDEX_HTML: &str = include_str!("index.html");

#[cfg(test)]
mod test {
    use super::*;
    use gotham::plain::test::TestServer;
    use std::ops::DerefMut;
    use gotham::test::Server;
    use tokio_tungstenite::WebSocketStream;
    use crate::ws::{Role, Message};
    use std::fmt::{Display, Formatter};
    use std::error::Error;

    fn create_test_server() -> TestServer {
        TestServer::new(|| Ok(handler)).expect("Failed to create TestServer")
    }

    #[test]
    fn server_should_respond_with_html_if_no_websocket_was_requested() {
        let server = create_test_server();
        let client = server.client();

        let response = client.get("http://localhost:10000").perform().expect("Failed to request HTML");
        let body = response.read_utf8_body().expect("Failed to read response body.");

        assert_eq!(body, INDEX_HTML);
    }

    #[test]
    fn server_should_echo_websocket_messages() {
        let server = create_test_server();
        let client = server.client();

        let mut request = client.get("ws://127.0.0.1:10000");
        let headers = request.headers_mut();
        headers.insert(UPGRADE, HeaderValue::from_static("websocket"));
        headers.insert(SEC_WEBSOCKET_KEY, HeaderValue::from_static("QmF0bWFu"));

        let mut response = client.perform(request).expect("Failed to request websocket upgrade");
        // This will be used to swap out the body from the TestResponse because it only implements `DerefMut` but not `Into<Response>`
        let mut body = Body::empty();
        std::mem::swap(&mut body, response.deref_mut().body_mut());

        server.run_future(async move {
            let upgraded = body.on_upgrade().await.expect("Failed to upgrade client websocket.");
            let mut websocket_stream = WebSocketStream::from_raw_socket(upgraded, Role::Client, None).await;

            let message = Message::Text("Hello".to_string());
            websocket_stream.send(message.clone()).await.expect("Failed to send text message.");

            let response = websocket_stream.next().await.expect("Socket was closed").expect("Failed to receive response");
            assert_eq!(message, response);
            Ok::<(), DummyError>(())
        }).unwrap();
    }

    #[derive(Debug)]
    struct DummyError {}

    impl Display for DummyError {
        fn fmt(&self, _formatter: &mut Formatter) -> std::fmt::Result {
            Ok(())
        }
    }

    impl Error for DummyError {}

}
