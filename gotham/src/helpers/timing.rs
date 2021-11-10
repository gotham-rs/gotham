//! Defines types for timing requests and emitting timing information.
use std::fmt::{self, Display, Formatter};

use time::OffsetDateTime;

/// Timer struct used to record execution times of requests.
///
/// The `elapsed` function returns the elapsed time in an easy to format way,
/// suitable for use with requset logging middlewares.
#[derive(Clone, Copy)]
pub(crate) struct Timer {
    start: OffsetDateTime,
}

impl Timer {
    /// Begins measuring from the current time.
    pub(crate) fn new() -> Timer {
        Timer {
            start: OffsetDateTime::now_utc(),
        }
    }

    /// Finishes measuring, and returns the elapsed time as a `Timing` value.
    pub(crate) fn elapsed(&self) -> Timing {
        let duration = (OffsetDateTime::now_utc() - self.start).whole_microseconds();
        Timing(duration)
    }

    /// Retrieves the start time of this timer.
    pub(crate) fn start_time(&self) -> &OffsetDateTime {
        &self.start
    }
}

/// Represents an elapsed time measured by `Timer`.
#[derive(Clone, Copy)]
pub(crate) struct Timing(
    /// A number of microseconds measured by `Timer`.
    i128,
);

impl Display for Timing {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.0 {
            i if i < 1000 => {
                write!(f, "{}µs", i)
            }
            i if i < 1_000_000 => {
                write!(f, "{:.2}ms", (i as f64) / 1000.0)
            }
            i => {
                write!(f, "{:.2}s", (i as f32) / 1_000_000.0)
            }
        }
    }
}
