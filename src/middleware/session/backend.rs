use super::*;

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

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
    storage: Arc<Mutex<LinkedHashMap<String, (Instant, Vec<u8>)>>>,
}

impl Default for MemoryBackend {
    fn default() -> MemoryBackend {
        MemoryBackend { storage: Arc::new(Mutex::new(LinkedHashMap::new())) }
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
