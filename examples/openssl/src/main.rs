//! A Hello World example application for working with Gotham.

extern crate failure;
extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate openssl;
extern crate tokio;
extern crate tokio_openssl;

use failure::{err_msg, Error};
use futures::prelude::*;
use openssl::{
    pkey::PKey,
    ssl::{SslAcceptor, SslMethod},
    x509::X509,
};
use std::net::ToSocketAddrs;
use tokio::{net::TcpListener, runtime::Runtime};
use tokio_openssl::SslAcceptorExt;

use gotham::{bind_server, state::State};

const HELLO_WORLD: &str = "Hello World!";

pub fn say_hello(state: State) -> (State, &'static str) {
    (state, HELLO_WORLD)
}

/// Create an OpenSSL acceptor, then set up Gotham to use it.
pub fn main() -> Result<(), Error> {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at https://{}", addr);
    let acceptor = build_acceptor()?;

    let addr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| err_msg("Invalid Socket Address"))?;

    let listener = TcpListener::bind(&addr)?;

    let mut runtime = Runtime::new()?;

    let server = bind_server(
        listener,
        || Ok(say_hello),
        // NOTE: We're ignoring handshake errors here. You can modify to e.g. report them.
        move |socket| acceptor.accept_async(socket).map_err(|_| ()),
    );

    runtime
        .block_on(server)
        .map_err(|()| err_msg("Server failed"))
}

fn build_acceptor() -> Result<SslAcceptor, Error> {
    let cert = X509::from_pem(&include_bytes!("cert.pem")[..])?;
    let pkey = PKey::private_key_from_pem(&include_bytes!("key.pem")[..])?;

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
    builder.set_certificate(&cert)?;
    builder.set_private_key(&pkey)?;
    Ok(builder.build())
}
