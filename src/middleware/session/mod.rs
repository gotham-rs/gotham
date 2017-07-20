//! Defines a default session middleware supporting multiple backends

#![allow(missing_docs)]

use std::io;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};
use std::marker::PhantomData;

use rand;
use base64;
use hyper::{self, StatusCode};
use hyper::server::{Request, Response};
use hyper::header::{Cookie, SetCookie};
use futures::{future, Future};
use serde::{Serialize, Deserialize};
use rmp_serde;

use super::{NewMiddleware, Middleware};
use handler::HandlerFuture;
use state::{State, StateData};

mod backend;

pub use self::backend::MemoryBackend;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionIdentifier {
    value: String,
}

#[derive(Debug)]
pub enum SessionError {
    Backend(String),
    Deserialize,
}

enum SessionCookieState {
    New,
    Existing,
}

enum SessionDataState {
    Clean,
    Dirty,
}

enum SecureCookie {
    Insecure,
    Secure,
}

pub struct SessionCookieConfig {
    name: String,
    secure: SecureCookie,
}

pub struct SessionData<T>
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    value: T,
    cookie_state: SessionCookieState,
    state: SessionDataState,
    identifier: SessionIdentifier,
    backend: Box<Backend + Send>,
    cookie_config: Arc<SessionCookieConfig>,
}

impl<T> SessionData<T>
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    fn new(backend: Box<Backend + Send>,
           cookie_config: Arc<SessionCookieConfig>)
           -> SessionData<T> {
        let state = SessionDataState::Dirty; // Always persist a new session
        let cookie_state = SessionCookieState::New;
        let identifier = backend.random_identifier();
        let value = T::default();

        trace!(" no existing session, assigning new identifier ({})",
               identifier.value);

        SessionData {
            value,
            cookie_state,
            state,
            identifier,
            backend,
            cookie_config,
        }
    }

    fn construct(backend: Box<Backend + Send>,
                 cookie_config: Arc<SessionCookieConfig>,
                 identifier: SessionIdentifier,
                 val: Option<Vec<u8>>)
                 -> Result<SessionData<T>, SessionError> {
        let cookie_state = SessionCookieState::Existing;
        let state = SessionDataState::Clean;

        match val {
            Some(val) => {
                match T::deserialize(&mut rmp_serde::Deserializer::new(&val[..])) {
                    Ok(value) => {
                        trace!(" successfully deserialized session data ({})",
                               identifier.value);
                        Ok(SessionData {
                               value,
                               cookie_state,
                               state,
                               identifier,
                               backend,
                               cookie_config,
                           })
                    }
                    // TODO: What's the correct thing to do here? If the app changes the structure
                    // of its session type, the existing data won't deserialize anymore, through no
                    // fault of the users. Should we fall back to `T::default()` instead?
                    Err(_) => {
                        error!(" failed to deserialize session data ({})", identifier.value);
                        Err(SessionError::Deserialize)
                    }
                }
            }
            None => Ok(SessionData::<T>::new(backend, cookie_config)),
        }
    }
}

impl<T> StateData for SessionData<T>
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
}

impl<T> Deref for SessionData<T>
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for SessionData<T>
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    fn deref_mut(&mut self) -> &mut T {
        self.state = SessionDataState::Dirty;
        &mut self.value
    }
}

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

trait SessionTypePhantom<T>: Send + Sync where T: Send {}

pub struct NewSessionMiddleware<B, T>
    where B: NewBackend,
          T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    new_backend: B,
    cookie_config: Arc<SessionCookieConfig>,
    phantom: PhantomData<SessionTypePhantom<T>>,
}

pub struct SessionMiddleware<B, T>
    where B: Backend,
          T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    backend: B,
    cookie_config: Arc<SessionCookieConfig>,
    phantom: PhantomData<T>,
}

impl<B, T> NewMiddleware for NewSessionMiddleware<B, T>
    where B: NewBackend,
          T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    type Instance = SessionMiddleware<B::Instance, T>;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        self.new_backend
            .new_backend()
            .map(|backend| {
                     SessionMiddleware {
                         backend,
                         cookie_config: self.cookie_config.clone(),
                         phantom: PhantomData,
                     }
                 })
    }
}

impl<T> Default for NewSessionMiddleware<MemoryBackend, T>
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    fn default() -> NewSessionMiddleware<MemoryBackend, T> {
        NewSessionMiddleware {
            new_backend: MemoryBackend::default(),
            cookie_config: Arc::new(SessionCookieConfig {
                                        name: "_gotham_session".to_owned(),
                                        secure: SecureCookie::Insecure,
                                    }),
            phantom: PhantomData,
        }
    }
}

impl<B, T> Middleware for SessionMiddleware<B, T>
    where B: Backend + Send + 'static,
          T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    fn call<Chain>(self, state: State, request: Request, chain: Chain) -> Box<HandlerFuture>
        where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static,
              Self: Sized
    {
        let session_identifier = request
            .headers()
            .get::<Cookie>()
            .and_then(|c| c.get(self.cookie_config.name.as_ref()))
            .map(|value| SessionIdentifier { value: value.to_owned() });

        match session_identifier {
            Some(id) => {
                self.backend
                    .read_session(id.clone())
                    .then(move |r| self.load_session(state, id, r))
                    .and_then(|state| chain(state, request))
                    .and_then(persist_session::<T>)
                    .boxed()
            }
            None => {
                self.new_session(state)
                    .and_then(|state| chain(state, request))
                    .and_then(persist_session::<T>)
                    .boxed()
            }
        }
    }
}

