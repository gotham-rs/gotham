//! Defines a default session middleware supporting multiple backends

use std::io;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};
use std::marker::PhantomData;

use hyper::{self, StatusCode};
use hyper::server::{Request, Response};
use hyper::header::{Cookie, SetCookie};
use futures::{future, Future};
use serde::{Serialize, Deserialize};
use rmp_serde;

use super::{NewMiddleware, Middleware};
use handler::HandlerFuture;
use state::{self, State, StateData, FromState};

mod backend;

pub use self::backend::{NewBackend, Backend};
pub use self::backend::memory::MemoryBackend;

/// Represents the session identifier which is held in the user agent's session cookie.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionIdentifier {
    /// The value which is passed as a cookie, identifying the session
    pub value: String,
}

/// The kind of failure which occurred trying to perform a session operation.
#[derive(Debug)]
pub enum SessionError {
    /// The backend failed, and a message describes why
    Backend(String),
    /// The session was unable to be deserialized
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

#[derive(Copy, Clone)]
enum SecureCookie {
    Insecure,
    Secure,
}

/// Configuration for how the `Set-Cookie` header is generated.
///
/// By default, the cookie has the name "_gotham_session", and the cookie header includes the
/// `secure` flag.  `NewSessionMiddleware` provides functions for adjusting the
/// `SessionCookieConfig`.
#[derive(Clone)]
pub struct SessionCookieConfig {
    name: String,
    secure: SecureCookie,
}

/// The wrapping type for application session data.
///
/// The application will receive a `SessionData<T>` via the `State` container, where `T` is the
/// session type given to `NewSessionMiddleware`.
///
/// ## Examples
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # extern crate futures;
/// # extern crate serde;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate rmp_serde;
/// #
/// # use std::time::Duration;
/// # use serde::Serialize;
/// # use futures::{future, Future, Stream};
/// # use gotham::state::State;
/// # use gotham::middleware::{NewMiddleware, Middleware};
/// # use gotham::middleware::session::{SessionData, NewSessionMiddleware, Backend, MemoryBackend};
/// # use hyper::header::Cookie;
/// # use hyper::server::{Request, Response};
/// # use hyper::{Method, StatusCode};
/// #
/// #[derive(Default, Serialize, Deserialize)]
/// struct MySessionType {
///     items: Vec<String>,
/// }
///
/// fn my_handler(state: State, _request: Request) -> (State, Response) {
///     // The `Router` has a `NewSessionMiddleware<_, MySessionType>` in a pipeline which is
///     // active for this handler.
///     let response = {
///         let session: &SessionData<MySessionType> = state.borrow().unwrap();
///
///         Response::new()
///             .with_status(StatusCode::Ok)
///             .with_body(format!("{:?}", session.items))
///     };
///
///     (state, response)
/// }
/// #
/// # fn main() {
/// #   let backend = MemoryBackend::new(Duration::from_secs(1));
/// #   let identifier = backend.random_identifier();
/// #   let mut bytes = Vec::new();
/// #   let session = MySessionType {
/// #       items: vec!["a".into(), "b".into(), "c".into()],
/// #   };
/// #
/// #   session.serialize(&mut rmp_serde::Serializer::new(&mut bytes)).unwrap();
/// #   backend.persist_session(identifier.clone(), &bytes[..]).unwrap();
/// #
/// #   let mut cookies = Cookie::new();
/// #   cookies.set("_gotham_session", identifier.value.clone());
/// #
/// #   let mut req = Request::new(Method::Get, "/".parse().unwrap());
/// #   req.headers_mut().set(cookies);
/// #
/// #   let state = State::new();
/// #
/// #   let nm = NewSessionMiddleware::new(backend).with_session_type::<MySessionType>();
/// #   let m = nm.new_middleware().unwrap();
/// #   let chain = |state, req| future::ok(my_handler(state, req)).boxed();
/// #   let (_state, response) = m.call(state, req, chain).wait().map_err(|(_, e)| e).unwrap();
/// #
/// #   let response_bytes = response
/// #       .body()
/// #       .concat2()
/// #       .wait()
/// #       .unwrap()
/// #       .to_vec();
/// #
/// #   assert_eq!(String::from_utf8(response_bytes).unwrap(),
/// #              r#"["a", "b", "c"]"#);
/// # }
/// ```
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
    /// Discards the session, invalidating it for future use and removing the data from the
    /// `Backend`.
    pub fn discard(self) -> Result<(), SessionError> {
        self.backend.drop_session(self.identifier)
    }

