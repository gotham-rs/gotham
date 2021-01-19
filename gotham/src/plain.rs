use futures::prelude::*;
use log::info;

use std::net::ToSocketAddrs;

use super::handler::NewHandler;
use super::{bind_server, new_runtime, tcp_listener};

pub mod test;

/// Starts a Gotham application on plain, unsecured HTTP.
pub fn start<NH, A>(addr: A, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static + Send,
{
    start_with_num_threads(addr, new_handler, num_cpus::get())
}

/// Starts a Gotham application with a designated number of threads.
pub fn start_with_num_threads<NH, A>(addr: A, new_handler: NH, threads: usize)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static + Send,
{
    let runtime = new_runtime(threads);
    let _ = runtime.block_on(async { init_server(addr, new_handler).await });
}

/// Returns a `Future` used to spawn an Gotham application.
///
/// This is used internally, but exposed in case the developer intends on doing any
/// manual wiring that isn't supported by the Gotham API. It's unlikely that this will
/// be required in most use cases; it's mainly exposed for shutdown handling.
pub async fn init_server<NH, A>(addr: A, new_handler: NH) -> Result<(), ()>
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static + Send,
{
    let listener = tcp_listener(addr).map_err(|_| ()).await?;
    let addr = listener.local_addr().unwrap();

    info!(
    target: "gotham::start",
    " Gotham listening on http://{}",
    addr
    );

    bind_server(listener, new_handler, future::ok).await
}
