use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;
use std::sync::{Arc, Mutex};

use hyper::server::Http;
use tokio_core;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;
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

    fn for_each<F, U>(self, f: F) -> stream::for_each::ForEach<Self, F, U>
    where
        F: FnMut(Self::Item) -> U,
        U: IntoFuture<Item = (), Error = Self::Error>,
        Self: Sized,
    {
        let tasks_m = self.notify.clone();

        future::join_all(vec![
            future::lazy(move || {
                let mut tasks = tasks_m
                    .lock()
                    .expect("mutex poisoned, futures::task::Task::notify panicked?");
                tasks.push(task::current());
                future::ok(())
            }),
            for_each::new(self, f),
        ])
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
    let addr = ::pick_addr(addr);
    let tcp = ::tcp_listener(addr);

    let protocol = Arc::new(Http::new());
    let new_handler = Arc::new(new_handler);

    let queue = SocketQueue::new();

    let listener = new_gotham_listener(tcp, addr)

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
        thread::spawn(move || serve(queue, &protocol, new_handler));
    }

    serve(queue, &protocol, new_handler);
}

fn new_gotham_listener(tcp: TcpListener, addr: SocketAddr) -> SocketQueue {
    let queue = SocketQueue::new();
    {
        let queue = queue.clone();
        thread::spawn(move || listen(tcp, addr, queue));
    }
    queue
}

fn listen(listener: TcpListener, addr: SocketAddr, queue: SocketQueue) {
    let mut n: usize = 0;

    let mut core = Core::new().expect("unable to spawn tokio reactor");
    let handle = core.handle();

    let listener = tokio_core::net::TcpListener::from_listener(listener, &addr, &handle)
        .expect("unable to convert TCP listener to tokio listener");

    core.run(listener.incoming().for_each(|conn| {
        queue.queue.push(conn);
        let tasks = queue
            .notify
            .lock()
            .expect("mutex poisoned, futures::task::Task::notify panicked?");

        n = (n + 1) % tasks.len();
        tasks[n].notify();
        Ok(())
    })).expect("unable to run reactor over listener");
}
