use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::{Duration, Instant};
use std::{io, thread};

use futures::future;
use linked_hash_map::LinkedHashMap;

use middleware::session::backend::{Backend, NewBackend, SessionFuture};
use middleware::session::{SessionError, SessionIdentifier};

/// Defines the in-process memory based session storage.
///
/// This is the default implementation which is used by `NewSessionMiddleware::default()`
#[derive(Clone)]
pub struct MemoryBackend {
    // Intuitively, a global `Mutex<_>` sounded like the slowest option. However, in some
    // benchmarking it proved to be the faster out of the options that were tried:
    //
    // 1. Background thread containing all data, acting as an internal "server" for session data,
    //    passing messages via `std::sync::mpsc::sync_channel`;
    // 2. Background thread maintaining only LRU data for each session ID, and purging them when
    //    they exceed the TTL, passing messages via a `std::sync::mpsc::sync_channel`;
    // 3. The same options, but with messages being passed via `crossbeam::sync::MsQueue`;
    // 4. Naive, global mutex.
    //
    // The performance was about 10~15% higher with the naive implementation, when measured in a
    // similarly naive benchmark using `wrk` and a lightweight sample app. Real-world use cases
    // might show a need to replace this with a smarter implementation, but today there's very
    // little overhead here.
    storage: Arc<Mutex<LinkedHashMap<String, (Instant, Vec<u8>)>>>,
}

impl MemoryBackend {
    /// Creates a new `MemoryBackend` where sessions expire and are removed after the `ttl` has
    /// elapsed.
    ///
    /// Alternately, `MemoryBackend::default()` creates a `MemoryBackend` with a `ttl` of one hour.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # use std::time::Duration;
    /// # use gotham::middleware::session::{MemoryBackend, NewSessionMiddleware};
    /// # fn main() {
    /// NewSessionMiddleware::new(MemoryBackend::new(Duration::from_secs(3600)))
    /// # ;}
    /// ```
    pub fn new(ttl: Duration) -> MemoryBackend {
        let storage = Arc::new(Mutex::new(LinkedHashMap::new()));

        {
            let storage = Arc::downgrade(&storage);
            thread::spawn(move || cleanup_loop(storage, ttl));
        }

        MemoryBackend { storage }
    }
}

impl Default for MemoryBackend {
    fn default() -> MemoryBackend {
        MemoryBackend::new(Duration::from_secs(3600))
    }
}

impl NewBackend for MemoryBackend {
    type Instance = MemoryBackend;

