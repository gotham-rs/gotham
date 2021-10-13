use futures_util::future::{MapErr, TryFutureExt};
use log::{error, info};
use std::io;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{rustls, Accept, TlsAcceptor};

use super::handler::NewHandler;
use super::{bind_server, new_runtime, tcp_listener};

#[cfg(feature = "testing")]
pub mod test;

/// Starts a Gotham application with the default number of threads.
pub fn start<NH, A>(addr: A, new_handler: NH, tls_config: rustls::ServerConfig) -> io::Result<()>
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
) -> io::Result<()>
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs + 'static + Send,
{
    let runtime = new_runtime(threads);
    runtime.block_on(init_server(addr, new_handler, tls_config))
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
) -> io::Result<()>
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

    let wrap = rustls_wrap(tls_config);
    bind_server(listener, new_handler, wrap).await
}

pub(crate) fn rustls_wrap(
    tls_config: rustls::ServerConfig,
) -> impl Fn(TcpStream) -> MapErr<Accept<TcpStream>, fn(std::io::Error) -> ()> {
    // function instead of closure, so the type is nameable, since impl ... impl is not allowed
    fn log_error(error: std::io::Error) {
        error!(target: "gotham::tls", "TLS handshake error: {:?}", error);
    }

    let tls = TlsAcceptor::from(Arc::new(tls_config));
    move |socket| tls.accept(socket).map_err(log_error)
}
