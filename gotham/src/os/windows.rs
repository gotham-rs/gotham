use std::net::{SocketAddr, TcpListener};
use std::thread;
use std::io;
use std::sync::{Arc, Mutex};

use tokio_core::net::{self, TcpStream};
use tokio_core::reactor::{Core, Handle};
use futures::{future, task, Async, Poll, Stream};

use crossbeam::sync::SegQueue;

#[derive(Clone)]
pub struct SocketQueue {
    addr: SocketAddr,
    queue: Arc<SegQueue<(TcpStream, SocketAddr)>>,
    notify: Arc<Mutex<Vec<task::Task>>>,
}

impl SocketQueue {
    fn new(addr: SocketAddr) -> SocketQueue {
        let queue = Arc::new(SegQueue::new());
        let notify = Arc::new(Mutex::new(Vec::new()));
        SocketQueue {
            addr,
            queue,
            notify,
        }
    }
}

impl SocketQueue {
    fn listen(self) {
        let mut n: usize = 0;

        let mut core = Core::new().expect("unable to spawn tokio reactor");
        let handle = core.handle();

        let tcp = TcpListener::bind(self.addr).expect("unable to open TCP listener");
        let listener = net::TcpListener::from_listener(tcp, &self.addr, &handle)
            .expect("unable to convert TCP listener to tokio listener");

        core.run(listener.incoming().for_each(|conn| {
            self.queue.push(conn);
            let tasks = self.notify
                .lock()
                .expect("mutex poisoned, futures::task::Task::notify panicked?");

            n = (n + 1) % tasks.len();
            tasks[n].notify();
            Ok(())
        })).expect("unable to run reactor over listener");
    }
}

impl ::GothamListener for SocketQueue {
    type Stream = SocketStream;

    fn incoming(self, handle: Handle) -> Self::Stream {
        let tasks_m = self.notify.clone();

        handle.spawn(future::lazy(move || {
            let mut tasks = tasks_m
                .lock()
                .expect("mutex poisoned, futures::task::Task::notify panicked?");
            tasks.push(task::current());
            future::ok(())
        }));

        SocketStream {
            queue: self.queue.clone(),
        }
    }
}

pub struct SocketStream {
    queue: Arc<SegQueue<(TcpStream, SocketAddr)>>,
}

impl Stream for SocketStream {
    type Item = (TcpStream, SocketAddr);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.queue.try_pop() {
            Some(t) => Ok(Async::Ready(Some(t))),
            None => Ok(Async::NotReady),
        }
    }
}

/// Constructs a GothamListener to handle incoming TCP connections.
///
/// Note: On Windows this function spawns an extra thread to handle
/// accepting connections.
pub fn new_gotham_listener(addr: SocketAddr) -> SocketQueue {
    let queue = SocketQueue::new(addr);
    {
        let queue = queue.clone();
        thread::spawn(move || queue.listen());
    }
    queue
}
