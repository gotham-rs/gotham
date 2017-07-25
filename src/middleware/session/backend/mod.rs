pub(super) mod memory;

use std::io;

use base64;
use rand;
use futures::Future;

use middleware::session::{SessionError, SessionIdentifier};

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
