//! Defines a default session middleware supporting multiple backends

use std::{io, fmt};
use std::sync::{Arc, Mutex, PoisonError};
use std::ops::{Deref, DerefMut};
use std::marker::PhantomData;

use base64;
use rand::Rng;
use hyper::StatusCode;
use hyper::server::Response;
use hyper::header::{Headers, Cookie, SetCookie};
use futures::{future, Future};
use serde::{Serialize, Deserialize};
use rmp_serde;

use super::{NewMiddleware, Middleware};
use handler::{HandlerFuture, HandlerError, IntoHandlerError};
use state::{self, State, FromState, StateData};
use http::response::create_response;

mod backend;
mod rng;

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
    /// The backend failed, and the included message describes the problem
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

#[derive(Copy, Clone, PartialEq)]
enum SameSiteEnforcement {
    Strict,
    Lax,
}

#[derive(Clone, PartialEq)]
enum CookieOption {
    // If `Expires` / `Max-Age` are ever added here, be sure to update `reset_session` to allow
    // for them.
    Secure,
    HttpOnly,
    SameSite(SameSiteEnforcement),
    Domain(String),
}

impl fmt::Display for CookieOption {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        use self::CookieOption::*;
        use self::SameSiteEnforcement::*;

        match *self {
            Secure => out.write_str("secure"),
            HttpOnly => out.write_str("HttpOnly"),
            SameSite(Strict) => out.write_str("SameSite=strict"),
            SameSite(Lax) => out.write_str("SameSite=lax"),
            Domain(ref s) => {
                out.write_str("Domain=")?;
                out.write_str(s)
            }
        }
    }
}

/// Configuration for how the `Set-Cookie` header is generated.
///
/// By default, the cookie has the name "_gotham_session", and the cookie header includes the
/// `secure` flag.  `NewSessionMiddleware` provides functions for adjusting the
/// `SessionCookieConfig`.
#[derive(Clone)]
pub struct SessionCookieConfig {
    name: String,
    options: Vec<CookieOption>,
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
/// # use gotham::handler::{NewHandlerService, HandlerFuture};
/// # use gotham::state::{State, FromState};
/// # use gotham::middleware::{NewMiddleware, Middleware};
/// # use gotham::middleware::session::{SessionData, NewSessionMiddleware, Backend, MemoryBackend,
/// #                                   SessionIdentifier};
/// # use gotham::http::response::create_response;
/// # use hyper::header::Cookie;
/// # use hyper::server::{Response, Service};
/// # use hyper::{Request, Method, StatusCode};
/// # use hyper::mime;
/// #
/// #[derive(Default, Serialize, Deserialize)]
/// struct MySessionType {
///     items: Vec<String>,
/// }
///
/// fn my_handler(state: State) -> (State, Response) {
///     // The `Router` has a `NewSessionMiddleware<_, MySessionType>` in a pipeline which is
///     // active for this handler.
///     let body = {
///         let session = SessionData::<MySessionType>::borrow_from(&state);
///         format!("{:?}", session.items).into_bytes()
///     };
///
///     let response = create_response(&state,
///                                    StatusCode::Ok,
///                                    Some((body, mime::TEXT_PLAIN)));
///
///     (state, response)
/// }
/// #
/// # fn main() {
/// #   let backend = MemoryBackend::new(Duration::from_secs(1));
/// #   let identifier = SessionIdentifier { value: "u0G6KdfckQgkV0qLANZjjNkEHBU".to_owned() };
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
/// #   let nm = NewSessionMiddleware::new(backend).with_session_type::<MySessionType>();
/// #
/// #   let service = NewHandlerService::new(move || {
/// #       let handler = |state| {
/// #           let m = nm.new_middleware().unwrap();
/// #           let chain = |state| Box::new(future::ok(my_handler(state))) as Box<HandlerFuture>;
/// #
/// #           m.call(state, chain)
/// #       };
/// #
/// #       Ok(handler)
/// #   });
/// #
/// #   let response = service.call(req).wait().unwrap();
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
where
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    value: T,
    cookie_state: SessionCookieState,
    state: SessionDataState,
    identifier: SessionIdentifier,
    backend: Box<Backend>,
    cookie_config: Arc<SessionCookieConfig>,
}

