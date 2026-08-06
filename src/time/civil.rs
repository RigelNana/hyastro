use core::{fmt, marker::PhantomData};

use super::{Calendar, Date, Error, TimeScale, Utc};

/// A validated time-of-day label with nanosecond precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
}

impl TimeOfDay {
    /// Midnight at the start of a civil day.
    pub const MIDNIGHT: Self = Self {
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0,
    };

    /// Constructs a conventional time of day with seconds in 0..=59.
    pub fn new(hour: u8, minute: u8, second: u8, nanosecond: u32) -> Result<Self, Error> {
        Self::validate_components(hour, minute, second, nanosecond, false)
    }

    /// Constructs a conventional time of day without a fractional second.
    pub fn from_hms(hour: u8, minute: u8, second: u8) -> Result<Self, Error> {
        Self::new(hour, minute, second, 0)
    }

    /// Constructs the UTC label `23:59:60`.
    ///
    /// This only constructs the label. Resolving it to an instant still requires
    /// a leap-second-aware backend that validates the date.
    pub fn utc_leap_second_label(nanosecond: u32) -> Result<Self, Error> {
        Self::validate_components(23, 59, 60, nanosecond, true)
    }

    /// Returns the hour in 0..=23.
    pub const fn hour(self) -> u8 {
        self.hour
    }

    /// Returns the minute in 0..=59.
    pub const fn minute(self) -> u8 {
        self.minute
    }

    /// Returns the second, including 60 for a UTC leap-second label.
    pub const fn second(self) -> u8 {
        self.second
    }

    /// Returns nanoseconds within the labeled second.
    pub const fn nanosecond(self) -> u32 {
        self.nanosecond
    }

    /// Returns whether this is the label `23:59:60`.
    pub const fn is_leap_second(self) -> bool {
        self.second == 60
    }

    /// Returns nominal nanoseconds since midnight.
    ///
    /// A leap-second label returns a value in the additional second following
    /// the first 86,400 seconds.
    pub fn nanoseconds_since_midnight(self) -> u64 {
        (u64::from(self.hour) * 3_600 + u64::from(self.minute) * 60 + u64::from(self.second))
            * 1_000_000_000
            + u64::from(self.nanosecond)
    }

    /// Constructs a conventional time from nanoseconds in a nominal day.
    pub fn from_nanoseconds_since_midnight(value: u64) -> Result<Self, Error> {
        const NANOS_PER_DAY: u64 = 86_400_000_000_000;
        if value >= NANOS_PER_DAY {
            return Err(Error::component(
                "nanoseconds since midnight",
                i128::from(value),
                0,
                i128::from(NANOS_PER_DAY - 1),
            ));
        }
        let seconds = value / 1_000_000_000;
        let nanosecond = (value % 1_000_000_000) as u32;
        let hour = (seconds / 3_600) as u8;
        let minute = ((seconds % 3_600) / 60) as u8;
        let second = (seconds % 60) as u8;
        Self::new(hour, minute, second, nanosecond)
    }

    pub(crate) fn from_backend_components(
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> Result<Self, Error> {
        Self::validate_components(hour, minute, second, nanosecond, second == 60)
    }

    fn validate_components(
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
        allow_leap_second: bool,
    ) -> Result<Self, Error> {
        if hour > 23 {
            return Err(Error::component("hour", i128::from(hour), 0, 23));
        }
        if minute > 59 {
            return Err(Error::component("minute", i128::from(minute), 0, 59));
        }
        let maximum_second = if allow_leap_second { 60 } else { 59 };
        if second > maximum_second {
            return Err(Error::component(
                "second",
                i128::from(second),
                0,
                i128::from(maximum_second),
            ));
        }
        if second == 60 && (hour != 23 || minute != 59) {
            return Err(Error::component("leap second label", 60, 0, 59));
        }
        if nanosecond > 999_999_999 {
            return Err(Error::component(
                "nanosecond",
                i128::from(nanosecond),
                0,
                999_999_999,
            ));
        }
        Ok(Self {
            hour,
            minute,
            second,
            nanosecond,
        })
    }
}

/// A calendar date and time label carrying an explicit time scale.
pub struct DateTime<C: Calendar, S: TimeScale> {
    date: Date<C>,
    time: TimeOfDay,
    scale: PhantomData<S>,
}

impl<C: Calendar, S: TimeScale> DateTime<C, S> {
    /// Constructs a validated date-time label.
    pub fn new(date: Date<C>, time: TimeOfDay) -> Result<Self, Error> {
        if time.is_leap_second() && !S::LEAP_SECOND_LABELS {
            return Err(Error::LeapSecondNotRepresentable { target: S::NAME });
        }
        Ok(Self {
            date,
            time,
            scale: PhantomData,
        })
    }

    /// Constructs a label from calendar and clock components.
    pub fn from_components(
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> Result<Self, Error> {
        let date = Date::new(year, month, day)?;
        let time = TimeOfDay::from_backend_components(hour, minute, second, nanosecond)?;
        Self::new(date, time)
    }

    /// Returns the calendar date.
    pub const fn date(self) -> Date<C> {
        self.date
    }

    /// Returns the time-of-day label.
    pub const fn time(self) -> TimeOfDay {
        self.time
    }

    /// Converts the calendar while preserving the same civil day and scale label.
    pub fn convert_calendar<D: Calendar>(self) -> Result<DateTime<D, S>, Error> {
        DateTime::new(self.date.convert()?, self.time)
    }
}

impl<C: Calendar> DateTime<C, Utc> {
    /// Constructs a UTC leap-second label at `23:59:60` on a date.
    pub fn leap_second_label(date: Date<C>, nanosecond: u32) -> Result<Self, Error> {
        Self::new(date, TimeOfDay::utc_leap_second_label(nanosecond)?)
    }
}

impl<C: Calendar, S: TimeScale> Copy for DateTime<C, S> {}

impl<C: Calendar, S: TimeScale> Clone for DateTime<C, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: Calendar, S: TimeScale> PartialEq for DateTime<C, S> {
    fn eq(&self, other: &Self) -> bool {
        self.date == other.date && self.time == other.time
    }
}

impl<C: Calendar, S: TimeScale> Eq for DateTime<C, S> {}

impl<C: Calendar, S: TimeScale> fmt::Debug for DateTime<C, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DateTime")
            .field("calendar", &C::NAME)
            .field("scale", &S::NAME)
            .field("date", &self.date)
            .field("time", &self.time)
            .finish()
    }
}
