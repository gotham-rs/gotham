//! Defines a session middleware with a pluggable backend.

use std::io;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::RefUnwindSafe;
use std::sync::{Arc, Mutex, PoisonError};

use base64;
use bincode;
use cookie::{Cookie, CookieJar};
use futures::{
    future::{self, FutureResult},
    Future,
};
use hyper::header::{HeaderMap, COOKIE, SET_COOKIE};
use hyper::{Body, Response, StatusCode};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use super::{Middleware, NewMiddleware};
use handler::{HandlerError, HandlerFuture, IntoHandlerError};
use helpers::http::response::create_empty_response;
use state::{self, FromState, State, StateData};

mod backend;
mod rng;

pub use self::backend::memory::MemoryBackend;
pub use self::backend::{Backend, NewBackend};

const SECURE_COOKIE_PREFIX: &str = "__Secure-";
const HOST_COOKIE_PREFIX: &str = "__Host-";

/// Represents the session identifier which is held in the user agent's session cookie.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionIdentifier {
    /// The value which is passed as a cookie, identifying the session.
    pub value: String,
}

/// The kind of failure which occurred trying to perform a session operation.
#[derive(Debug)]
pub enum SessionError {
    /// The backend failed, and the included message describes the problem.
    Backend(String),
    /// The session was unable to be deserialized.
    Deserialize,
    /// Exhaustive match against this enum is unsupported.
    #[doc(hidden)]
    __NonExhaustive,
}

enum SessionCookieState {
    New,
    Existing,
}

enum SessionDataState {
    Clean,
    Dirty,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum SameSiteEnforcement {
    Disabled,
    Strict,
    Lax,
}

/// Configuration for how the `Set-Cookie` header is generated.
///
/// By default, the cookie has the name "_gotham_session", and the cookie header includes the
/// `secure` flag.  `NewSessionMiddleware` provides functions for adjusting the
/// `SessionCookieConfig`.
#[derive(Clone, Debug)]
struct SessionCookieConfig {
    // If `Expires` / `Max-Age` are ever added update `reset_session` to allow for them.
    name: String,
    secure: bool,
    http_only: bool,
    same_site: SameSiteEnforcement,
    path: String,
    domain: Option<String>,
}

impl Default for SessionCookieConfig {
    fn default() -> SessionCookieConfig {
        SessionCookieConfig {
            name: "_gotham_session".to_string(),
            secure: true,
            http_only: true,
            same_site: SameSiteEnforcement::Lax,
            domain: None,
            path: "/".to_string(),
        }
    }
}

impl SessionCookieConfig {
    fn to_cookie_string(&self, value: &str) -> String {
        // Ensure this is always enough to prevent re-allocs
        let mut cookie_value = String::with_capacity(255);

        cookie_value.push_str(&self.name);
        cookie_value.push('=');
        cookie_value.push_str(value);

        if self.secure {
            cookie_value.push_str("; Secure")
        }

        if self.http_only {
            cookie_value.push_str("; HttpOnly")
        }

        match self.same_site {
            SameSiteEnforcement::Strict => cookie_value.push_str("; SameSite=Strict"),
            SameSiteEnforcement::Lax => cookie_value.push_str("; SameSite=Lax"),
            SameSiteEnforcement::Disabled => (),
        }

        if let Some(ref domain) = self.domain {
            cookie_value.push_str("; Domain=");
            cookie_value.push_str(domain);
        }

        cookie_value.push_str("; Path=");
        cookie_value.push_str(&self.path);

        cookie_value
    }

