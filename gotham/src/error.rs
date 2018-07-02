use failure::{self, Backtrace, Context, Fail};
use std::fmt::{self, Display};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

pub type Result<T> = ::std::result::Result<T, failure::Error>;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "The Gotham devs haven't gotten around to characterizing this error yet.")]
    LazyDevelopers,

    #[fail(display = "The response was not received before the timeout duration elapsed.")]
    TimedOut,
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}
