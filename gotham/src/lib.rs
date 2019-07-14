//! Gotham &ndash; A flexible web framework that promotes stability, safety, security and speed.
//!
//! You can find out more about Gotham, including where to get help, at <https://gotham.rs>.
//!
//! We look forward to welcoming you into the Gotham community!
#![doc(html_root_url = "https://docs.rs/gotham/0.4.0")] // Update when changed in Cargo.toml
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

/// Functions for creating a Gotham service using HTTP.
pub mod plain;

/// Functions for creating a Gotham service using HTTPS.
pub mod tls;

use std::net::ToSocketAddrs;

use tokio::net::TcpListener;
use tokio::runtime::{self, Runtime};

pub use plain::*;
pub use tls::start as start_with_tls;

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
    let addr = addr
        .to_socket_addrs()
        .expect("unable to parse listener address")
        .next()
        .expect("unable to resolve listener address");

    TcpListener::bind(&addr).expect("unable to open TCP listener")
}
