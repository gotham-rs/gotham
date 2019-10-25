use futures::{Future, IntoFuture};
use log::info;
use std::net::ToSocketAddrs;
use tokio::runtime::TaskExecutor;

use super::handler::NewHandler;
use super::{bind_server, new_runtime, tcp_listener};

pub mod test;

/// Starts a Gotham application on plain, unsecured HTTP.
pub fn start<NH, A>(addr: A, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static,
{
    start_with_num_threads(addr, new_handler, num_cpus::get())
}

/// Starts a Gotham application with a designated number of threads.
pub fn start_with_num_threads<NH, A>(addr: A, new_handler: NH, threads: usize)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static,
{
    let runtime = new_runtime(threads);
    start_on_executor(addr, new_handler, runtime.executor());
    runtime.shutdown_on_idle().wait().unwrap();
}

/// Starts a Gotham application with a designated backing `TaskExecutor`.
///
/// This function can be used to spawn the server on an existing `Runtime`.
pub fn start_on_executor<NH, A>(addr: A, new_handler: NH, executor: TaskExecutor)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static,
{
    executor.spawn(init_server(addr, new_handler));
}

/// Returns a `Future` used to spawn an Gotham application.
///
/// This is used internally, but exposed in case the developer intends on doing any
/// manual wiring that isn't supported by the Gotham API. It's unlikely that this will
/// be required in most use cases; it's mainly exposed for shutdown handling.
pub fn init_server<NH, A>(addr: A, new_handler: NH) -> impl Future<Item = (), Error = ()>
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static,
{
    let listener = tcp_listener(addr);
    let addr = listener.local_addr().unwrap();

    info!(
    target: "gotham::start",
    " Gotham listening on http://{}",
    addr
    );

    bind_server(listener, new_handler, |tcp| Ok(tcp).into_future())
}
