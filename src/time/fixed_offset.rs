use core::{fmt, marker::PhantomData};

use super::{Calendar, Date, Duration, Error, TimeOfDay};

/// A constant signed offset from UTC without daylight-saving or historical rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedUtcOffset {
    seconds: i32,
}

impl FixedUtcOffset {
    /// UTC itself.
    pub const UTC: Self = Self { seconds: 0 };

    /// Constructs an offset shorter than 24 hours in either direction.
    pub fn from_seconds(seconds: i32) -> Result<Self, Error> {
        const MAXIMUM: i32 = 86_399;
        if !(-MAXIMUM..=MAXIMUM).contains(&seconds) {
            return Err(Error::component(
                "fixed UTC offset seconds",
                i128::from(seconds),
                i128::from(-MAXIMUM),
                i128::from(MAXIMUM),
            ));
        }
        Ok(Self { seconds })
    }

    /// Constructs a whole-hour offset east of UTC.
    pub fn east_hours(hours: u8) -> Result<Self, Error> {
        if hours > 23 {
            return Err(Error::component(
                "fixed UTC offset hours east",
                i128::from(hours),
                0,
                23,
            ));
        }
        Self::from_seconds(i32::from(hours) * 3_600)
    }

    /// Constructs a whole-hour offset west of UTC.
    pub fn west_hours(hours: u8) -> Result<Self, Error> {
        if hours > 23 {
            return Err(Error::component(
                "fixed UTC offset hours west",
                i128::from(hours),
                0,
                23,
            ));
        }
        Self::from_seconds(-i32::from(hours) * 3_600)
    }

    /// Returns signed seconds east of UTC.
    pub const fn seconds(self) -> i32 {
        self.seconds
    }

    /// Returns the offset as an exact signed duration.
    pub const fn as_duration(self) -> Duration {
        Duration::from_nanoseconds(self.seconds as i128 * Duration::NANOSECONDS_PER_SECOND)
    }
}

/// A conventional civil date-time paired with a constant UTC offset.
///
/// This value has no daylight-saving or historical time-zone rules. Leap-second
/// instants cannot be represented because their shifted `:60` label need not
/// occur at the end of the civil day.
pub struct CivilDateTime<C: Calendar> {
    date: Date<C>,
    time: TimeOfDay,
    offset: FixedUtcOffset,
    calendar: PhantomData<C>,
}

impl<C: Calendar> CivilDateTime<C> {
    /// Constructs a conventional fixed-offset civil label.
    pub fn new(date: Date<C>, time: TimeOfDay, offset: FixedUtcOffset) -> Result<Self, Error> {
        if time.is_leap_second() {
            return Err(Error::FixedOffsetLeapSecondUnsupported);
        }
        Ok(Self {
            date,
            time,
            offset,
            calendar: PhantomData,
        })
    }

    /// Returns the calendar date.
    pub const fn date(self) -> Date<C> {
        self.date
    }

    /// Returns the conventional time of day.
    pub const fn time(self) -> TimeOfDay {
        self.time
    }

    /// Returns the constant offset from UTC.
    pub const fn offset(self) -> FixedUtcOffset {
        self.offset
    }
}

impl<C: Calendar> Copy for CivilDateTime<C> {}

impl<C: Calendar> Clone for CivilDateTime<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: Calendar> PartialEq for CivilDateTime<C> {
    fn eq(&self, other: &Self) -> bool {
        self.date == other.date && self.time == other.time && self.offset == other.offset
    }
}

impl<C: Calendar> Eq for CivilDateTime<C> {}

impl<C: Calendar> fmt::Debug for CivilDateTime<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CivilDateTime")
            .field("calendar", &C::NAME)
            .field("date", &self.date)
            .field("time", &self.time)
            .field("offset", &self.offset)
            .finish()
    }
}
