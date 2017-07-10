//! Defines a default session middleware supporting multiple backends

#![allow(missing_docs)]

use std::io;

use rand;
use base64;
use hyper::server::{Request, Response};
use futures::{future, Future};

use super::{NewMiddleware, Middleware};
use handler::HandlerFuture;
use state::State;

mod backend;

pub use self::backend::MemoryBackend;
pub use self::backend::NewMemoryBackend;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionIdentifier {
    value: String,
}

pub trait NewBackend {
    type Instance: Backend;

    fn new_backend(&self) -> io::Result<Self::Instance>;
}

pub type SessionFuture = Future<Item = Option<Vec<u8>>, Error = SessionError>;

pub trait Backend {
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

pub enum SessionError {
    Backend(String),
}

pub struct NewSessionMiddleware<T>
    where T: NewBackend
{
    t: T,
}

pub struct SessionMiddleware<T>
    where T: Backend
{
    t: T,
}

impl<T> NewMiddleware for NewSessionMiddleware<T>
    where T: NewBackend
{
    type Instance = SessionMiddleware<T::Instance>;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        self.t.new_backend().map(|t| SessionMiddleware { t })
    }
}

impl Default for NewSessionMiddleware<NewMemoryBackend> {
    fn default() -> NewSessionMiddleware<NewMemoryBackend> {
        NewSessionMiddleware { t: NewMemoryBackend::default() }
    }
}

impl<T> Middleware for SessionMiddleware<T>
    where T: Backend
{
    fn call<Chain>(self, state: State, request: Request, chain: Chain) -> Box<HandlerFuture>
        where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static,
              Self: Sized
    {
        future::empty().boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::cell::RefCell;
    use hyper::{Body, Method};

    enum TestSessionOperation {
        New,
        Update,
    }

    #[derive(Clone)]
    struct TestBackend {
        inner: Arc<RefCell<Vec<(SessionIdentifier, Vec<u8>, TestSessionOperation)>>>,
    }

    impl Default for TestBackend {
        fn default() -> TestBackend {
            TestBackend { inner: Arc::new(RefCell::new(Vec::with_capacity(10))) }
        }
    }

    impl NewBackend for TestBackend {
        type Instance = TestBackend;

        fn new_backend(&self) -> io::Result<TestBackend> {
            Ok(self.clone())
        }
    }

    impl Backend for TestBackend {
        fn new_session(&self, content: &[u8]) -> Result<SessionIdentifier, SessionError> {
            let mut vec = Vec::new();
            vec.extend_from_slice(content);
            let identifier = self.random_identifier();
            self.inner
                .borrow_mut()
                .push((identifier.clone(), vec, TestSessionOperation::New));

            Ok(identifier)
        }

        fn update_session(&self,
                          identifier: SessionIdentifier,
                          content: &[u8])
                          -> Result<(), SessionError> {
            let mut vec = Vec::new();
            vec.extend_from_slice(content);

            self.inner
                .borrow_mut()
                .push((identifier, vec, TestSessionOperation::Update));

            Ok(())
        }

        fn read_session(&self, identifier: SessionIdentifier) -> Box<SessionFuture> {
            let r = self.inner
                .borrow()
                .iter()
                .filter(|t| t.0 == identifier)
                .last()
                .map(|t| t.1.clone());
            future::ok(r).boxed()
        }
    }

    #[test]
    fn random_identifier() {
        let backend = TestBackend::default();
        assert!(backend.random_identifier() != backend.random_identifier(),
                "identifier collision");
    }

    #[test]
    fn existing_session() {
        let backend = TestBackend::default();
        let req = Request::<Body>::new(Method::Get, "/".parse().unwrap());

        let nm = NewSessionMiddleware { t: backend };
        let m = nm.new_middleware().unwrap();
    }

    #[test]
    fn default_impl_works() {
        let nm = NewSessionMiddleware::default();
    }
}
