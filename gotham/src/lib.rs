//! Gotham &ndash; A flexible web framework that promotes stability, safety, security and speed.
//!
//! You can find out more about Gotham, including where to get help, at
//! <https://gotham.rs/>.
//!
//! We look forward to welcoming you into the Gotham community!
#![doc(html_root_url = "https://docs.rs/gotham/0.7.4")]
// Update when changed in Cargo.toml
// Stricter requirements once we get to pull request stage, all warnings must be resolved.
#![cfg_attr(feature = "ci", deny(warnings))]
#![allow(
    clippy::needless_lifetimes,
    clippy::should_implement_trait,
    clippy::unit_arg,
    clippy::match_wild_err_arm,
    clippy::new_without_default,
    clippy::wrong_self_convention,
    clippy::mutex_atomic,
    clippy::borrowed_box,
    clippy::get_unwrap
)]
#![warn(missing_docs, rust_2018_idioms, unreachable_pub)]
#![deny(elided_lifetimes_in_paths, unsafe_code)]
#![doc(test(no_crate_inject, attr(deny(warnings))))]

pub mod extractor;
pub mod handler;
pub mod helpers;
pub mod middleware;
pub mod pipeline;
pub mod prelude;
pub mod router;
pub mod service;
pub mod state;

/// Test utilities for Gotham and Gotham consumer apps.
#[cfg(feature = "testing")]
pub mod test;

/// Functions for creating a Gotham service using HTTP.
pub mod plain;

/// Functions for creating a Gotham service using HTTPS.
#[cfg(feature = "__tls")]
pub mod tls;

/// Re-export anyhow
pub use anyhow;
/// Re-export bytes
pub use bytes;
/// Re-export cookie
pub use cookie;
/// Re-export http
pub use http;
/// Re-export http-body
pub use http_body;
/// Re-export http-body-util
pub use http_body_util;
/// Re-export hyper
pub use hyper;
/// Re-export hyper-util
pub use hyper_util;
/// Re-export mime
pub use mime;
/// Re-export tower-service
pub use tower_service;

/// Re-export rustls
#[cfg(feature = "__tls")]
pub use tokio_rustls::rustls;

use futures_util::TryFutureExt;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::future::Future;
use std::io;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{self, Runtime};

use crate::handler::NewHandler;
use crate::service::GothamService;

pub use plain::*;
#[cfg(feature = "__tls")]
pub use tls::start as start_with_tls;

/// The error that can occur when starting the gotham server.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StartError {
    /// I/O error.
    #[error("I/O Error: {0}")]
    IoError(#[from] io::Error),
}

fn new_runtime(threads: usize) -> Runtime {
    runtime::Builder::new_multi_thread()
        .worker_threads(threads)
        .thread_name("gotham-worker")
        .enable_all()
        .build()
        .unwrap()
}

async fn tcp_listener<A>(addr: A) -> io::Result<TcpListener>
where
    A: ToSocketAddrs + 'static,
{
    let addr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::other("unable to resolve listener address"))?;
    TcpListener::bind(addr).await
}

/// Returns a `Future` used to spawn a Gotham application.
///
/// This is used internally, but it's exposed for clients that want to set up their own TLS
/// support. The wrap argument is a function that will receive a tokio-io TcpStream and should wrap
/// the socket as necessary. Errors returned by this function will be ignored and the connection
/// will be dropped if the future returned by the wrapper resolves to an error.
pub async fn bind_server<NH, F, Wrapped, Wrap>(
    listener: TcpListener,
    new_handler: NH,
    wrap: Wrap,
) -> !
where
    NH: NewHandler + 'static,
    F: Future<Output = Result<Wrapped, ()>> + Unpin + Send + 'static,
    Wrapped: Unpin + AsyncRead + AsyncWrite + Send + 'static,
    Wrap: Fn(TcpStream) -> F,
{
    let protocol = Arc::new(hyper_util::server::conn::auto::Builder::new(
        TokioExecutor::new(),
    ));
    let gotham_service = GothamService::new(new_handler);

    loop {
        let (socket, addr) = match listener.accept().await {
            Ok(ok) => ok,
            Err(err) => {
                log::error!("Socket Error: {}", err);
                continue;
            }
        };

        let service = gotham_service.connect(addr);
        let accepted_protocol = Arc::clone(&protocol);
        let wrapper = wrap(socket);

        // NOTE: HTTP protocol errors and handshake errors are ignored here (i.e. so the socket
        // will be dropped).
        let task = async move {
            let socket = wrapper.await?;

            accepted_protocol
                .serve_connection_with_upgrades(TokioIo::new(socket), service)
                .map_err(drop)
                .await?;

            Result::<_, ()>::Ok(())
        };

        tokio::spawn(task);
    }
}
