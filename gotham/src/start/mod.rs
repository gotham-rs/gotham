use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use handler::NewHandler;
use hyper::Chunk;
use hyper::server::Http;
use service::GothamService;

use futures::{Future, Stream};
use tokio;
use tokio::net::TcpListener;

/// Starts a Gotham application.
pub fn start<NH, A>(addr: A, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let (listener, addr) = tcp_listener(addr);
    let new_handler = Arc::new(new_handler);
    let gotham_service = GothamService::new(new_handler);
    let protocol = Arc::new(Http::<Chunk>::new());

    let server = listener.incoming()
        .map_err(|e| println!("error = {:?}", e))
        .for_each(move |socket| {
            let service = gotham_service.connect(addr);
            let f = protocol.serve_connection(socket, service).then(|_| Ok(()));

            tokio::spawn(f);

            Ok(())
        });

    tokio::run(server);
}

fn tcp_listener<A>(addr: A) -> (TcpListener, SocketAddr)
where
    A: ToSocketAddrs,
{
    let addr = match addr.to_socket_addrs().map(|ref mut i| i.next()) {
        Ok(Some(a)) => a,
        Ok(_) => panic!("unable to resolve listener address"),
        Err(_) => panic!("unable to parse listener address"),
    };

    let listener = TcpListener::bind(&addr).expect("unable to open TCP listener");

    (listener, addr)
}