//! Defines a default session middleware supporting multiple backends

#![allow(missing_docs)]

use std::io;
use std::sync::Arc;

use rand;
use base64;
use hyper;
use hyper::server::Request;
use hyper::header::Cookie;
use futures::{future, Future};

use super::{NewMiddleware, Middleware};
use handler::HandlerFuture;
use state::{State, StateData};

mod backend;

pub use self::backend::MemoryBackend;
pub use self::backend::NewMemoryBackend;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionIdentifier {
    value: String,
}

pub struct SessionData {
    value: Vec<u8>,
}

impl StateData for SessionData {}

pub trait NewBackend {
    type Instance: Backend + Send + 'static;

    fn new_backend(&self) -> io::Result<Self::Instance>;
}

pub type SessionFuture = Future<Item = Option<Vec<u8>>, Error = SessionError> + Send;

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

#[derive(Debug)]
pub enum SessionError {
    Backend(String),
}

pub struct NewSessionMiddleware<T>
    where T: NewBackend
{
    t: T,
    cookie: Arc<String>,
}

pub struct SessionMiddleware<T>
    where T: Backend
{
    t: T,
    cookie: Arc<String>,
}

impl<T> NewMiddleware for NewSessionMiddleware<T>
    where T: NewBackend
{
    type Instance = SessionMiddleware<T::Instance>;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        let cookie = self.cookie.clone();

        self.t
            .new_backend()
            .map(move |t| SessionMiddleware { t, cookie })
    }
}

impl Default for NewSessionMiddleware<NewMemoryBackend> {
    fn default() -> NewSessionMiddleware<NewMemoryBackend> {
        NewSessionMiddleware {
            t: NewMemoryBackend::default(),
            cookie: Arc::new("_gotham_session".to_owned()),
        }
    }
}

impl<T> Middleware for SessionMiddleware<T>
    where T: Backend + Send + 'static
{
    fn call<Chain>(self, state: State, request: Request, chain: Chain) -> Box<HandlerFuture>
        where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static,
              Self: Sized
    {
        let session_identifier = request
            .headers()
            .get::<Cookie>()
            .and_then(|c| c.get(self.cookie.as_ref()))
            .map(|value| SessionIdentifier { value: value.to_owned() });

        match session_identifier {
            Some(id) => {
                self.t
                    .read_session(id)
                    .then(move |r| self.store_session(state, r))
                    .and_then(|state| chain(state, request))
                    .boxed()
            }
            None => chain(state, request),
        }
    }
}

impl<T> SessionMiddleware<T>
    where T: Backend + Send + 'static
{
    fn store_session(&self,
                     mut state: State,
                     result: Result<Option<Vec<u8>>, SessionError>)
                     -> future::FutureResult<State, (State, hyper::Error)> {
        match result {
            Ok(Some(value)) => {
                state.put(SessionData { value });
                future::ok(state)
            }
            Ok(None) => future::ok(state),
            Err(e) => {
                let e = io::Error::new(io::ErrorKind::Other,
                                       format!("backend failed to return session: {:?}", e));
                future::err((state, e.into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use hyper::{Body, Method, StatusCode, Response};

    #[test]
    fn random_identifier() {
        let backend = NewMemoryBackend::default().new_backend().unwrap();
        assert!(backend.random_identifier() != backend.random_identifier(),
                "identifier collision");
    }

    #[test]
    fn existing_session() {
        let nm = NewSessionMiddleware::default();
        let m = nm.new_middleware().unwrap();

        let identifier = m.t.random_identifier();
        let bytes: Vec<u8> = (0..64).map(|_| rand::random()).collect();

        m.t.update_session(identifier.clone(), &bytes);

        let mut cookies = Cookie::new();
        cookies.set("_gotham_session", identifier.value);

        let mut req: Request<hyper::Body> = Request::new(Method::Get, "/".parse().unwrap());
        req.headers_mut().set::<Cookie>(cookies);

        let received: Arc<Mutex<Option<SessionData>>> = Arc::new(Mutex::new(None));
        let r = received.clone();

        let f = move |mut state: State, req: Request| {
            *r.lock().unwrap() = state.take::<SessionData>();
            future::ok((state, Response::new().with_status(StatusCode::Accepted))).boxed()
        };

        match m.call(State::new(), req, f).wait() {
            Ok(_) => {
                let guard = received.lock().unwrap();
                if let Some(SessionData { ref value }) = *guard {
                    assert_eq!(value, &bytes);
                } else {
                    panic!("no session data");
                }
            }
            Err(e) => panic!(e),
        }
    }
}
