use crate::constants::time::TT_MINUS_TAI_NANOSECONDS;

use super::{Duration, Error, Instant, LeapSeconds, Tai, TimeScale, Ut1MinusUtc};

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
    pub(crate) fn from_ut1_minus_utc(
        epoch: Instant<S>,
        ut1_minus_utc: Ut1MinusUtc,
        leap_seconds: LeapSeconds<'_>,
    ) -> Result<Self, Error> {
        let tai_minus_utc = leap_seconds.offset(epoch.retag::<Tai>())?;
        let tt_minus_ut1 = Duration::from_nanoseconds(TT_MINUS_TAI_NANOSECONDS)
            .checked_add(tai_minus_utc)?
            .checked_sub(ut1_minus_utc.as_duration())?;
        Ok(Self::new(epoch, tt_minus_ut1))
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
