use super::*;

use std::sync::{Arc, Weak, Mutex, PoisonError};
use std::time::{Instant, Duration};
use std::thread;

use linked_hash_map::LinkedHashMap;

pub trait NewBackend: Sync {
    type Instance: Backend + Send + 'static;

    fn new_backend(&self) -> io::Result<Self::Instance>;
}

pub type SessionFuture = Future<Item = Option<Vec<u8>>, Error = SessionError> + Send;

pub trait Backend: Send {
    fn random_identifier(&self) -> SessionIdentifier {
        let bytes: Vec<u8> = (0..64).map(|_| rand::random()).collect();
        SessionIdentifier { value: base64::encode_config(&bytes, base64::URL_SAFE_NO_PAD) }
    }

    fn persist_session(&self,
                       identifier: SessionIdentifier,
                       content: &[u8])
                       -> Result<(), SessionError>;

    fn read_session(&self, identifier: SessionIdentifier) -> Box<SessionFuture>;
}

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
    fn persist_session(&self,
                       identifier: SessionIdentifier,
                       content: &[u8])
                       -> Result<(), SessionError> {
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
            Ok(mut storage) => {
                match storage.remove(&identifier.value) {
                    Some((_old_instant, value)) => {
                        storage.insert(identifier.value, (Instant::now(), value.clone()));
                        future::ok(Some(value)).boxed()
                    }
                    None => future::ok(None).boxed(),
                }
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

fn cleanup_once(storage: &mut LinkedHashMap<String, (Instant, Vec<u8>)>,
                ttl: Duration)
                -> Option<Duration> {
    match storage.front() {
        Some((_, &(instant, _))) => {
            let age = instant.elapsed();

            if age >= ttl {
                storage.pop_front();
                None
            } else {
                Some(ttl - age)
            }
        }
        None => Some(ttl),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_test() {
        let mut storage = LinkedHashMap::new();

        storage.insert("abcd".to_owned(),
                       (Instant::now() - Duration::from_secs(3601), vec![]));

        cleanup_once(&mut storage, Duration::from_secs(3600));
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
        let identifier = new_backend.new_backend().unwrap().random_identifier();

        new_backend
            .new_backend()
            .expect("can't create backend for write")
            .persist_session(identifier.clone(), &bytes[..])
            .expect("failed to persist");

        let received = new_backend
            .new_backend()
            .expect("can't create backend for read")
            .read_session(identifier)
            .wait()
            .expect("no response from backend")
            .expect("session data missing");

        assert_eq!(bytes, received);
    }
}
