use std::io;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;
use std::sync::{Arc, Mutex};

use hyper::server::Http;
use tokio_core;
use tokio_core::net::TcpStream;
use tokio_core::reactor::{Core, Handle};
use futures::{future, task, Async, Future, Poll, Stream};

use handler::NewHandler;
use service::GothamService;

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

/// Starts a Gotham application, with the given number of threads.
///
/// ## Windows
///
/// An additional thread is used on Windows to accept connections.
pub fn start_with_num_threads<NH, A>(addr: A, threads: usize, new_handler: NH)
where
    NH: NewHandler + 'static,
    A: ToSocketAddrs,
{
    let (listener, addr) = ::tcp_listener(addr);

    let protocol = Arc::new(Http::new());
    let new_handler = Arc::new(new_handler);

    let queue = SocketQueue::new();

    {
        let queue = queue.clone();
        thread::spawn(move || start_listen_core(listener, addr, queue));
    }

    info!(
        target: "gotham::start",
        " Gotham listening on http://{} with {} threads",
        addr,
        threads,
    );

    for _ in 0..threads - 1 {
        let protocol = protocol.clone();
        let queue = queue.clone();
        let new_handler = new_handler.clone();
        thread::spawn(move || start_serve_core(queue, &protocol, new_handler));
    }

    start_serve_core(queue, &protocol, new_handler);
}

fn start_listen_core(listener: TcpListener, addr: SocketAddr, queue: SocketQueue) {
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();
    core.run(listen(listener, addr, queue, &handle))
        .expect("unable to run reactor over listener");
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

fn start_serve_core<NH>(queue: SocketQueue, protocol: &Http, new_handler: Arc<NH>)
where
    NH: NewHandler + 'static,
{
    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();
    core.run(serve(queue, protocol, new_handler, &handle))
        .expect("unable to run reactor for work stealing");
}

fn serve<'a, NH>(
    queue: SocketQueue,
    protocol: &'a Http,
    new_handler: Arc<NH>,
    handle: &'a Handle,
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
                let f = protocol.serve_connection(socket, service).then(|_| Ok(()));

                handle.spawn(f);
                Ok(())
            })
        }),
    )
}
