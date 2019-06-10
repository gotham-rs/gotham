//! A Hello World example application for working with Gotham.

extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate tokio_rustls;

use std::io::BufReader;
use tokio_rustls::{
    rustls::{
        self,
        internal::pemfile::{certs, pkcs8_private_keys},
        NoClientAuth,
    },
};

use gotham::state::State;

const HELLO_WORLD: &str = "Hello World!";

/// Create a `Handler` which is invoked when responding to a `Request`.
///
/// How does a function become a `Handler`?.
/// We've simply implemented the `Handler` trait, for functions that match the signature used here,
/// within Gotham itself.
pub fn say_hello(state: State) -> (State, &'static str) {
    (state, HELLO_WORLD)
}

/// Start a server and call the `Handler` we've defined above for each `Request` we receive.
pub fn main() -> Result<(), rustls::TLSError> {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start_with_tls(addr, || Ok(say_hello), build_config()?);
    Ok(())
}

fn build_config() -> Result<rustls::ServerConfig, rustls::TLSError> {
    let mut cfg = rustls::ServerConfig::new(NoClientAuth::new());
    let mut cert_file = BufReader::new(&include_bytes!("cert.pem")[..]);
    let mut key_file = BufReader::new(&include_bytes!("key.pem")[..]);
    let certs = certs(&mut cert_file).unwrap();
    let mut keys = pkcs8_private_keys(&mut key_file).unwrap();
    cfg.set_single_cert(certs, keys.remove(0))?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;
    use hyper::StatusCode;

    #[test]
    fn receive_hello_world_response() {
        let test_server = TestServer::new(|| Ok(say_hello)).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"Hello World!");
    }
}