    /// Validates cookie attributes if the name includes a Cookie Prefix.
    /// see: https://tools.ietf.org/html/draft-west-cookie-prefixes-05
    /// Returns an updated `SessionCookieConfig` with any invalid attributes overridden and emits a warning.
    fn validate_prefix(self) -> SessionCookieConfig {
        if self.invalid_secure_config() {
            self.warn_overriding_attrs(SECURE_COOKIE_PREFIX, "Secure");
            SessionCookieConfig {
                secure: true,
                ..self
            }
        } else if self.invalid_host_config() {
            if !self.secure {
                self.warn_overriding_attrs(HOST_COOKIE_PREFIX, "Secure")
            };
            if self.domain.is_some() {
                self.warn_overriding_attrs(HOST_COOKIE_PREFIX, "Domain")
            };
            if self.path != "/".to_string() {
                self.warn_overriding_attrs(HOST_COOKIE_PREFIX, "Path")
            };
            SessionCookieConfig {
                secure: true,
                path: "/".to_string(),
                domain: None,
                ..self
            }
        } else {
            self
        }
    }

    fn invalid_secure_config(&self) -> bool {
        self.name.starts_with(SECURE_COOKIE_PREFIX) && !self.secure
    }

    fn invalid_host_config(&self) -> bool {
        self.name.starts_with(HOST_COOKIE_PREFIX)
            && (!self.secure || self.domain.is_some() || self.path != "/".to_string())
    }

    fn warn_overriding_attrs(&self, prefix: &str, attribute: &str) {
        warn!(
            "{} prefix is used for cookie but {} attribute is not set correctly! This will be overridden. Cookie is: {:?}",
            prefix, attribute, self
        )
    }
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
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate bincode;
/// # extern crate mime;
/// #
/// # use std::time::Duration;
/// # use futures::future;
/// # use gotham::handler::HandlerFuture;
/// # use gotham::state::{State, FromState};
/// # use gotham::middleware::{NewMiddleware, Middleware};
/// # use gotham::middleware::session::{SessionData, NewSessionMiddleware, Backend, MemoryBackend,
/// #                                   SessionIdentifier};
/// # use gotham::helpers::http::response::create_response;
/// # use gotham::test::TestServer;
/// # use hyper::{Body, Response, StatusCode};
/// # use hyper::header::COOKIE;
/// #
/// #[derive(Default, Serialize, Deserialize)]
/// struct MySessionType {
///     items: Vec<String>,
/// }
///
/// fn my_handler(state: State) -> (State, Response<Body>) {
///     // The `Router` has a `NewSessionMiddleware<_, MySessionType>` in a pipeline which is
///     // active for this handler.
///     let body = {
///         let session = SessionData::<MySessionType>::borrow_from(&state);
///         format!("{:?}", session.items)
///     };
///
///     let response = create_response(&state,
///                                    StatusCode::OK,
///                                    mime::TEXT_PLAIN,
///                                    body);
///
///     (state, response)
/// }
/// #
/// # fn main() {
/// #   let backend = MemoryBackend::new(Duration::from_secs(1));
/// #   let identifier = SessionIdentifier { value: "u0G6KdfckQgkV0qLANZjjNkEHBU".to_owned() };
/// #   let session = MySessionType {
/// #       items: vec!["a".into(), "b".into(), "c".into()],
/// #   };
/// #
/// #   let bytes = bincode::serialize(&session).unwrap();
/// #   backend.persist_session(identifier.clone(), &bytes[..]).unwrap();
/// #
/// #   let nm = NewSessionMiddleware::new(backend).with_session_type::<MySessionType>();
/// #
/// #   let new_handler = move || {
/// #       let handler = |state| {
/// #           let m = nm.new_middleware().unwrap();
/// #           let chain = |state| Box::new(future::ok(my_handler(state))) as Box<HandlerFuture>;
/// #
/// #           m.call(state, chain)
/// #       };
/// #
/// #       Ok(handler)
/// #   };
/// #
/// #   let test_server = TestServer::new(new_handler).unwrap();
/// #   let response = test_server
/// #       .client()
/// #       .get("http://localhost/")
/// #       .with_header(COOKIE, format!("_gotham_session={}", identifier.value.clone()).parse().unwrap())
/// #       .perform()
/// #       .unwrap();
/// #   let response_bytes = response.read_body().unwrap();
/// #   assert_eq!(String::from_utf8(response_bytes).unwrap(),
/// #              r#"["a", "b", "c"]"#);
/// # }
/// ```
pub struct SessionData<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    value: T,
    cookie_state: SessionCookieState,
    state: SessionDataState,
    identifier: SessionIdentifier,
    backend: Box<Backend + Send>,
    cookie_config: Arc<SessionCookieConfig>,
}

struct SessionDropData {
    cookie_config: Arc<SessionCookieConfig>,
}

impl<T> SessionData<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    /// Discards the session, invalidating it for future use and removing the data from the
    /// `Backend`.
    // TODO: Add test case that covers this.
    pub fn discard(self, state: &mut State) -> Result<(), SessionError> {
        state.put(SessionDropData {
            cookie_config: self.cookie_config,
        });
        self.backend.drop_session(self.identifier)
    }

