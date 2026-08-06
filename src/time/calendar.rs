use core::{fmt, hash::Hash, marker::PhantomData};

use super::Error;

mod sealed {
    pub trait Sealed {}
}

/// A sealed proleptic calendar used by [`Date`].
pub trait Calendar: sealed::Sealed + Copy + Clone + fmt::Debug + Eq {
    /// Human-readable calendar name.
    const NAME: &'static str;

    /// Returns whether an astronomically numbered year is a leap year.
    #[doc(hidden)]
    fn is_leap_year(year: i32) -> bool;

    /// Converts a valid date to a Julian Day Number.
    #[doc(hidden)]
    fn to_julian_day_number(year: i32, month: u8, day: u8) -> i64;

    /// Converts a Julian Day Number to year, month, and day.
    #[doc(hidden)]
    fn from_julian_day_number(value: i64) -> Result<(i32, u8, u8), Error>;

    /// Returns the number of days in a month.
    #[doc(hidden)]
    fn days_in_month(year: i32, month: u8) -> Option<u8> {
        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(year) => 29,
            2 => 28,
            _ => return None,
        };
        Some(days)
    }
}

/// The proleptic Gregorian calendar with astronomical year numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gregorian;

impl sealed::Sealed for Gregorian {}

impl Calendar for Gregorian {
    const NAME: &'static str = "proleptic Gregorian";

    fn is_leap_year(year: i32) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    fn to_julian_day_number(year: i32, month: u8, day: u8) -> i64 {
        let month = i64::from(month);
        let day = i64::from(day);
        let correction = (14 - month).div_euclid(12);
        let shifted_year = i64::from(year) + 4_800 - correction;
        let shifted_month = month + 12 * correction - 3;
        day + (153 * shifted_month + 2).div_euclid(5)
            + 365 * shifted_year
            + shifted_year.div_euclid(4)
            - shifted_year.div_euclid(100)
            + shifted_year.div_euclid(400)
            - 32_045
    }

    fn from_julian_day_number(value: i64) -> Result<(i32, u8, u8), Error> {
        let a = i128::from(value) + 32_044;
        let b = (4 * a + 3).div_euclid(146_097);
        let c = a - (146_097 * b).div_euclid(4);
        let d = (4 * c + 3).div_euclid(1_461);
        let e = c - (1_461 * d).div_euclid(4);
        let m = (5 * e + 2).div_euclid(153);
        let day = e - (153 * m + 2).div_euclid(5) + 1;
        let month = m + 3 - 12 * m.div_euclid(10);
        let year = 100 * b + d - 4_800 + m.div_euclid(10);
        Date::<Self>::checked_components(year, month, day)
    }
}

/// The proleptic Julian calendar with astronomical year numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Julian;

impl sealed::Sealed for Julian {}

impl Calendar for Julian {
    const NAME: &'static str = "proleptic Julian";

    fn is_leap_year(year: i32) -> bool {
        year % 4 == 0
    }

    fn to_julian_day_number(year: i32, month: u8, day: u8) -> i64 {
        let month = i64::from(month);
        let day = i64::from(day);
        let correction = (14 - month).div_euclid(12);
        let shifted_year = i64::from(year) + 4_800 - correction;
        let shifted_month = month + 12 * correction - 3;
        day + (153 * shifted_month + 2).div_euclid(5)
            + 365 * shifted_year
            + shifted_year.div_euclid(4)
            - 32_083
    }

    fn from_julian_day_number(value: i64) -> Result<(i32, u8, u8), Error> {
        let c = i128::from(value) + 32_082;
        let d = (4 * c + 3).div_euclid(1_461);
        let e = c - (1_461 * d).div_euclid(4);
        let m = (5 * e + 2).div_euclid(153);
        let day = e - (153 * m + 2).div_euclid(5) + 1;
        let month = m + 3 - 12 * m.div_euclid(10);
        let year = d - 4_800 + m.div_euclid(10);
        Date::<Self>::checked_components(year, month, day)
    }
}

/// An integer Julian day beginning at noon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JulianDayNumber(i64);

impl JulianDayNumber {
    /// Constructs a Julian Day Number.
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Returns the integer day number.
    pub const fn value(self) -> i64 {
        self.0
    }
}

/// A validated date in calendar `C` using astronomical year numbering.
pub struct Date<C: Calendar> {
    year: i32,
    month: u8,
    day: u8,
    calendar: PhantomData<C>,
}

impl<C: Calendar> Date<C> {
    pub(crate) const fn from_valid_components(year: i32, month: u8, day: u8) -> Self {
        Self {
            year,
            month,
            day,
            calendar: PhantomData,
        }
    }

