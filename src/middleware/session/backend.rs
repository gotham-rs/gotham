use super::*;

use std::thread::{spawn, JoinHandle};
use std::collections::HashMap;
use std::sync::{mpsc, Mutex, Arc, RwLock};

use futures::sync::oneshot;

pub struct NewMemoryBackend {
    storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

pub struct MemoryBackend {
    storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl NewMemoryBackend {
    pub fn new(_bound: usize) -> NewMemoryBackend {
        NewMemoryBackend { storage: Arc::new(RwLock::new(HashMap::new())) }
    }
}

impl Default for NewMemoryBackend {
    fn default() -> NewMemoryBackend {
        NewMemoryBackend::new(100)
    }
}

impl NewBackend for NewMemoryBackend {
    type Instance = MemoryBackend;

    fn new_backend(&self) -> io::Result<Self::Instance> {
        Ok(MemoryBackend { storage: self.storage.clone() })
    }
}

impl Backend for MemoryBackend {
    fn new_session(&self, content: &[u8]) -> Result<SessionIdentifier, SessionError> {
        unimplemented!()
    }

    fn update_session(&self,
                      identifier: SessionIdentifier,
                      content: &[u8])
                      -> Result<(), SessionError> {
        let mut storage = self.storage.write().unwrap();
        storage.insert(identifier.value, Vec::from(content));
        Ok(())
    }

    fn read_session(&self, identifier: SessionIdentifier) -> Box<SessionFuture> {
        let storage = self.storage.read().unwrap();
        future::ok(storage.get(&identifier.value).map(Clone::clone)).boxed()
    }
}
