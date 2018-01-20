#![deny(warnings)]
extern crate futures;
extern crate gotham;
extern crate gotham_middleware_r2d2;
extern crate borrow_bag;

extern crate hyper;
extern crate mime;
extern crate r2d2;
extern crate r2d2_redis;
extern crate redis;

use gotham::router::Router;

pub mod config;
pub mod controllers;

pub fn init() -> Result<(String, Router), String> {
    let address = "127.0.0.1:7878";
    Ok((address.to_string(), config::routes::get()))
}