    // Create a new, blank `SessionData<T>`
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

    // Load an existing, serialized session into a `SessionData<T>`
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

impl<T> FromState<SessionData<T>> for SessionData<T>
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    fn take_from(s: &mut State) -> SessionData<T> {
        s.take::<SessionData<T>>()
            .unwrap_or_else(|| {
                                panic!("[{}] [take] SessionData<T> is not stored in State, \
                                        is NewSessionMiddleware configured correctly?",
                                       state::request_id(s))
                            })
    }

    fn borrow_from(s: &State) -> &SessionData<T> {
        s.borrow::<SessionData<T>>()
            .unwrap_or_else(|| {
                                panic!("[{}] [borrow] SessionData<T> is not stored in State, \
                                        is NewSessionMiddleware configured correctly?",
                                       state::request_id(s))
                            })
    }

    fn borrow_mut_from(s: &mut State) -> &mut SessionData<T> {
        let req_id = String::from(state::request_id(s));
        s.borrow_mut::<SessionData<T>>()
            .unwrap_or_else(|| {
                                panic!("[{}] [borrow_mut] SessionData<T> is not stored in State, \
                                        is NewSessionMiddleware configured correctly?",
                                       req_id)
                            })
    }
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

trait SessionTypePhantom<T>: Send + Sync where T: Send {}

/// Added to a `Pipeline`, this spawns the per-request `SessionMiddleware`
///
/// There are two ways to construct the `NewSessionMiddleware`, but `with_session_type` **must** be
/// called before the middleware is useful:
///
/// 1. Using the `Default` implementation, which sets up an in-memory session store. When
///    constructed this way, sessions are unable to be shared between multiple application servers,
///    and are lost on restart:
///
///     ```rust
///     # extern crate gotham;
///     # use gotham::middleware::session::NewSessionMiddleware;
///     # fn main() {
///     NewSessionMiddleware::default()
///     # ;}
///     ```
///
/// 2. Using the `NewSessionMiddleware::new` function, and providing a backend. The `Default`
///    implementation uses `MemoryBackend`, but this can be changed by providing your own:
///
///     ```rust
///     # extern crate gotham;
///     # use gotham::middleware::session::{MemoryBackend, NewSessionMiddleware};
///     # fn main() {
///     NewSessionMiddleware::new(MemoryBackend::default())
///     # ;}
///     ```
///
/// Before the middleware can be used, it must be associated with a session type provided by the
/// application. This gives type-safe storage for all session data:
///
/// ```rust
/// # extern crate gotham;
/// # #[macro_use]
/// # extern crate serde_derive;
/// #
/// # use gotham::middleware::session::NewSessionMiddleware;
/// #
/// #[derive(Default, Serialize, Deserialize)]
/// struct MySessionType {
///     items: Vec<String>,
/// }
///
/// # fn main() {
/// NewSessionMiddleware::default().with_session_type::<MySessionType>()
/// # ;}
/// ```
///
/// For plaintext HTTP servers, the `insecure` method must also be called to instruct the
/// middleware not to set the `secure` flag on the cookie.
pub struct NewSessionMiddleware<B, T>
    where B: NewBackend,
          T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    new_backend: B,
    cookie_config: Arc<SessionCookieConfig>,
    phantom: PhantomData<SessionTypePhantom<T>>,
}

/// The per-request value which deals with sessions
///
/// See `NewSessionMiddleware` for usage details.
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

impl Default for NewSessionMiddleware<MemoryBackend, ()> {
    fn default() -> NewSessionMiddleware<MemoryBackend, ()> {
        NewSessionMiddleware::new(MemoryBackend::default())
    }
}

