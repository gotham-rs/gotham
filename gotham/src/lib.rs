//! Gotham &ndash; A flexible web framework that promotes stability, safety, security and speed.
//!
//! You can find out more about Gotham, including where to get help, at <https://gotham.rs>.
//!
//! We look forward to welcoming you into the Gotham community!
#![doc(html_root_url = "https://docs.rs/gotham/0.3.0")] // Update when changed in Cargo.toml
#![warn(missing_docs, deprecated)]
// Stricter requirements once we get to pull request stage, all warnings must be resolved.
#![cfg_attr(feature = "ci", deny(warnings))]
#![cfg_attr(
  feature = "cargo-clippy",
  allow(
    clippy::needless_lifetimes,
    clippy::should_implement_trait,
    clippy::unit_arg,
    clippy::match_wild_err_arm,
    clippy::new_without_default,
    clippy::wrong_self_convention,
    clippy::mutex_atomic,
    clippy::borrowed_box,
    clippy::get_unwrap,
    )
  )]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]
pub mod error;
pub mod extractor;
pub mod handler;
pub mod helpers;
pub mod middleware;
pub mod pipeline;
pub mod router;
mod service;
pub mod state;

/// Test utilities for Gotham and Gotham consumer apps.
pub mod test;

/// Functions for creating a Gotham service on plain HTTP.
/// In general, prefer the tls module.
pub mod plain;

/// Functions for creating a Gotham service on HTTPS.
pub mod tls;

use std::net::ToSocketAddrs;

use tokio::net::TcpListener;
use tokio::runtime::{self, Runtime};
use tokio_rustls::rustls;

use handler::NewHandler;

/// Starts a Gotham application with the default number of threads.
/// If `tls_config` is `None`, the resulting service will run on HTTP,
/// otherwise on HTTPS, which is preferred.
pub fn start<NH, A>(addr: A, new_handler: NH, tls_config: Option<rustls::ServerConfig>)
  where
  NH: NewHandler + 'static,
  A: ToSocketAddrs + 'static,
{
  match tls_config {
    Some(cfg) => tls::start(addr, new_handler, cfg),
    None => plain::start(addr, new_handler)
  }
}

fn new_runtime(threads: usize) -> Runtime {
  runtime::Builder::new()
    .core_threads(threads)
    .name_prefix("gotham-worker-")
    .build()
    .unwrap()
}

fn tcp_listener<A>(addr: A) -> TcpListener
where
A: ToSocketAddrs + 'static,
{
  let addr = match addr.to_socket_addrs().map(|ref mut i| i.next()) {
    Ok(Some(a)) => a,
    Ok(_) => panic!("unable to resolve listener address"),
    Err(_) => panic!("unable to parse listener address"),
  };

  TcpListener::bind(&addr).expect("unable to open TCP listener")
}