    // Create a new, blank `SessionData<T>`
    fn new<B>(middleware: SessionMiddleware<B, T>) -> SessionData<T>
    where
        B: Backend + Send + 'static,
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
        B: Backend + Send + 'static,
    {
        let cookie_state = SessionCookieState::Existing;
        let state = SessionDataState::Clean;

        match val {
            Some(val) => {
                match bincode::deserialize::<T>(&val[..]) {
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

impl<T> StateData for SessionData<T> where
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static
{}

impl<T> Deref for SessionData<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for SessionData<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    fn deref_mut(&mut self) -> &mut T {
        self.state = SessionDataState::Dirty;
        &mut self.value
    }
}

impl StateData for SessionDropData {}

trait SessionTypePhantom<T>: Send + Sync + RefUnwindSafe
where
    T: Send,
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
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    new_backend: B,
    identifier_rng: Arc<Mutex<rng::SessionIdentifierRng>>,
    cookie_config: Arc<SessionCookieConfig>,
    phantom: PhantomData<SessionTypePhantom<T>>,
}

/// The per-request value which provides session storage for other middleware and handlers.
///
/// See `NewSessionMiddleware` for usage details.
pub struct SessionMiddleware<B, T>
where
    B: Backend,
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    backend: B,
    identifier_rng: Arc<Mutex<rng::SessionIdentifierRng>>,
    cookie_config: Arc<SessionCookieConfig>,
    phantom: PhantomData<T>,
}

impl<B, T> NewMiddleware for NewSessionMiddleware<B, T>
where
    B: NewBackend,
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    type Instance = SessionMiddleware<B::Instance, T>;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        self.new_backend
            .new_backend()
            .map(|backend| SessionMiddleware {
                backend,
                identifier_rng: self.identifier_rng.clone(),
                cookie_config: self.cookie_config.clone(),
                phantom: PhantomData,
            })
    }
}

impl<B, T> Clone for NewSessionMiddleware<B, T>
where
    B: NewBackend,
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    fn clone(&self) -> Self {
        NewSessionMiddleware {
            new_backend: self.new_backend.clone(),
            identifier_rng: self.identifier_rng.clone(),
            cookie_config: self.cookie_config.clone(),
            phantom: PhantomData,
        }
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
            cookie_config: Arc::new(SessionCookieConfig::default()),
            phantom: PhantomData,
        }
    }
}

