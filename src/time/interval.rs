use core::fmt;

use super::{Duration, Error, Instant, TimeScale};

/// A non-empty closed interval between two physical instants in one typed scale.
pub struct TimeInterval<S: TimeScale> {
    start: Instant<S>,
    end: Instant<S>,
}

impl<S: TimeScale> TimeInterval<S> {
    /// Constructs a closed interval whose start must precede its end.
    pub fn new(start: Instant<S>, end: Instant<S>) -> Result<Self, Error> {
        if start >= end {
            return Err(Error::InvalidTimeInterval {
                start_tai_nanoseconds: start.tai_nanoseconds_since_1900(),
                end_tai_nanoseconds: end.tai_nanoseconds_since_1900(),
            });
        }
        Ok(Self { start, end })
    }

    /// Returns the inclusive interval start.
    pub const fn start(self) -> Instant<S> {
        self.start
    }

    /// Returns the inclusive interval end.
    pub const fn end(self) -> Instant<S> {
        self.end
    }

    /// Returns the exact positive interval duration.
    pub fn duration(self) -> Duration {
        Duration::from_nanoseconds(
            self.end.tai_nanoseconds_since_1900() - self.start.tai_nanoseconds_since_1900(),
        )
    }

    /// Reports whether the interval contains an instant, including both endpoints.
    pub fn contains(self, instant: Instant<S>) -> bool {
        self.start <= instant && instant <= self.end
    }
}

impl<S: TimeScale> Copy for TimeInterval<S> {}

impl<S: TimeScale> Clone for TimeInterval<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for TimeInterval<S> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl<S: TimeScale> Eq for TimeInterval<S> {}

impl<S: TimeScale> fmt::Debug for TimeInterval<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TimeInterval")
            .field("scale", &S::NAME)
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}
