//! Gotham &ndash; A flexible web framework that promotes stability, safety, security and speed.
//!
//! You can find out more about Gotham, including where to get help, at <https://gotham.rs>.
//!
//! We look forward to welcoming you into the Gotham community!
#![doc(html_root_url = "https://docs.rs/gotham/0.2.1")] // Update when changed in Cargo.toml
#![warn(missing_docs, deprecated)]
// Stricter requirements once we get to pull request stage, all warnings must be resolved.
#![cfg_attr(feature = "ci", deny(warnings))]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

extern crate base64;
extern crate bincode;
extern crate borrow_bag;
extern crate chrono;
#[cfg(windows)]
extern crate crossbeam;
extern crate futures;
extern crate futures_timer;
#[macro_use]
extern crate hyper;
extern crate linked_hash_map;
#[macro_use]
extern crate log;
extern crate mime;
extern crate mio;
extern crate rand;
extern crate regex;
#[macro_use]
extern crate serde;
extern crate tokio;
extern crate url;
extern crate uuid;

#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub mod extractor;
pub mod handler;
pub mod middleware;
pub mod pipeline;
pub mod http;
pub mod router;
mod service;
mod start;
pub mod state;
// pub mod test;

pub use start::start;
