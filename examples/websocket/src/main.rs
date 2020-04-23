use futures::prelude::*;
use gotham::hyper::{Body, HeaderMap, Response, StatusCode};
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

const INDEX_HTML: &str = r#"
<!DOCTYPE html>
<body>
<h1>Websocket Echo Server</h1>
<form id="ws" onsubmit="return send(this.message);">
    <input name="message">
    <input type="submit" value="Send">
</form>
<script>
    var sock = new WebSocket("ws://" + window.location.host);
    sock.onopen = recv.bind(window, "Connected");
    sock.onclose = recv.bind(window, "Disconnected");
    sock.onmessage = function(msg) { recv(msg.data) };
    sock.onerror = function(err) { recv("Error: " + e); };

    function recv(msg) {
        var e = document.createElement("PRE");
        e.innerText = msg;
        document.body.appendChild(e);
    }

    function send(msg) {
        sock.send(msg.value);
        msg.value = "";
        return false;
    }
</script>
</body>
"#;
