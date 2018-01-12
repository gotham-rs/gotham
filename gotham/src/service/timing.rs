//! Defines types for timing requests and emitting timing information into logs and responses.

use std::fmt::{self, Display, Formatter};

use chrono::prelude::*;
use hyper::Response;

use state::{request_id, State};
use http::header::XRuntimeMicroseconds;

/// Used by `GothamService` to time requests. The `elapsed` function returns the elapsed time
/// in a way that can be used for logging and adding the `X-Runtime-Microseconds` header to
/// responses.
#[derive(Clone, Copy)]
pub(super) struct Timer {
    start: DateTime<Utc>,
}

impl Timer {
    /// Begins measuring from the current time.
    pub(super) fn new() -> Timer {
        Timer { start: Utc::now() }
    }

    /// Finishes measuring, and returns the elapsed time as a `Timing` value.
    pub(super) fn elapsed(self, state: &State) -> Timing {
        let timing = self.elapsed_no_logging();

        if let Timing::Invalid = timing {
            error!(
                "[{}] Unable to measure timing of request, num_microseconds was None",
                request_id(state)
            );
        }

        timing
    }

    pub(super) fn elapsed_no_logging(self) -> Timing {
        let Timer { start } = self;
        match Utc::now().signed_duration_since(start).num_microseconds() {
            Some(dur) => Timing::Microseconds(dur),
            None => Timing::Invalid,
        }
    }
}

/// Represents an elapsed time measured by `Timer`.
#[derive(Clone, Copy)]
pub(super) enum Timing {
    /// A number of microseconds measured by `Timer`.
    Microseconds(i64),

    /// An invalid state, where the amount of time elapsed was unable to be measured.
    Invalid,
}

impl Timing {
    /// Converts a `Response` into a new `Response` with the `X-Runtime-Microseconds` header
    /// included (assuming the time elapsed was able to be measured).
    pub(super) fn add_to_response(&self, response: Response) -> Response {
        match *self {
            Timing::Microseconds(i) => response.with_header(XRuntimeMicroseconds(i)),
            Timing::Invalid => response,
        }
    }
}

impl Display for Timing {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Timing::Microseconds(i) => {
                i.fmt(f)?;
                f.write_str("µs")
            }
            Timing::Invalid => f.write_str("invalid"),
        }
    }
}
