use super::{Duration, Instant, TimeScale};

/// The resolved difference `TT−UT1` at one physical epoch.
///
/// Delta T is derived from observed `UT1−UTC`, the applicable `TAI−UTC`
/// offset, and the exact definition `TT−TAI = 32.184 s`. It is not a leap-
/// second count or another name for `UT1−UTC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeltaT<S: TimeScale> {
    epoch: Instant<S>,
    tt_minus_ut1: Duration,
}

impl<S: TimeScale> DeltaT<S> {
    pub(crate) const fn new(epoch: Instant<S>, tt_minus_ut1: Duration) -> Self {
        Self {
            epoch,
            tt_minus_ut1,
        }
    }

    /// Returns the physical epoch at which Delta T was resolved.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the exact nanosecond-rounded difference `TT−UT1`.
    pub const fn tt_minus_ut1(self) -> Duration {
        self.tt_minus_ut1
    }

    /// Returns `TT−UT1` in SI seconds.
    pub fn as_seconds(self) -> f64 {
        self.tt_minus_ut1.as_seconds_f64()
    }
}