    fn new_backend(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl Backend for MemoryBackend {
    fn persist_session(
        &self,
        identifier: SessionIdentifier,
        content: &[u8],
    ) -> Result<(), SessionError> {
        match self.storage.lock() {
            Ok(mut storage) => {
                storage.insert(identifier.value, (Instant::now(), Vec::from(content)));
                Ok(())
            }
            Err(PoisonError { .. }) => {
                unreachable!("session memory backend lock poisoned, HashMap panicked?")
            }
        }
    }

    fn read_session(&self, identifier: SessionIdentifier) -> Box<SessionFuture> {
        match self.storage.lock() {
            Ok(mut storage) => match storage.get_refresh(&identifier.value) {
                Some(&mut (ref mut instant, ref value)) => {
                    *instant = Instant::now();
                    Box::new(future::ok(Some(value.clone())))
                }
                None => Box::new(future::ok(None)),
            },
            Err(PoisonError { .. }) => {
                unreachable!("session memory backend lock poisoned, HashMap panicked?")
            }
        }
    }

    fn drop_session(&self, identifier: SessionIdentifier) -> Result<(), SessionError> {
        match self.storage.lock() {
            Ok(mut storage) => {
                storage.remove(&identifier.value);
                Ok(())
            }
            Err(PoisonError { .. }) => {
                unreachable!("session memory backend lock poisoned, HashMap panicked?")
            }
        }
    }
}

fn cleanup_loop(storage: Weak<Mutex<LinkedHashMap<String, (Instant, Vec<u8>)>>>, ttl: Duration) {
    loop {
        // If the original `Arc<_>` goes away, we don't need to keep sweeping the cache, because
        // it's gone too. We can bail out of this thread when the weak ref fails to upgrade.
        let storage = match storage.upgrade() {
            None => break,
            Some(storage) => storage,
        };

        let duration = match storage.lock() {
            Err(PoisonError { .. }) => break,
            Ok(mut storage) => cleanup_once(&mut storage, ttl),
        };

        if let Some(duration) = duration {
            thread::sleep(duration);
        }
    }
}

fn cleanup_once(
    storage: &mut LinkedHashMap<String, (Instant, Vec<u8>)>,
    ttl: Duration,
) -> Option<Duration> {
    match storage.front() {
        Some((_, &(instant, _))) => {
            let age = instant.elapsed();

            if age >= ttl {
                if let Some((key, _)) = storage.pop_front() {
                    trace!(" expired session {} and removed from MemoryBackend", key);
                }

                // We just removed one, so skip the sleep and check the next entry
                None
            } else {
                // Ensure to shrink the storage after a spike in sessions.
                //
                // Even with this, memory usage won't always drop back to pre-spike levels because
                // the OS can hang onto it.
                //
                // The arbitrary numbers here were chosen to avoid the resizes being extremely
                // frequent. Powers of 2 seemed like a reasonable idea, to let the optimiser
                // potentially shave off a few CPU cycles. Totally unscientific though.
                let cap = storage.capacity();
                let len = storage.len();

                if cap >= 65536 && cap / 8 > len {
                    storage.shrink_to_fit();

                    trace!(
                        " session backend had capacity {} and {} sessions, new capacity: {}",
                        cap,
                        len,
                        storage.capacity()
                    );
                }

                // Sleep until the next entry expires, but for at least 1 second
                Some(::std::cmp::max(ttl - age, Duration::from_secs(1)))
            }
        }
        // No sessions; sleep for the TTL, because that's the soonest we'll need to expire anything
        None => Some(ttl),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::Future;
    use rand;

    #[test]
    fn cleanup_test() {
        let mut storage = LinkedHashMap::new();

        storage.insert(
            "abcd".to_owned(),
            (Instant::now() - Duration::from_secs(2), vec![]),
        );

        cleanup_once(&mut storage, Duration::from_secs(1));
        assert!(storage.is_empty());
    }

    #[test]
    fn cleanup_join_test() {
        let storage = Arc::new(Mutex::new(LinkedHashMap::new()));
        let weak = Arc::downgrade(&storage);

        let handle = thread::spawn(move || cleanup_loop(weak, Duration::from_millis(1)));

        drop(storage);
        handle.join().unwrap();
    }

    #[test]
    fn memory_backend_test() {
        let new_backend = MemoryBackend::new(Duration::from_millis(100));
        let bytes: Vec<u8> = (0..64).map(|_| rand::random()).collect();
        let identifier = SessionIdentifier {
            value: "totally_random_identifier".to_owned(),
        };

        new_backend
            .new_backend()
            .expect("can't create backend for write")
            .persist_session(identifier.clone(), &bytes[..])
            .expect("failed to persist");

        let received = new_backend
            .new_backend()
            .expect("can't create backend for read")
            .read_session(identifier.clone())
            .wait()
            .expect("no response from backend")
            .expect("session data missing");

        assert_eq!(bytes, received);
    }

    #[test]
    fn memory_backend_refresh_test() {
        let new_backend = MemoryBackend::new(Duration::from_millis(100));
        let bytes: Vec<u8> = (0..64).map(|_| rand::random()).collect();
        let identifier = SessionIdentifier {
            value: "totally_random_identifier".to_owned(),
        };
        let bytes2: Vec<u8> = (0..64).map(|_| rand::random()).collect();
        let identifier2 = SessionIdentifier {
            value: "another_totally_random_identifier".to_owned(),
        };

        let backend = new_backend
            .new_backend()
            .expect("can't create backend for write");

        backend
            .persist_session(identifier.clone(), &bytes[..])
            .expect("failed to persist");

        backend
            .persist_session(identifier2.clone(), &bytes2[..])
            .expect("failed to persist");

        {
            let mut storage = backend.storage.lock().expect("couldn't lock storage");
            assert_eq!(
                storage.front().expect("no front element").0,
                &identifier.value
            );

            assert_eq!(
                storage.back().expect("no back element").0,
                &identifier2.value
            );
        }

        backend
            .read_session(identifier.clone())
            .wait()
            .expect("failed to read session");

        {
            // Identifiers have swapped
            let mut storage = backend.storage.lock().expect("couldn't lock storage");
            assert_eq!(
                storage.front().expect("no front element").0,
                &identifier2.value
            );

            assert_eq!(
                storage.back().expect("no back element").0,
                &identifier.value
            );
        }
    }
}
