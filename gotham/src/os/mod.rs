#[cfg(not(windows))]
pub mod unix;
#[cfg(not(windows))]
pub use self::unix as current;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use self::windows as current;

use std::cell::RefCell;
use std::fmt::Debug;
use std::io;
use std::rc::{Rc, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use futures::{task, Async, Future, Poll};
use futures::sync::oneshot;
use hyper::server::Service;
use tokio_core::reactor::{Core, Timeout};

pub(crate) struct ConnectionCounter {
    connections: usize,
    blocker: Option<task::Task>,
}

impl ConnectionCounter {
    fn new() -> Self {
        Self {
            connections: 0,
            blocker: None,
        }
    }
}

pub(crate) struct CountedConnection<S> {
    counter: Weak<RefCell<ConnectionCounter>>,
    service: S,
}

impl<S> CountedConnection<S> {
    fn new(counter: &Rc<RefCell<ConnectionCounter>>, service: S) -> Self {
        {
            let mut counter = counter.borrow_mut();
            counter.connections += 1;
            trace!(
                "Connection created ({} connections total)",
                counter.connections
            );
        }
        Self {
            counter: Rc::downgrade(counter),
            service,
        }
    }
}

impl<S> Drop for CountedConnection<S> {
    fn drop(&mut self) {
        let counter = match self.counter.upgrade() {
            Some(counter) => counter,
            None => return,
        };
        let mut counter = counter.borrow_mut();
        counter.connections -= 1;
        trace!(
            "Connection destroyed ({} connections left)",
            counter.connections
        );
        if counter.connections == 0 {
            if let Some(task) = counter.blocker.take() {
                task.notify();
            }
        }
    }
}

impl<S> Service for CountedConnection<S>
where
    S: Service,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, req: Self::Request) -> Self::Future {
        self.service.call(req)
    }
}

#[derive(Clone)]
pub(crate) struct WaitUntilZeroConnections(Rc<RefCell<ConnectionCounter>>);

impl WaitUntilZeroConnections {
    pub(crate) fn new() -> Self {
        WaitUntilZeroConnections(Rc::new(RefCell::new(ConnectionCounter::new())))
    }

    pub(crate) fn count_connection<S>(&self, service: S) -> CountedConnection<S> {
        CountedConnection::new(&self.0, service)
    }

    pub(crate) fn num_connections(&self) -> usize {
        self.0.borrow().connections
    }
}

impl Future for WaitUntilZeroConnections {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        let mut counter = self.0.borrow_mut();
        if counter.connections == 0 {
            Ok(().into())
        } else {
            counter.blocker = Some(task::current());
            Ok(Async::NotReady)
        }
    }
}

pub(crate) fn propagate_shutdown_signal<'a, F>(
    shutdown_signal: F,
    threads_shutdown_tx: Vec<oneshot::Sender<()>>,
) -> Box<'a + Future<Item = (), Error = io::Error>>
where
    F: 'a + Future<Item = (), Error = ()>,
{
    // Accept either value (both Ok or Err) as a valid signal.
    Box::new(shutdown_signal.then(|_| {
        threads_shutdown_tx.into_iter().for_each(|shutdown_tx| {
            shutdown_tx.send(()).unwrap_or_else(|_| {
                warn!("Receiver of shutdown signal for child thread was dropped")
            });
        });
        Ok(())
    }))
}

pub(crate) fn run_until_shutdown<E, F, SE, SF>(
    core: &mut Core,
    srv: F,
    shutdown_signal: SF,
    expect_msg: &str,
) where
    E: Debug,
    F: Future<Item = (), Error = E>,
    SE: Debug,
    SF: Future<Item = (), Error = SE>,
{
    let shutdown_signal =
        shutdown_signal.map_err(|_| panic!("Sender of shutdown signal was dropped"));
    match core.run(shutdown_signal.select(srv)) {
        Ok(((), _)) => Ok(()),
        Err((error, _)) => Err(error),
    }.expect(expect_msg);
}

pub(crate) fn maybe_wait_for_remaining_connections(
    mut core: Core,
    wait_until_zero_connections: &WaitUntilZeroConnections,
    shutdown_timeout: Duration,
) {
    if shutdown_timeout == Duration::default() {
        debug!(
            target: "gotham::start",
            "Shutting down immediately, without waiting for remaining connections \
            (connections left: {})",
            wait_until_zero_connections.num_connections(),
        );
    } else {
        let handle = core.handle();

        // Wait for remaining active connections.
        debug!(
            target: "gotham::start",
            "Shutting down and waiting for remaining active connections \
            (connections left: {}, timeout: {:?})",
            wait_until_zero_connections.num_connections(),
            shutdown_timeout,
        );

        let timeout = Timeout::new(shutdown_timeout, &handle).expect("unable to set the timeout");
        match core.run(wait_until_zero_connections.clone().select(timeout)) {
            Ok(_) => Ok(()),
            Err((error, _)) => Err(error),
        }.expect("unable to wait for remaining active connections");

        debug!(
            target: "gotham::start",
            "Done waiting for remaining active connections (connections left: {})",
            wait_until_zero_connections.num_connections(),
        );
    }
}

pub(crate) fn join_threads<T>(threads_handles: Vec<JoinHandle<T>>) {
    threads_handles.into_iter().for_each(|thread| {
        thread
            .join()
            .map(|_| ())
            .unwrap_or_else(|error| warn!("Unable to join child thread: {:?}", error))
    });
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::thread;
    use std::time::Duration;

    use futures::Future;
    use futures::sync::oneshot;
    use hyper::{Client, StatusCode};
    use tokio_core::reactor::Core;

    use handler::{Handler, HandlerFuture, NewHandler};
    use state::State;
    use os::current;

    #[derive(Copy, Clone)]
    struct DummyHandler;

    impl NewHandler for DummyHandler {
        type Instance = DummyHandler;

        fn new_handler(&self) -> io::Result<Self::Instance> {
            Ok(DummyHandler)
        }
    }

    impl Handler for DummyHandler {
        fn handle(self, _state: State) -> Box<HandlerFuture> {
            unimplemented!()
        }
    }

    fn try_request(addr: &str) -> bool {
        let mut core = Core::new().unwrap();
        let client = Client::new(&core.handle());

        let uri = format!("http://{}/", addr);
        let uri_parsed = uri.parse().unwrap();
        let work = client.get(uri_parsed).map(|res| {
            assert_eq!(res.status(), StatusCode::InternalServerError);
        });

        match core.run(work) {
            Ok(_) => true,

            Err(error) => {
                eprintln!("Unable to get \"{}\": {}", uri, error);
                false
            }
        }
    }

    fn run_with_num_threads_until(addr: &'static str, shutdown_timeout: Duration) {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let handle = thread::spawn(move || {
            current::run_with_num_threads_until(
                addr,
                2, // Use two threads to test join().
                DummyHandler,
                shutdown_rx
                    .map_err(|error| panic!("Sender of shutdown signal was dropped ({})", error)),
                shutdown_timeout,
            )
        });

        // Wait until server will be able to answer.
        let mut max_retries = 25;
        while (max_retries != 0) && !try_request(addr) {
            max_retries -= 1;
            thread::sleep(Duration::from_millis(200));
        }
        assert_ne!(max_retries, 0);

        shutdown_tx.send(()).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn run_with_num_threads_until_without_timeout() {
        run_with_num_threads_until("127.0.0.1:12345", Duration::default());
    }

    #[test]
    fn run_with_num_threads_until_with_timeout() {
        run_with_num_threads_until("127.0.0.1:54321", Duration::from_secs(5));
    }
}
