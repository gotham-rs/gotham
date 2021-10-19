//! A Hello World example application for working with Gotham.
use gotham::anyhow;
use gotham::rustls::{self, Certificate, PrivateKey, ServerConfig};
use gotham::state::State;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::io::BufReader;

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
pub fn main() -> anyhow::Result<()> {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at https://{}", addr);
    gotham::start_with_tls(addr, || Ok(say_hello), build_config()?)?;
    Ok(())
}

fn build_config() -> Result<ServerConfig, rustls::Error> {
    let mut cert_file = BufReader::new(&include_bytes!("cert.pem")[..]);
    let mut key_file = BufReader::new(&include_bytes!("key.pem")[..]);
    let certs = certs(&mut cert_file)
        .unwrap()
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys = pkcs8_private_keys(&mut key_file).unwrap();
    ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, PrivateKey(keys.remove(0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::hyper::StatusCode;
    use gotham::test::TestServer;

    #[test]
    fn receive_hello_world_response() {
        let test_server = TestServer::new(|| Ok(say_hello)).unwrap();
        let response = test_server
            .client()
            .get("https://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"Hello World!");
    }
}
