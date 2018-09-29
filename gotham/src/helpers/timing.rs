//! Defines types for timing requests and emitting timing information.
use chrono::prelude::*;
use std::fmt::{self, Display, Formatter};

/// Timer struct used to record execution times of requests.
///
/// The `elapsed` function returns the elapsed time in an easy to format way,
/// suitable for use with requset logging middlewares.
#[derive(Clone, Copy)]
pub struct Timer {
    start: DateTime<Utc>,
}

impl Timer {
    /// Begins measuring from the current time.
    pub fn new() -> Timer {
        Timer { start: Utc::now() }
    }

    /// Finishes measuring, and returns the elapsed time as a `Timing` value.
    pub fn elapsed(&self) -> Timing {
        let duration = Utc::now()
            .signed_duration_since(self.start)
            .num_microseconds();

        match duration {
            Some(dur) => Timing::Microseconds(dur),
            None => Timing::Invalid,
        }
    }

    /// Retrieves the start time of this timer.
    pub fn start_time(&self) -> &DateTime<Utc> {
        &self.start
    }
}

/// Represents an elapsed time measured by `Timer`.
#[derive(Clone, Copy)]
pub enum Timing {
    /// A number of microseconds measured by `Timer`.
    Microseconds(i64),

    /// An invalid state, where the amount of time elapsed was unable to be measured.
    Invalid,
}

impl Display for Timing {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Timing::Microseconds(i) => {
                if i < 1000 {
                    write!(f, "{}µs", i)
                } else if i < 1000000 {
                    write!(f, "{:.2}ms", (i as f32) / 1000.0)
                } else {
                    write!(f, "{:.2}s", (i as f32) / 1000000.0)
                }
            }
            Timing::Invalid => f.write_str("invalid"),
        }
    }
}