fn persist_session<T>((mut state, mut response): (State, Response))
                      -> future::FutureResult<(State, Response), (State, hyper::Error)>
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    match state.take::<SessionData<T>>() {
        Some(session_data) => {
            let mut bytes = Vec::new();
            let ise_response = || Response::new().with_status(StatusCode::InternalServerError);

            if let SessionCookieState::New = session_data.cookie_state {
                let cookie_string = match session_data.cookie_config.secure {
                    SecureCookie::Insecure => {
                        format!("{}={}; HttpOnly",
                                session_data.cookie_config.name,
                                session_data.identifier.value)
                    }

                    SecureCookie::Secure => {
                        format!("{}={}; secure; HttpOnly",
                                session_data.cookie_config.name,
                                session_data.identifier.value)
                    }
                };

                let set_cookie = SetCookie(vec![cookie_string]);
                response.headers_mut().set(set_cookie);
            }

            match session_data.serialize(&mut rmp_serde::Serializer::new(&mut bytes)) {
                Ok(()) => {
                    match session_data.backend.update_session(session_data.identifier, &bytes[..]) {
                        Ok(()) => {
                            trace!(" persisted session successfully");
                            future::ok((state, response))
                        }
                        Err(_) => future::ok((state, ise_response())),
                    }
                }
                Err(_) => future::ok((state, ise_response())),
            }
        }
        None => future::ok((state, response)),
    }
}

impl<B, T> SessionMiddleware<B, T>
    where B: Backend + Send + 'static,
          T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    fn load_session(self,
                    mut state: State,
                    identifier: SessionIdentifier,
                    result: Result<Option<Vec<u8>>, SessionError>)
                    -> future::FutureResult<State, (State, hyper::Error)> {
        match result {
            Ok(v) => {
                let result = SessionData::<T>::construct(Box::new(self.backend),
                                                         self.cookie_config.clone(),
                                                         identifier,
                                                         v);
                match result {
                    Ok(session_data) => {
                        state.put(session_data);
                        future::ok(state)
                    }
                    Err(e) => {
                        let e = io::Error::new(io::ErrorKind::Other,
                                               format!("session couldn't be deserialized: {:?}",
                                                       e));
                        future::err((state, e.into()))
                    }
                }
            }
            Err(e) => {
                let e = io::Error::new(io::ErrorKind::Other,
                                       format!("backend failed to return session: {:?}", e));
                future::err((state, e.into()))
            }
        }
    }

    fn new_session(self, mut state: State) -> future::FutureResult<State, (State, hyper::Error)> {
        let session_data = SessionData::<T>::new(Box::new(self.backend),
                                                 self.cookie_config.clone());
        state.put(session_data);
        future::ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use hyper::{Method, StatusCode, Response};

    #[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
    struct TestSession {
        val: u64,
    }

    #[test]
    fn random_identifier() {
        let backend = MemoryBackend::default().new_backend().unwrap();
        assert!(backend.random_identifier() != backend.random_identifier(),
                "identifier collision");
    }

    #[test]
    fn existing_session() {
        let nm: NewSessionMiddleware<_, TestSession> = NewSessionMiddleware::default();
        let m = nm.new_middleware().unwrap();

        let identifier = m.backend.random_identifier();

        let session = TestSession { val: rand::random() };
        let mut bytes = Vec::new();
        session
            .serialize(&mut rmp_serde::Serializer::new(&mut bytes))
            .unwrap();

        m.backend
            .update_session(identifier.clone(), &bytes)
            .unwrap();

        let mut cookies = Cookie::new();
        cookies.set("_gotham_session", identifier.value.clone());

        let mut req: Request<hyper::Body> = Request::new(Method::Get, "/".parse().unwrap());
        req.headers_mut().set::<Cookie>(cookies);

        let received: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
        let r = received.clone();

        let handler = move |mut state: State, _req: Request| {
            {
                let session_data = state
                    .borrow_mut::<SessionData<TestSession>>()
                    .expect("no session data??");

                *r.lock().unwrap() = Some(session_data.val);
                session_data.val += 1;
            }

            future::ok((state, Response::new().with_status(StatusCode::Accepted))).boxed()
        };

        match m.call(State::new(), req, handler).wait() {
            Ok(_) => {
                let guard = received.lock().unwrap();
                if let Some(value) = *guard {
                    assert_eq!(value, session.val);
                } else {
                    panic!("no session data");
                }
            }
            Err(e) => panic!(e),
        }

        let m = nm.new_middleware().unwrap();
        let bytes = m.backend.read_session(identifier).wait().unwrap().unwrap();
        let updated = TestSession::deserialize(&mut rmp_serde::Deserializer::new(&bytes[..]))
            .unwrap();

        assert_eq!(updated.val, session.val + 1);
    }
}
