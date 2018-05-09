use std::io;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::fmt::Debug;

use hyper::server::Http;
use tokio_core;
use tokio_core::net::TcpStream;
use tokio_core::reactor::{Core, Handle};
use futures::{future, task, Async, Future, Poll, Stream};
use futures::sync::oneshot;

use handler::NewHandler;
use service::GothamService;
use os::{join_threads, maybe_wait_for_remaining_connections, propagate_shutdown_signal,
         run_until_shutdown, WaitUntilZeroConnections};

use crossbeam::sync::SegQueue;

#[derive(Clone)]
struct SocketQueue {
    queue: Arc<SegQueue<(TcpStream, SocketAddr)>>,
    notify: Arc<Mutex<Vec<task::Task>>>,
}

impl SocketQueue {
    fn new() -> SocketQueue {
        let queue = Arc::new(SegQueue::new());
        let notify = Arc::new(Mutex::new(Vec::new()));
        SocketQueue { queue, notify }
    }
}

impl Stream for SocketQueue {
    type Item = (TcpStream, SocketAddr);
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.queue.try_pop() {
            Some(t) => Ok(Async::Ready(Some(t))),
            None => Ok(Async::NotReady),
        }
    }
}

/// Starts a Gotham application, with the given number of threads and ability to gracefully shut down.
///
/// This function blocks current thread until `shutdown_signal` resolved or panic occur.
///
/// When `shutdown_timeout` is not equal to `Duration::default()`, function waits for remaining open connections to
/// finish for specified time.
///
/// ## Windows
///
/// An additional thread is used on Windows to accept connections.
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

    let queue = SocketQueue::new();

    let mut threads_shutdown_tx = Vec::new();
    let mut threads_handles = Vec::new();

    {
        let queue = queue.clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        threads_shutdown_tx.push(shutdown_tx);
        threads_handles.push(
            thread::Builder::new()
                .name("gotham-listener".into())
                .spawn(move || start_listen_core(listener, addr, queue, shutdown_rx))
                .expect("unable to spawn thread"),
        );
    }

    info!(
        target: "gotham::start",
        " Gotham listening on http://{} with {} threads",
        addr,
        threads,
    );

    for thread_n in 0..threads - 1 {
        let protocol = protocol.clone();
        let queue = queue.clone();
        let new_handler = new_handler.clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        threads_shutdown_tx.push(shutdown_tx);
        threads_handles.push(
            thread::Builder::new()
                .name(format!("gotham-{}", thread_n))
                .spawn(move || {
                    start_serve_core(queue, &protocol, new_handler, shutdown_rx, shutdown_timeout)
                })
                .expect("unable to spawn thread"),
        );
    }

    let shutdown_signal = propagate_shutdown_signal(shutdown_signal, threads_shutdown_tx);
    start_serve_core(
        queue,
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
///
/// ## Windows
///
/// An additional thread is used on Windows to accept connections.
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

fn start_listen_core<E, F>(
    listener: TcpListener,
    addr: SocketAddr,
    queue: SocketQueue,
    shutdown_signal: F,
) where
    E: Debug,
    F: Future<Item = (), Error = E>,
{
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();
    let srv = listen(listener, addr, queue, &handle);
    run_until_shutdown(
        &mut core,
        srv,
        shutdown_signal,
        "unable to run reactor over listener",
    );
}

fn listen(
    listener: TcpListener,
    addr: SocketAddr,
    queue: SocketQueue,
    handle: &Handle,
) -> Box<Future<Item = (), Error = io::Error>> {
    let listener = tokio_core::net::TcpListener::from_listener(listener, &addr, handle)
        .expect("unable to convert TCP listener to tokio listener");

    let mut n: usize = 0;

    Box::new(listener.incoming().for_each(move |conn| {
        queue.queue.push(conn);
        let tasks = queue
            .notify
            .lock()
            .expect("mutex poisoned, futures::task::Task::notify panicked?");

        n = (n + 1) % tasks.len();
        tasks[n].notify();
        Ok(())
    }))
}

fn start_serve_core<NH, E, F>(
    queue: SocketQueue,
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
        queue,
        protocol,
        new_handler,
        &handle,
        &wait_until_zero_connections,
    );
    run_until_shutdown(
        &mut core,
        srv,
        shutdown_signal,
        "unable to run reactor for work stealing",
    );
    maybe_wait_for_remaining_connections(core, &wait_until_zero_connections, shutdown_timeout);
}

fn serve<'a, NH>(
    queue: SocketQueue,
    protocol: &'a Http,
    new_handler: Arc<NH>,
    handle: &'a Handle,
    wait_until_zero_connections: &'a WaitUntilZeroConnections,
) -> Box<Future<Item = (), Error = ()> + 'a>
where
    NH: NewHandler + 'static,
{
    let gotham_service = GothamService::new(new_handler, handle.clone());
    let tasks_m = queue.notify.clone();

    Box::new(
        future::lazy(move || {
            let mut tasks = tasks_m
                .lock()
                .expect("mutex poisoned, futures::task::Task::notify panicked?");
            tasks.push(task::current());
            future::ok(())
        }).and_then(move |_| {
            queue.for_each(move |(socket, addr)| {
                let service = gotham_service.connect(addr);
                let f = protocol
                    .serve_connection(
                        socket,
                        wait_until_zero_connections.count_connection(service),
                    )
                    .then(|_| Ok(()));

                handle.spawn(f);
                Ok(())
            })
        }),
    )
}