impl<B> NewSessionMiddleware<B, ()>
    where B: NewBackend
{
    /// Create a `NewSessionMiddleware` value for the provided backend and with a blank session
    /// type. `with_session_type` **must** be called before the result is useful.
    pub fn new(b: B) -> NewSessionMiddleware<B, ()> {
        NewSessionMiddleware {
            new_backend: b,
            cookie_config: Arc::new(SessionCookieConfig {
                                        name: "_gotham_session".to_owned(),
                                        secure: SecureCookie::Secure,
                                    }),
            phantom: PhantomData,
        }
    }
}

impl<B, T> NewSessionMiddleware<B, T>
    where B: NewBackend,
          T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    /// Configures the `NewSessionMiddleware` not to send the `secure` flag along with the cookie.
    /// This is required for plaintext HTTP connections.
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate serde_derive;
    /// #
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// #
    /// # #[derive(Default, Serialize, Deserialize)]
    /// # struct MySessionType {
    /// #   items: Vec<String>,
    /// # }
    /// #
    /// # fn main() {
    /// NewSessionMiddleware::default()
    ///     .with_session_type::<MySessionType>()
    ///     .insecure()
    /// # ;}
    /// ```
    pub fn insecure(self) -> NewSessionMiddleware<B, T> {
        let cookie_config = SessionCookieConfig {
            secure: SecureCookie::Insecure,
            ..(*self.cookie_config).clone()
        };

        NewSessionMiddleware {
            cookie_config: Arc::new(cookie_config),
            ..self
        }
    }

    /// Changes the session type to the provided type parameter. This is required to override the
    /// default (unusable) session type of `()`.
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate serde_derive;
    /// #
    /// # use gotham::middleware::session::NewSessionMiddleware;
    /// #
    /// #[derive(Default, Serialize, Deserialize)]
    /// struct MySessionType {
    ///     items: Vec<String>,
    /// }
    ///
    /// # fn main() {
    /// NewSessionMiddleware::default().with_session_type::<MySessionType>()
    /// # ;}
    /// ```
    pub fn with_session_type<U>(self) -> NewSessionMiddleware<B, U>
        where U: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
    {
        NewSessionMiddleware {
            new_backend: self.new_backend,
            cookie_config: self.cookie_config,
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
            if let SessionCookieState::New = session_data.cookie_state {
                send_cookie(&mut response, &session_data);
            }

            match session_data.state {
                SessionDataState::Dirty => write_session(state, response, session_data),
                SessionDataState::Clean => future::ok((state, response)),
            }
        }
        // Session was discarded with `SessionData::discard`, or otherwise removed
        None => future::ok((state, response)),
    }
}

fn send_cookie<T>(response: &mut Response, session_data: &SessionData<T>)
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
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

fn write_session<T>(state: State,
                    response: Response,
                    session_data: SessionData<T>)
                    -> future::FutureResult<(State, Response), (State, hyper::Error)>
    where T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    let ise_response = || Response::new().with_status(StatusCode::InternalServerError);
    let mut bytes = Vec::new();

    {
        let mut serializer = rmp_serde::Serializer::new(&mut bytes);

        match session_data.value.serialize(&mut serializer) {
            Err(_) => return future::ok((state, ise_response())),
            Ok(_) => {}
        }
    }

    let identifier = session_data.identifier;
    let slice = &bytes[..];

    match session_data.backend.persist_session(identifier, slice) {
        Ok(()) => {
                                    trace!(" persisted session successfully");
                                    future::ok((state, response))
                                }
        Err(_) => future::ok((state, ise_response())),
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
    use rand;
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
        let nm = NewSessionMiddleware::default().with_session_type::<TestSession>();
        let m = nm.new_middleware().unwrap();

        let identifier = m.backend.random_identifier();

        let session = TestSession { val: rand::random() };
        let mut bytes = Vec::new();
        session
            .serialize(&mut rmp_serde::Serializer::new(&mut bytes))
            .unwrap();

        m.backend
            .persist_session(identifier.clone(), &bytes)
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
