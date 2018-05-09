use std::io;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;
use std::sync::Arc;
use std::time::Duration;
use std::fmt::Debug;

use hyper::server::Http;
use tokio_core;
use tokio_core::reactor::{Core, Handle};
use futures::{future, Future, Stream};
use futures::sync::oneshot;

use handler::NewHandler;
use service::GothamService;
use os::{join_threads, maybe_wait_for_remaining_connections, propagate_shutdown_signal,
         run_until_shutdown, WaitUntilZeroConnections};

/// Starts a Gotham application, with the given number of threads and ability to gracefully shut down.
///
/// This function blocks current thread until `shutdown_signal` resolved or panic occur.
///
/// When `shutdown_timeout` is not equal to `Duration::default()`, function waits for remaining open connections to
/// finish for specified time.
pub fn run_with_num_threads_until<NH, A, F>(
    addr: A,
    threads: usize,
    new_handler: NH,
    shutdown_signal: F,
    shutdown_timeout: Duration,
) where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
    F: Future<Item = (), Error = ()>,
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

    let mut threads_shutdown_tx = Vec::new();
    let mut threads_handles = Vec::new();
    for thread_n in 0..threads - 1 {
        let listener = listener.try_clone().expect("unable to clone TCP listener");
        let protocol = protocol.clone();
        let new_handler = new_handler.clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        threads_shutdown_tx.push(shutdown_tx);
        threads_handles.push(
            thread::Builder::new()
                .name(format!("gotham-{}", thread_n))
                .spawn(move || {
                    start_core(
                        listener,
                        &addr,
                        &protocol,
                        new_handler,
                        shutdown_rx,
                        shutdown_timeout,
                    )
                })
                .expect("unable to spawn thread"),
        );
    }

    let shutdown_signal = propagate_shutdown_signal(shutdown_signal, threads_shutdown_tx);
    start_core(
        listener,
        &addr,
        &protocol,
        new_handler,
        shutdown_signal,
        shutdown_timeout,
    );
    join_threads(threads_handles);
}

/// Starts a Gotham application, with the given number of threads.
///
/// This function never return but may panic because of errors.
pub fn start_with_num_threads<NH, A>(addr: A, threads: usize, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    run_with_num_threads_until(
        addr,
        threads,
        new_handler,
        future::empty(),
        Duration::default(),
    )
}

fn start_core<NH, E, F>(
    listener: TcpListener,
    addr: &SocketAddr,
    protocol: &Http,
    new_handler: Arc<NH>,
    shutdown_signal: F,
    shutdown_timeout: Duration,
) where
    NH: NewHandler + 'static,
    E: Debug,
    F: Future<Item = (), Error = E>,
{
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();

    let wait_until_zero_connections = WaitUntilZeroConnections::new();

    let srv = serve(
        listener,
        addr,
        protocol,
        new_handler,
        &handle,
        &wait_until_zero_connections,
    );
    run_until_shutdown(
        &mut core,
        srv,
        shutdown_signal,
        "unable to run reactor over listener",
    );
    maybe_wait_for_remaining_connections(core, &wait_until_zero_connections, shutdown_timeout);
}

fn serve<'a, NH>(
    listener: TcpListener,
    addr: &SocketAddr,
    protocol: &'a Http,
    new_handler: Arc<NH>,
    handle: &'a Handle,
    wait_until_zero_connections: &'a WaitUntilZeroConnections,
) -> Box<Future<Item = (), Error = io::Error> + 'a>
where
    NH: NewHandler + 'static,
{
    let gotham_service = GothamService::new(new_handler, handle.clone());

    let listener = tokio_core::net::TcpListener::from_listener(listener, addr, handle)
        .expect("unable to convert TCP listener to tokio listener");

    Box::new(listener.incoming().for_each(move |(socket, addr)| {
        let service = gotham_service.connect(addr);
        let f = protocol
            .serve_connection(
                socket,
                wait_until_zero_connections.count_connection(service),
            )
            .then(|_| Ok(()));

        handle.spawn(f);
        Ok(())
    }))
}
