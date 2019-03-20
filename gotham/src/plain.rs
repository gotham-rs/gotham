use std::net::ToSocketAddrs;
use tokio::runtime::TaskExecutor;
use futures::{Future, Stream};
use log::info;
use tokio::net::TcpListener;
use std::sync::Arc;
use hyper::server::conn::Http;
use tokio::executor;

use super::{new_runtime, tcp_listener};
use super::{handler::NewHandler,service::GothamService};

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

  bind_server(listener, new_handler)
}

fn bind_server<NH>(listener: TcpListener, new_handler: NH) -> impl Future<Item = (), Error = ()>
where
  NH: NewHandler + 'static,
{
  let protocol = Arc::new(Http::new());
  let gotham_service = GothamService::new(new_handler);

  listener
    .incoming()
    .map_err(|e| panic!("socket error = {:?}", e))
    .for_each(move |socket| {
      let addr = socket.peer_addr().unwrap();
      let service = gotham_service.connect(addr);
      let handler = protocol.serve_connection(socket, service).then(|_| Ok(()));

      executor::spawn(handler);

      Ok(())
    })
}
