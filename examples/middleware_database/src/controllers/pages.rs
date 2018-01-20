use futures::future;
use hyper::StatusCode;
use mime;
use redis;
use gotham::handler::HandlerFuture;
use gotham::http::response::create_response;
use gotham::state::State;
use gotham_middleware_r2d2::state_data::try_connection;

use r2d2_redis::RedisConnectionManager;

pub fn database(state: State) -> Box<HandlerFuture> {
    let get_conn = try_connection::<RedisConnectionManager>(&state);
    let res = match get_conn {
        Ok(pool) => {
            let reply = redis::cmd("PING").query::<String>(&*pool).unwrap();
            create_response(
                &state,
                StatusCode::Ok,
                Some((reply.to_string().into_bytes(), mime::TEXT_PLAIN))
            )
        },
        Err(_err) => {
            create_response(
                &state,
                StatusCode::InternalServerError,
                Some(("Error connecting to redis".to_string().into_bytes(), mime::TEXT_PLAIN))
            )
        },
    };
    Box::new(future::lazy(move || future::ok((state, res))))
}