    /// Constructs a validated date.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, Error> {
        let Some(maximum_day) = C::days_in_month(year, month) else {
            return Err(Error::InvalidDate {
                year,
                month,
                day,
                calendar: C::NAME,
            });
        };
        if day == 0 || day > maximum_day {
            return Err(Error::InvalidDate {
                year,
                month,
                day,
                calendar: C::NAME,
            });
        }
        Ok(Self {
            year,
            month,
            day,
            calendar: PhantomData,
        })
    }

    /// Constructs a date from a Julian Day Number.
    pub fn from_julian_day_number(value: JulianDayNumber) -> Result<Self, Error> {
        let (year, month, day) = C::from_julian_day_number(value.0)?;
        Self::new(year, month, day)
    }

    /// Returns the astronomically numbered year, including year zero.
    pub const fn year(self) -> i32 {
        self.year
    }

    /// Returns the one-based month.
    pub const fn month(self) -> u8 {
        self.month
    }

    /// Returns the one-based day of month.
    pub const fn day(self) -> u8 {
        self.day
    }

    /// Returns whether the date's year is a leap year in this calendar.
    pub fn is_leap_year(self) -> bool {
        C::is_leap_year(self.year)
    }

    /// Returns the number of days in the date's month.
    pub fn days_in_month(self) -> u8 {
        C::days_in_month(self.year, self.month).unwrap_or(0)
    }

    /// Returns the one-based ordinal day of year.
    pub fn ordinal(self) -> u16 {
        let mut ordinal = u16::from(self.day);
        let mut month = 1;
        while month < self.month {
            ordinal += u16::from(C::days_in_month(self.year, month).unwrap_or(0));
            month += 1;
        }
        ordinal
    }

    /// Returns the weekday.
    pub fn weekday(self) -> Weekday {
        Weekday::from_monday_index(self.to_julian_day_number().0.rem_euclid(7) as u8)
    }

    /// Converts the date to its Julian Day Number.
    pub fn to_julian_day_number(self) -> JulianDayNumber {
        JulianDayNumber(C::to_julian_day_number(self.year, self.month, self.day))
    }

    /// Converts the same civil day into another proleptic calendar.
    pub fn convert<D: Calendar>(self) -> Result<Date<D>, Error> {
        Date::from_julian_day_number(self.to_julian_day_number())
    }

    /// Adds whole calendar days with overflow checking.
    pub fn checked_add_days(self, days: i64) -> Result<Self, Error> {
        let value = self
            .to_julian_day_number()
            .0
            .checked_add(days)
            .ok_or(Error::Overflow {
                operation: "adding days to date",
            })?;
        Self::from_julian_day_number(JulianDayNumber(value))
    }

    /// Returns the whole-day difference from another date in the same calendar.
    pub fn days_since(self, earlier: Self) -> Result<i64, Error> {
        self.to_julian_day_number()
            .0
            .checked_sub(earlier.to_julian_day_number().0)
            .ok_or(Error::Overflow {
                operation: "subtracting dates",
            })
    }

    fn checked_components(year: i128, month: i128, day: i128) -> Result<(i32, u8, u8), Error> {
        let year = i32::try_from(year).map_err(|_| Error::Overflow {
            operation: "converting Julian Day Number to year",
        })?;
        let month = u8::try_from(month).map_err(|_| Error::Overflow {
            operation: "converting Julian Day Number to month",
        })?;
        let day = u8::try_from(day).map_err(|_| Error::Overflow {
            operation: "converting Julian Day Number to day",
        })?;
        Ok((year, month, day))
    }
}

impl<C: Calendar> Copy for Date<C> {}

impl<C: Calendar> Clone for Date<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: Calendar> PartialEq for Date<C> {
    fn eq(&self, other: &Self) -> bool {
        self.year == other.year && self.month == other.month && self.day == other.day
    }
}

impl<C: Calendar> Eq for Date<C> {}

impl<C: Calendar> Hash for Date<C> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.year.hash(state);
        self.month.hash(state);
        self.day.hash(state);
    }
}

impl<C: Calendar> fmt::Debug for Date<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Date")
            .field("calendar", &C::NAME)
            .field("year", &self.year)
            .field("month", &self.month)
            .field("day", &self.day)
            .finish()
    }
}

/// A weekday with Monday as the first day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

impl Weekday {
    /// Returns the ISO weekday number from Monday=1 through Sunday=7.
    pub const fn iso_number(self) -> u8 {
        match self {
            Self::Monday => 1,
            Self::Tuesday => 2,
            Self::Wednesday => 3,
            Self::Thursday => 4,
            Self::Friday => 5,
            Self::Saturday => 6,
            Self::Sunday => 7,
        }
    }

    fn from_monday_index(index: u8) -> Self {
        match index {
            0 => Self::Monday,
            1 => Self::Tuesday,
            2 => Self::Wednesday,
            3 => Self::Thursday,
            4 => Self::Friday,
            5 => Self::Saturday,
            _ => Self::Sunday,
        }
    }
}
