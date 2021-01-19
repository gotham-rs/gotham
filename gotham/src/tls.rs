use futures::prelude::*;
use log::{error, info};
use std::net::ToSocketAddrs;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio_rustls::{rustls, TlsAcceptor};

use super::{bind_server, new_runtime, tcp_listener};

use super::handler::NewHandler;

pub mod test;

/// Starts a Gotham application with the default number of threads.
pub fn start<NH, A>(addr: A, new_handler: NH, tls_config: rustls::ServerConfig)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static + Send,
{
    start_with_num_threads(addr, new_handler, tls_config, num_cpus::get())
}

/// Starts a Gotham application with a designated number of threads.
pub fn start_with_num_threads<NH, A>(
    addr: A,
    new_handler: NH,
    tls_config: rustls::ServerConfig,
    threads: usize,
) where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static + Send,
{
    let runtime = new_runtime(threads);
    let _ = runtime.block_on(async { init_server(addr, new_handler, tls_config).await });
}

/// Returns a `Future` used to spawn an Gotham application.
///
/// This is used internally, but exposed in case the developer intends on doing any
/// manual wiring that isn't supported by the Gotham API. It's unlikely that this will
/// be required in most use cases; it's mainly exposed for shutdown handling.
pub async fn init_server<NH, A>(
    addr: A,
    new_handler: NH,
    tls_config: rustls::ServerConfig,
) -> Result<(), ()>
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

    bind_server_rustls(listener, new_handler, tls_config)
        .map_err(|_| ())
        .await
}

async fn bind_server_rustls<NH>(
    listener: TcpListener,
    new_handler: NH,
    tls_config: rustls::ServerConfig,
) -> Result<(), ()>
where
    NH: NewHandler + 'static,
{
    let tls = TlsAcceptor::from(Arc::new(tls_config));
    bind_server(listener, new_handler, move |socket| {
        tls.accept(socket).map_err(|e| {
            error!(target: "gotham::tls", "TLS handshake error: {:?}", e);
        })
    })
    .await
}