impl<B, T> NewSessionMiddleware<B, T>
where
    B: NewBackend,
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    fn rebuild_new_session_middleware(
        self,
        cookie_config: SessionCookieConfig,
    ) -> NewSessionMiddleware<B, T> {
        NewSessionMiddleware {
            cookie_config: Arc::new(cookie_config.validate_prefix()),
            ..self
        }
    }

    /// Configures the session cookie to be set at a more restrictive path.
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
    ///     .with_cookie_path("/app".to_string())
    /// # ;}
    /// ```
    pub fn with_cookie_path<P>(self, path: P) -> NewSessionMiddleware<B, T>
    where
        P: AsRef<str>,
    {
        let cookie_config = SessionCookieConfig {
            path: path.as_ref().to_owned(),
            ..(*self.cookie_config).clone()
        };
        self.rebuild_new_session_middleware(cookie_config)
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
        let cookie_config = SessionCookieConfig {
            secure: false,
            ..(*self.cookie_config).clone()
        };
        self.rebuild_new_session_middleware(cookie_config)
    }

    /// Configures the `NewSessionMiddleware` to use an alternate cookie name. The default cookie
    /// name is `_gotham_session`.
    ///
    /// When a cookie name with a [cookie prefix][cookie-prefix] is used, the other options are
    /// forced to be correct (ignoring overridden settings from the application). i.e.:
    ///
    /// * For a cookie prefix of `__Secure-`, the cookie attributes will include `Secure`
    /// * For a cookie prefix of `__Host-`, the cookie attributes will include `Secure; Path=/` and
    ///   not include `Domain=`
    ///
    /// If the session cookie configuration set by the application does not match the prefix, a
    /// warning will be logged upon startup and the cookie prefix options will override what was
    /// provided by the application.
    ///
    /// [cookie-prefix]: https://tools.ietf.org/html/draft-west-cookie-prefixes-05
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
        self.rebuild_new_session_middleware(cookie_config)
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
    pub fn with_cookie_domain<S>(self, domain: S) -> NewSessionMiddleware<B, T>
    where
        S: AsRef<str>,
    {
        let cookie_config = SessionCookieConfig {
            domain: Some(domain.as_ref().to_owned()),
            ..(*self.cookie_config).clone()
        };
        self.rebuild_new_session_middleware(cookie_config)
    }

    /// Removes the `SameSite` cookie attribute, allowing cross-site requests to include the cookie.
    ///
    /// By default, the session cookie will be set with `SameSite=lax`, which ensures cross-site
    /// requests will include the cookie if and only if they are top-level navigations which use a
    /// "safe" (in the [RFC7231](https://tools.ietf.org/html/rfc7231#section-4.2.1) sense) HTTP
    /// method.
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
        let cookie_config = SessionCookieConfig {
            same_site: SameSiteEnforcement::Disabled,
            ..(*self.cookie_config).clone()
        };
        self.rebuild_new_session_middleware(cookie_config)
    }

    /// Sets the "SameSite" cookie attribute value to "strict".
    ///
    /// This will ensure that the cookie is never sent for cross-site requests (including top-level
    /// navigations).
    ///
    /// By default, the session cookie will be set with "SameSite=lax", which ensures cross-site
    /// requests will include the cookie if and only if they are top-level navigations which use a
    /// "safe" (in the [RFC7231](https://tools.ietf.org/html/rfc7231#section-4.2.1) sense) HTTP
    /// method.
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
        let cookie_config = SessionCookieConfig {
            same_site: SameSiteEnforcement::Strict,
            ..(*self.cookie_config).clone()
        };
        self.rebuild_new_session_middleware(cookie_config)
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
        U: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
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
    B: Backend + Send + 'static,
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture> + Send + 'static,
        Self: Sized,
    {
        let state_cookies = {
            HeaderMap::borrow_from(&state)
                .get_all(COOKIE)
                .iter()
                .flat_map(|cv| cv.to_str())
                .flat_map(|cs| Cookie::parse(cs.to_owned()))
                .fold(CookieJar::new(), |mut jar, cookie| {
                    jar.add_original(cookie);
                    jar
                })
        };

        let session_identifier = state_cookies
            .get(&self.cookie_config.name)
            .map(|cookie| cookie.value())
            .map(|value| SessionIdentifier {
                value: value.to_owned(),
            });

        match session_identifier {
            Some(id) => {
                trace!(
                    "[{}] SessionIdentifier {} found in cookie from user-agent",
                    state::request_id(&state),
                    id.value
                );

                let f = self
                    .backend
                    .read_session(id.clone())
                    .then(move |r| self.load_session_into_state(state, id, r))
                    .and_then(|state| chain(state))
                    .and_then(persist_session::<T>);

                Box::new(f)
            }
            None => {
                trace!(
                    "[{}] No SessionIdentifier found in cookie from user-agent",
                    state::request_id(&state),
                );

                let f = self
                    .new_session(state)
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
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    fn random_identifier(&self) -> SessionIdentifier {
        let mut bytes = [0u8; 64];

        match self.identifier_rng.lock() {
            Ok(mut rng) => rng.fill_bytes(&mut bytes),
            Err(PoisonError { .. }) => unreachable!("identifier_rng lock poisoned. Rng panicked?"),
        };

        SessionIdentifier {
            value: base64::encode_config(&bytes[..], base64::URL_SAFE_NO_PAD),
        }
    }
}

fn persist_session<T>(
    (mut state, mut response): (State, Response<Body>),
) -> FutureResult<(State, Response<Body>), (State, HandlerError)>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
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

fn send_cookie<B, T>(response: &mut Response<B>, session_data: &SessionData<T>)
where
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    let cookie_string = session_data
        .cookie_config
        .to_cookie_string(&session_data.identifier.value);
    write_cookie(cookie_string, response);
}

fn reset_cookie<B>(response: &mut Response<B>, session_drop_data: &SessionDropData) {
    let cookie_string = session_drop_data
        .cookie_config
        .to_cookie_string("discarded");
    let cookie_string = format!(
        "{}; expires=Thu, 01 Jan 1970 00:00:00 GMT; max-age=0",
        cookie_string
    );
    write_cookie(cookie_string, response);
}

fn write_cookie<B>(cookie: String, response: &mut Response<B>) {
    response
        .headers_mut()
        .append(SET_COOKIE, cookie.parse().unwrap());
}

fn write_session<T>(
    state: State,
    response: Response<Body>,
    session_data: SessionData<T>,
) -> future::FutureResult<(State, Response<Body>), (State, HandlerError)>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    let bytes = match bincode::serialize(&session_data.value) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(
                "[{}] failed to serialize session: {:?}",
                state::request_id(&state),
                e
            );

            let response = create_empty_response(&state, StatusCode::INTERNAL_SERVER_ERROR);

            return future::ok((state, response));
        }
    };

    let identifier = session_data.identifier;
    let slice = &bytes[..];

    let result = session_data
        .backend
        .persist_session(identifier.clone(), slice);

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
            let response = create_empty_response(&state, StatusCode::INTERNAL_SERVER_ERROR);

            future::ok((state, response))
        }
    }
}

