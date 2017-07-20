use super::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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

    fn new_session(&self, content: &[u8]) -> Result<SessionIdentifier, SessionError>;
    fn update_session(&self,
                      identifier: SessionIdentifier,
                      content: &[u8])
                      -> Result<(), SessionError>;
    fn read_session(&self, identifier: SessionIdentifier) -> Box<SessionFuture>;
}

#[derive(Clone)]
pub struct MemoryBackend {
    storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl Default for MemoryBackend {
    fn default() -> MemoryBackend {
        MemoryBackend { storage: Arc::new(RwLock::new(HashMap::new())) }
    }
}

impl NewBackend for MemoryBackend {
    type Instance = MemoryBackend;

    fn new_backend(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
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
