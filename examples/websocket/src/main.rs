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

        hyper::rt::spawn(ws);

        (state, response)
    } else {
        (state, Response::new(Body::from(INDEX_HTML)))
    }
}

fn connected<S>(req_id: String, stream: S) -> impl Future<Output = Result<(), ()>>
where
    S: Stream<Item = ws::Message, Error = ws::Error>
        + Sink<SinkItem = ws::Message, SinkError = ws::Error>,
{
    let (sink, stream) = stream.split();

    println!("Client {} connected", req_id);

    sink.send_all(stream.map({
        let req_id = req_id.clone();
        move |msg| {
            println!("{}: {:?}", req_id, msg);
            msg
        }
    }))
    .map_err(|err| println!("Websocket error: {}", err))
    .map(move |_| println!("Client {} disconnected", req_id))
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
