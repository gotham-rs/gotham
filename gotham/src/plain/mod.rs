use futures_util::future;
use log::info;
use std::net::ToSocketAddrs;

use super::handler::NewHandler;
use super::{bind_server, new_runtime, tcp_listener, StartError};

#[cfg(feature = "testing")]
pub mod test;

/// Starts a Gotham application on plain, unsecured HTTP.
pub fn start<NH, A>(addr: A, new_handler: NH) -> Result<(), StartError>
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static + Send,
{
    start_with_num_threads(addr, new_handler, num_cpus::get())
}

/// Starts a Gotham application with a designated number of threads.
pub fn start_with_num_threads<NH, A>(
    addr: A,
    new_handler: NH,
    threads: usize,
) -> Result<(), StartError>
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static + Send,
{
    let runtime = new_runtime(threads);
    runtime.block_on(init_server(addr, new_handler))
}

/// Returns a `Future` used to spawn an Gotham application.
///
/// This is used internally, but exposed in case the developer intends on doing any
/// manual wiring that isn't supported by the Gotham API. It's unlikely that this will
/// be required in most use cases; it's mainly exposed for shutdown handling.
pub async fn init_server<NH, A>(addr: A, new_handler: NH) -> Result<(), StartError>
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static + Send,
{
    let listener = tcp_listener(addr).await?;
    let addr = listener.local_addr().unwrap();

    info! {
        target: "gotham::start",
        " Gotham listening on http://{}", addr
    }

    bind_server(listener, new_handler, future::ok).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;
    use hyper::{Body, Response};

    fn handler(_: State) -> (State, Response<Body>) {
        unimplemented!()
    }

    #[test]
    fn test_error_on_invalid_port() {
        let res = start("0.0.0.0:99999", || Ok(handler));
        assert!(res.is_err());
    }
}
