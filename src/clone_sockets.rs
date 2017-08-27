use std::net::{SocketAddr, ToSocketAddrs, TcpListener};
use std::thread;
use std::sync::Arc;

use hyper::server::{Http, NewService};
use tokio_core;
use tokio_core::reactor::Core;
use futures::Stream;

use handler::{NewHandler, NewHandlerService};

/// Starts a Gotham application, with the given number of threads.
pub fn start_with_num_threads<NH, A>(addr: A, threads: usize, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let (listener, addr) = super::tcp_listener(addr);

    let protocol = Arc::new(Http::new());
    let service = NewHandlerService::new(new_handler);

    info!(
        target: "gotham::start",
        " Gotham listening on http://{} with {} threads",
        addr,
        threads,
    );

    for _ in 0..threads - 1 {
        let listener = listener.try_clone().expect("unable to clone TCP listener");
        let protocol = protocol.clone();
        let service = service.clone();
        thread::spawn(move || serve(listener, &addr, &protocol, &service));
    }

    serve(listener, &addr, &protocol, &service);
}

fn serve<NH>(
    listener: TcpListener,
    addr: &SocketAddr,
    protocol: &Http,
    new_service: &NewHandlerService<NH>,
) where
    NH: NewHandler + 'static,
{
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();

    let listener = tokio_core::net::TcpListener::from_listener(listener, addr, &handle)
        .expect("unable to convert TCP listener to tokio listener");

    core.run(listener.incoming().for_each(|(socket, addr)| {
        match new_service.new_service() {
            Ok(service) => protocol.bind_connection(&handle, socket, addr, service),
            Err(e) => error!(" unable to spawn service: {:?}", e),
        }
        Ok(())
    })).expect("unable to run reactor over listener");
}
