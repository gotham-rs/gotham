use std::io;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;
use std::sync::Arc;

use hyper::server::Http;
use tokio_core;
use tokio_core::reactor::{Core, Handle};
use futures::{Future, Stream};

use handler::NewHandler;
use service::GothamService;

/// Starts a Gotham application, with the given number of threads.
pub fn start_with_num_threads<NH, A>(addr: A, threads: usize, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let (listener, addr) = ::tcp_listener(addr);

    let protocol = Arc::new(Http::new());
    let new_handler = Arc::new(new_handler);

    info!(
        target: "gotham::start",
        " Gotham listening on http://{} with {} threads",
        addr,
        threads,
    );

    for _ in 0..threads - 1 {
        let listener = listener.try_clone().expect("unable to clone TCP listener");
        let protocol = protocol.clone();
        let new_handler = new_handler.clone();
        thread::spawn(move || start_core(listener, &addr, &protocol, new_handler));
    }

    start_core(listener, &addr, &protocol, new_handler);
}

fn start_core<NH>(listener: TcpListener, addr: &SocketAddr, protocol: &Http, new_handler: Arc<NH>)
where
    NH: NewHandler + 'static,
{
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();
    core.run(serve(listener, addr, protocol, new_handler, &handle))
        .expect("unable to run reactor over listener");
}

fn serve<'a, NH>(
    listener: TcpListener,
    addr: &SocketAddr,
    protocol: &'a Http,
    new_handler: Arc<NH>,
    handle: &'a Handle,
) -> Box<Future<Item = (), Error = io::Error> + 'a>
where
    NH: NewHandler + 'static,
{
    let gotham_service = GothamService::new(new_handler, handle.clone());

    let listener = tokio_core::net::TcpListener::from_listener(listener, addr, handle)
        .expect("unable to convert TCP listener to tokio listener");

    Box::new(listener.incoming().for_each(move |(socket, addr)| {
        let service = gotham_service.connect(addr);
        let f = protocol.serve_connection(socket, service).then(|_| Ok(()));

        handle.spawn(f);
        Ok(())
    }))
}