struct SessionDropData {
    cookie_config: Arc<SessionCookieConfig>,
}

impl<T> SessionData<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    /// Discards the session, invalidating it for future use and removing the data from the
    /// `Backend`.
    pub fn discard(self, state: &mut State) -> Result<(), SessionError> {
        state.put(SessionDropData { cookie_config: self.cookie_config });
        self.backend.drop_session(self.identifier)
    }

    // Create a new, blank `SessionData<T>`
    fn new<B>(middleware: SessionMiddleware<B, T>) -> SessionData<T>
    where
        B: Backend + 'static,
    {
        let state = SessionDataState::Dirty; // Always persist a new session
        let cookie_state = SessionCookieState::New;
        let identifier = middleware.random_identifier();
        let value = T::default();
        let backend = Box::new(middleware.backend);
        let cookie_config = middleware.cookie_config.clone();

        trace!(
            " no existing session, assigning new identifier ({})",
            identifier.value
        );

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
    fn construct<B>(
        middleware: SessionMiddleware<B, T>,
        identifier: SessionIdentifier,
        val: Option<Vec<u8>>,
    ) -> SessionData<T>
    where
        B: Backend + 'static,
    {
        let cookie_state = SessionCookieState::Existing;
        let state = SessionDataState::Clean;

        match val {
            Some(val) => {
                match T::deserialize(&mut rmp_serde::Deserializer::new(&val[..])) {
                    Ok(value) => {
                        let backend = Box::new(middleware.backend);
                        let cookie_config = middleware.cookie_config.clone();

                        trace!(
                            " successfully deserialized session data ({})",
                            identifier.value
                        );

                        SessionData {
                            value,
                            cookie_state,
                            state,
                            identifier,
                            backend,
                            cookie_config,
                        }
                    }
                    Err(_) => {
                        // This is most likely caused by the application changing their session
                        // struct but the backend not being purged of sessions.
                        warn!(
                            " failed to deserialize session data ({}), falling back to new session",
                            identifier.value
                        );
                        SessionData::new(middleware)
                    }
                }
            }
            None => SessionData::new(middleware),
        }
    }
}

impl<T> StateData for SessionData<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
}

impl<T> Deref for SessionData<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for SessionData<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    fn deref_mut(&mut self) -> &mut T {
        self.state = SessionDataState::Dirty;
        &mut self.value
    }
}

impl StateData for SessionDropData {}

trait SessionTypePhantom<T>: Send + Sync
where
    T: Send
{
}

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
where
    B: NewBackend,
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    new_backend: B,
    identifier_rng: Arc<Mutex<rng::SessionIdentifierRng>>,
    cookie_config: Arc<SessionCookieConfig>,
    phantom: PhantomData<SessionTypePhantom<T>>,
}

/// The per-request value which deals with sessions
///
/// See `NewSessionMiddleware` for usage details.
pub struct SessionMiddleware<B, T>
where
    B: Backend,
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    backend: B,
    identifier_rng: Arc<Mutex<rng::SessionIdentifierRng>>,
    cookie_config: Arc<SessionCookieConfig>,
    phantom: PhantomData<T>,
}

