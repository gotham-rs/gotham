//! Defines types for timing requests and emitting timing information.
use std::fmt::{self, Display, Formatter};
use std::time::{Duration, Instant};

use time::OffsetDateTime;

/// Timer struct used to record execution times of requests.
///
/// The `elapsed` function returns the elapsed time in an easy to format way,
/// suitable for use with requset logging middlewares.
#[derive(Clone, Copy)]
pub(crate) struct Timer {
    // We use 2 start fields
    // because we want formattable time to print start time
    // but we cannot use it to calculate duration because it is not monotonic.
    //
    // It is possible that we spent a lot of time between initialization of fields,
    // for example, if current thread unscheduled by OS but it should be very rare.
    // On the other hand, adjusting system clock by NTP is much more possible.
    start_monotonic: Instant,
    start_formattable: OffsetDateTime,
}

impl Timer {
    /// Begins measuring from the current time.
    pub(crate) fn new() -> Timer {
        Timer {
            start_monotonic: Instant::now(),
            start_formattable: OffsetDateTime::now_utc(),
        }
    }

    /// Finishes measuring, and returns the elapsed time as a `Timing` value.
    pub(crate) fn elapsed(&self) -> Timing {
        let duration = self.start_monotonic.elapsed();
        Timing(duration)
    }

    /// Retrieves the start time of this timer.
    pub(crate) fn start_time(&self) -> &OffsetDateTime {
        &self.start_formattable
    }
}

/// Represents an elapsed time measured by `Timer`.
#[derive(Clone, Copy)]
pub(crate) struct Timing(Duration);

impl Display for Timing {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let duration = self.0;
        match duration.as_micros() {
            i if i < 1000 => {
                write!(f, "{}µs", i)
            }
            i if i < 1_000_000 => {
                write!(f, "{:.2}ms", (i as f64) / 1000.0)
            }
            _ => {
                write!(f, "{:.2}s", duration.as_secs_f32())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Timing;

    #[test]
    fn test_durations() {
        let microsecond = Duration::from_micros(1);

        let t0 = Timing(microsecond * 555);
        assert_eq!(t0.to_string(), "555µs");

        let t1 = Timing(microsecond * 666_444);
        assert_eq!(t1.to_string(), "666.44ms");

        let t2 = Timing(microsecond * 777_444_333);
        assert_eq!(t2.to_string(), "777.44s");
    }
}