impl<B, T> SessionMiddleware<B, T>
where
    B: Backend + 'static,
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + 'static,
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
                    "[{}] got response for session ({}) from backend, data located: {}",
                    state::request_id(&state),
                    identifier.value,
                    v.is_some()
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
    use hyper::header::{HeaderMap, COOKIE};
    use hyper::{Response, StatusCode};
    use rand;
    use std::sync::Mutex;
    use std::time::Duration;

    #[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
    struct TestSession {
        val: u64,
    }

    #[test]
    fn new_session() {
        let backend = MemoryBackend::new(Duration::from_secs(1));
        let nm = NewSessionMiddleware::new(backend).with_session_type::<TestSession>();
        let m = nm.new_middleware().unwrap();

        // Identifier generation is functioning as expected
        //
        // 64 -> 512 bits = (85 * 6 + 2)
        // Without padding that requires 86 base64 characters to represent.
        let identifier = m.random_identifier();
        assert_eq!(identifier.value.len(), 86);
        let identifier2 = m.random_identifier();
        assert_eq!(identifier2.value.len(), 86);
        assert_ne!(identifier, identifier2);

        assert_eq!(&m.cookie_config.name, "_gotham_session");
        assert!(m.cookie_config.secure);
        assert!(m.cookie_config.http_only);
        assert_eq!(m.cookie_config.same_site, SameSiteEnforcement::Lax);
        assert_eq!(&m.cookie_config.path, "/");
        assert!(m.cookie_config.domain.is_none());

        assert_eq!(
            m.cookie_config.to_cookie_string(&identifier.value),
            format!(
                "_gotham_session={}; Secure; HttpOnly; SameSite=Lax; Path=/",
                &identifier.value
            )
        );
    }

    #[test]
    fn enforce_secure_cookie_prefix_attributes() {
        let backend = MemoryBackend::new(Duration::from_secs(1));
        let nm = NewSessionMiddleware::new(backend.clone())
            .with_cookie_name("__Secure-my_session")
            .insecure()
            .with_session_type::<TestSession>();

        let m = nm.new_middleware().unwrap();
        assert!(m.cookie_config.secure);
    }

    #[test]
    fn enforce_host_cookie_prefix_attributes() {
        let backend = MemoryBackend::new(Duration::from_secs(1));
        let nm = NewSessionMiddleware::new(backend.clone())
            .with_cookie_name("__Host-my_session")
            .insecure()
            .with_cookie_domain("example.com")
            .with_cookie_path("/myapp")
            .with_session_type::<TestSession>();

        let m = nm.new_middleware().unwrap();
        assert!(m.cookie_config.secure);
        assert!(m.cookie_config.domain.is_none());
        assert!(m.cookie_config.path == "/".to_string());
    }

    #[test]
    fn new_session_custom_settings() {
        let backend = MemoryBackend::new(Duration::from_secs(1));
        let nm = NewSessionMiddleware::new(backend.clone())
            .with_cookie_name("_my_session")
            .with_cookie_domain("example.com")
            .with_strict_same_site_enforcement()
            .with_cookie_path("/myapp")
            .insecure()
            .with_session_type::<TestSession>();

        let m = nm.new_middleware().unwrap();
        let identifier = m.random_identifier();
        assert_eq!(identifier.value.len(), 86);

        assert_eq!(
            m.cookie_config.to_cookie_string(&identifier.value),
            format!(
                "_my_session={}; HttpOnly; SameSite=Strict; Domain=example.com; Path=/myapp",
                &identifier.value
            )
        );

        let nm = NewSessionMiddleware::new(backend)
            .with_cookie_name("x_session")
            .with_cookie_path("/xapp")
            .allow_cross_site_usage()
            .with_session_type::<TestSession>();

        let m = nm.new_middleware().unwrap();
        let identifier = m.random_identifier();
        assert_eq!(identifier.value.len(), 86);

        assert_eq!(
            m.cookie_config.to_cookie_string(&identifier.value),
            format!(
                "x_session={}; Secure; HttpOnly; Path=/xapp",
                &identifier.value
            )
        );
    }

    #[test]
    fn existing_session() {
        let nm = NewSessionMiddleware::default().with_session_type::<TestSession>();
        let m = nm.new_middleware().unwrap();

        let identifier = m.random_identifier();
        // 64 -> 512 bits = (85 * 6 + 2)
        // Without padding that requires 86 base64 characters to represent.
        assert_eq!(identifier.value.len(), 86);

        let session = TestSession {
            val: rand::random(),
        };
        let bytes = bincode::serialize(&session).unwrap();

        m.backend
            .persist_session(identifier.clone(), &bytes)
            .unwrap();

        let received: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
        let r = received.clone();

        let handler = move |mut state: State| {
            {
                let session_data = state.borrow_mut::<SessionData<TestSession>>();
                *r.lock().unwrap() = Some(session_data.val);
                session_data.val += 1;
            }

            Box::new(future::ok((
                state,
                Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .body(Body::empty())
                    .unwrap(),
            ))) as Box<HandlerFuture>
        };

        let mut state = State::new();
        let mut headers = HeaderMap::new();
        let cookie = Cookie::build("_gotham_session", identifier.value.clone()).finish();
        headers.insert(COOKIE, cookie.to_string().parse().unwrap());
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
        let updated = bincode::deserialize::<TestSession>(&bytes[..]).unwrap();

        assert_eq!(updated.val, session.val + 1);
    }
}
