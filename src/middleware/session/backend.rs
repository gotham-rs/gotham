use super::*;

use std::thread::{spawn, JoinHandle};
use std::collections::HashMap;
use std::sync::mpsc;

use futures::sync::oneshot;

enum Command {
    Get(SessionIdentifier, oneshot::Sender<Option<Vec<u8>>>),
    Take(SessionIdentifier, oneshot::Sender<Option<Vec<u8>>>),
    Put(SessionIdentifier, Vec<u8>),
}

pub struct NewMemoryBackend {
    join_handle: JoinHandle<()>,
    tx: mpsc::SyncSender<Command>,
}

pub struct MemoryBackend {
    tx: mpsc::SyncSender<Command>,
}

impl NewMemoryBackend {
    pub fn new(bound: usize) -> NewMemoryBackend {
        let (tx, rx) = mpsc::sync_channel(bound);
        let join_handle = spawn(move || NewMemoryBackend::run(rx));

        NewMemoryBackend { join_handle, tx }
    }

    fn run(rx: mpsc::Receiver<Command>) {
        let mut storage = HashMap::new();

        loop {
            match rx.recv() {
                Ok(Command::Get(id, tx)) => {
                    // We can't act on a failed oneshot channel, discard the result instead
                    let _discard = tx.send(storage.get(&id.value).map(Vec::clone));
                }
                Ok(Command::Take(id, tx)) => {
                    // We can't act on a failed oneshot channel, discard the result instead
                    let _discard = tx.send(storage.remove(&id.value));
                }
                Ok(Command::Put(id, t)) => {
                    storage.insert(id.value, t);
                }
                Err(mpsc::RecvError) => {
                    break;
                }
            }
        }
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
        Ok(MemoryBackend { tx: self.tx.clone() })
    }
}

impl MemoryBackend {
    fn send(&self, command: Command) -> Result<(), SessionError> {
        self.tx
            .send(command)
            .map_err(|_| SessionError::Backend("SyncSender::send returned Err(_)".into()))
    }
}

impl Backend for MemoryBackend {
    fn new_session(&self, content: &[u8]) -> Result<SessionIdentifier, SessionError> {
        let identifier = self.random_identifier();
        self.send(Command::Put(identifier.clone(), Vec::from(content))).map(|()| identifier)
    }

    fn update_session(&self,
                      identifier: SessionIdentifier,
                      content: &[u8])
                      -> Result<(), SessionError> {
        self.send(Command::Put(identifier.clone(), Vec::from(content)))
    }

    fn read_session(&self, identifier: SessionIdentifier) -> Box<SessionFuture> {
        let (tx, rx) = oneshot::channel();
        match self.send(Command::Get(identifier, tx)) {
            Ok(_) => {
                rx.map_err(|_| SessionError::Backend("Received cancelled".into()))
                    .boxed()
            }
            Err(e) => future::err(e).boxed(),
        }
    }
}