impl<B, T> NewMiddleware for NewSessionMiddleware<B, T>
where
    B: NewBackend,
    T: Default
        + Serialize
        + for<'de> Deserialize<'de>
        + 'static,
{
    type Instance = SessionMiddleware<B::Instance, T>;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        self.new_backend.new_backend().map(|backend| {
            SessionMiddleware {
                backend,
                identifier_rng: self.identifier_rng.clone(),
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
where
    B: NewBackend,
{
    /// Create a `NewSessionMiddleware` value for the provided backend and with a blank session
    /// type. `with_session_type` **must** be called before the result is useful.
    pub fn new(b: B) -> NewSessionMiddleware<B, ()> {
        NewSessionMiddleware {
            new_backend: b,
            identifier_rng: Arc::new(Mutex::new(rng::session_identifier_rng())),
            cookie_config: Arc::new(SessionCookieConfig {
                name: "_gotham_session".to_owned(),
                options: default_cookie_options(),
            }),
            phantom: PhantomData,
        }
    }
}

fn default_cookie_options() -> Vec<CookieOption> {
    vec![
        CookieOption::HttpOnly,
        CookieOption::Secure,
        CookieOption::SameSite(SameSiteEnforcement::Lax),
    ]
}

impl<B, T> NewSessionMiddleware<B, T>
where
    B: NewBackend,
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    fn add_cookie_option(self, opt: CookieOption) -> NewSessionMiddleware<B, T> {
        let mut cookie_config = (*self.cookie_config).clone();
        cookie_config.options.push(opt);

        NewSessionMiddleware {
            cookie_config: Arc::new(cookie_config),
            ..self
        }
    }

    fn remove_cookie_option(self, opt: CookieOption) -> NewSessionMiddleware<B, T> {
        let mut cookie_config = (*self.cookie_config).clone();
        cookie_config.options = cookie_config
            .options
            .iter()
            .cloned()
            .filter(|v| *v != opt)
            .collect();

        NewSessionMiddleware {
            cookie_config: Arc::new(cookie_config),
            ..self
        }
    }

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
        self.remove_cookie_option(CookieOption::Secure)
    }

    /// Configures the `NewSessionMiddleware` to use an alternate cookie name.
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
    ///     .with_cookie_name("_myapp_session")
    /// # ;}
    /// ```
    pub fn with_cookie_name<S>(self, name: S) -> NewSessionMiddleware<B, T>
    where
        S: AsRef<str>,
    {
        let cookie_config = SessionCookieConfig {
            name: name.as_ref().to_owned(),
            ..(*self.cookie_config).clone()
        };

        NewSessionMiddleware {
            cookie_config: Arc::new(cookie_config),
            ..self
        }
    }

    /// Configures the `NewSessionMiddleware` to use a `Domain` attribute with the provided value.
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
    ///     .with_cookie_domain("example.com")
    /// # ;}
    /// ```
    pub fn with_cookie_domain<S>(self, name: S) -> NewSessionMiddleware<B, T>
    where
        S: AsRef<str>,
    {
        self.add_cookie_option(CookieOption::Domain(name.as_ref().to_owned()))
    }

    /// Removes the `SameSite` cookie attribute, allowing cross-site requests to include the cookie.
    ///
    /// By default, the session cookie will be set with `SameSite=lax`, which ensures cross-site
    /// requests will include the cookie if and only if they are top-level navigations which use a
    /// "safe" (in the RFC7231 sense) HTTP method.
    ///
    /// See: <https://tools.ietf.org/html/draft-ietf-httpbis-cookie-same-site-00#section-4.1>
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
    ///     .allow_cross_site_usage()
    /// # ;}
    /// ```
    pub fn allow_cross_site_usage(self) -> NewSessionMiddleware<B, T> {
        self.remove_cookie_option(CookieOption::SameSite(SameSiteEnforcement::Lax))
    }

    /// Sets the "SameSite" cookie attribute value to "strict".
    ///
    /// This will ensure that the cookie is never sent for cross-site requests (including top-level
    /// navigations).
    ///
    /// By default, the session cookie will be set with "SameSite=lax", which ensures cross-site
    /// requests will include the cookie if and only if they are top-level navigations which use a
    /// "safe" (in the [RFC7231] sense) HTTP method.
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
    ///     .with_strict_same_site_enforcement()
    /// # ;}
    /// ```
    pub fn with_strict_same_site_enforcement(self) -> NewSessionMiddleware<B, T> {
        self.remove_cookie_option(CookieOption::SameSite(SameSiteEnforcement::Lax))
            .add_cookie_option(CookieOption::SameSite(SameSiteEnforcement::Strict))
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
    where
        U: Default + Serialize + for<'de> Deserialize<'de> + 'static,
    {
        NewSessionMiddleware {
            new_backend: self.new_backend,
            identifier_rng: self.identifier_rng,
            cookie_config: self.cookie_config,
            phantom: PhantomData,
        }
    }
}

impl<B, T> Middleware for SessionMiddleware<B, T>
where
    B: Backend + 'static,
    T: Default
        + Serialize
        + for<'de> Deserialize<'de>
        + 'static,
{
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture> + 'static,
        Self: Sized,
    {
        let session_identifier = Headers::borrow_from(&state)
            .get::<Cookie>()
            .and_then(|c| c.get(self.cookie_config.name.as_ref()))
            .map(|value| SessionIdentifier { value: value.to_owned() });

        match session_identifier {
            Some(id) => {
                let f = self.backend
                    .read_session(id.clone())
                    .then(move |r| self.load_session_into_state(state, id, r))
                    .and_then(|state| chain(state))
                    .and_then(persist_session::<T>);

                Box::new(f)
            }
            None => {
                let f = self.new_session(state)
                    .and_then(|state| chain(state))
                    .and_then(persist_session::<T>);

                Box::new(f)
            }
        }
    }
}

impl<B, T> SessionMiddleware<B, T>
where
    B: Backend + 'static,
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    fn random_identifier(&self) -> SessionIdentifier {
        let mut bytes: Vec<u8> = Vec::with_capacity(64);

        match self.identifier_rng.lock() {
            Ok(mut rng) => rng.fill_bytes(bytes.as_mut_slice()),
            Err(PoisonError { .. }) => unreachable!("identifier_rng lock poisoned. Rng panicked?"),
        };

        SessionIdentifier { value: base64::encode_config(&bytes, base64::URL_SAFE_NO_PAD) }
    }
}

fn persist_session<T>(
    (mut state, mut response): (State, Response),
) -> future::FutureResult<(State, Response), (State, HandlerError)>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    match state.try_take::<SessionDropData>() {
        Some(ref session_drop_data) => {
            trace!(
                "[{}] SessionDropData found in state, removing session cookie from user agent",
                state::request_id(&state)
            );
            reset_cookie(&mut response, session_drop_data);
            return future::ok((state, response));
        }
        None => {
            trace!(
                "[{}] SessionDropData is not present, retaining session cookie",
                state::request_id(&state)
            );
        }
    }

    match state.try_take::<SessionData<T>>() {
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

fn cookie_string(cookie_config: &SessionCookieConfig, value: &str) -> String {
    let cookie_value = format!("{}={}", cookie_config.name, value);

    cookie_config.options.iter().fold(
        cookie_value,
        |acc, opt| {
            format!("{}; {}", acc, opt)
        },
    )
}

fn send_cookie<T>(response: &mut Response, session_data: &SessionData<T>)
where
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    let cookie_string = cookie_string(&*session_data.cookie_config, &session_data.identifier.value);

    let set_cookie = SetCookie(vec![cookie_string]);
    response.headers_mut().set(set_cookie);
}

fn reset_cookie(response: &mut Response, session_drop_data: &SessionDropData) {
    let cookie_string = cookie_string(&*session_drop_data.cookie_config, "discarded");
    let cookie_string = format!(
        "{}; expires=Thu, 01 Jan 1970 00:00:00 GMT; max-age=0",
        cookie_string
    );

    let set_cookie = SetCookie(vec![cookie_string]);
    response.headers_mut().set(set_cookie);
}

fn write_session<T>(
    state: State,
    response: Response,
    session_data: SessionData<T>,
) -> future::FutureResult<(State, Response), (State, HandlerError)>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    let mut bytes = Vec::new();

    {
        let mut serializer = rmp_serde::Serializer::new(&mut bytes);

        match session_data.value.serialize(&mut serializer) {
            Err(_) => {
                let response = create_response(&state, StatusCode::InternalServerError, None);
                return future::ok((state, response));
            }
            Ok(_) => {}
        }
    }

    let identifier = session_data.identifier;
    let slice = &bytes[..];

    let result = session_data.backend.persist_session(
        identifier.clone(),
        slice,
    );

    match result {
        Ok(_) => {
            trace!(
                "[{}] persisted session ({}) successfully",
                state::request_id(&state),
                identifier.value
            );

            future::ok((state, response))
        }
        Err(_) => {
            let response = create_response(&state, StatusCode::InternalServerError, None);
            return future::ok((state, response));
        }
    }
}

impl<B, T> SessionMiddleware<B, T>
where
    B: Backend + 'static,
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    fn load_session_into_state(
        self,
        mut state: State,
        identifier: SessionIdentifier,
        result: Result<Option<Vec<u8>>, SessionError>,
    ) -> future::FutureResult<State, (State, HandlerError)> {
        match result {
            Ok(v) => {
                trace!(
                    "[{}] retrieved session ({}) from backend successfully",
                    state::request_id(&state),
                    identifier.value
                );

                let session_data = SessionData::<T>::construct(self, identifier, v);

                state.put(session_data);
                future::ok(state)
            }
            Err(e) => {
                error!(
                    "[{}] failed to retrieve session ({}) from backend: {:?}",
                    state::request_id(&state),
                    identifier.value,
                    e
                );

                let e = io::Error::new(
                    io::ErrorKind::Other,
                    format!("backend failed to return session: {:?}", e),
                );

                future::err((state, e.into_handler_error()))
            }
        }
    }

    fn new_session(self, mut state: State) -> future::FutureResult<State, (State, HandlerError)> {
        let session_data = SessionData::<T>::new(self);

        trace!(
            "[{}] created new session ({})",
            state::request_id(&state),
            session_data.identifier.value
        );

        state.put(session_data);

        future::ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use rand;
    use hyper::{StatusCode, Response};
    use hyper::header::Headers;


    #[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
    struct TestSession {
        val: u64,
    }

    #[test]
    fn existing_session() {
        let nm = NewSessionMiddleware::default().with_session_type::<TestSession>();
        let m = nm.new_middleware().unwrap();

        let identifier = m.random_identifier();

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

        let received: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
        let r = received.clone();

        let handler = move |mut state: State| {
            {
                let session_data = state.borrow_mut::<SessionData<TestSession>>();
                *r.lock().unwrap() = Some(session_data.val);
                session_data.val += 1;
            }

            Box::new(future::ok(
                (state, Response::new().with_status(StatusCode::Accepted)),
            )) as Box<HandlerFuture>
        };

        let mut state = State::new();
        let mut headers = Headers::new();
        headers.set::<Cookie>(cookies);
        state.put(headers);

        let r: Box<HandlerFuture> = m.call(state, handler);
        match r.wait() {
            Ok(_) => {
                let guard = received.lock().unwrap();
                if let Some(value) = *guard {
                    assert_eq!(value, session.val);
                } else {
                    panic!("no session data");
                }
            }
            Err((_, e)) => panic!("error: {:?}", e),
        }

        let m = nm.new_middleware().unwrap();
        let bytes = m.backend.read_session(identifier).wait().unwrap().unwrap();
        let updated = TestSession::deserialize(&mut rmp_serde::Deserializer::new(&bytes[..]))
            .unwrap();

        assert_eq!(updated.val, session.val + 1);
    }
}
