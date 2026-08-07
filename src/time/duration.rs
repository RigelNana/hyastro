use crate::constants::time::{
    NANOSECONDS_PER_DAY, NANOSECONDS_PER_JULIAN_YEAR, NANOSECONDS_PER_MICROSECOND,
    NANOSECONDS_PER_MILLISECOND, NANOSECONDS_PER_SECOND,
};

use super::Error;

/// A signed physical duration stored exactly as integer nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
    nanoseconds: i128,
}

impl Duration {
    /// Number of nanoseconds in one SI second.
    pub const NANOSECONDS_PER_SECOND: i128 = NANOSECONDS_PER_SECOND;
    /// Number of nanoseconds in one nominal 86,400-second day.
    pub const NANOSECONDS_PER_DAY: i128 = NANOSECONDS_PER_DAY;
    /// Number of nanoseconds in one 365.25-day Julian year.
    pub const NANOSECONDS_PER_JULIAN_YEAR: i128 = NANOSECONDS_PER_JULIAN_YEAR;
    /// A zero-length duration.
    pub const ZERO: Self = Self { nanoseconds: 0 };

    /// Constructs a duration from an exact signed nanosecond count.
    pub const fn from_nanoseconds(nanoseconds: i128) -> Self {
        Self { nanoseconds }
    }

    /// Constructs a duration from whole SI seconds.
    pub fn from_seconds(seconds: i64) -> Result<Self, Error> {
        i128::from(seconds)
            .checked_mul(Self::NANOSECONDS_PER_SECOND)
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "constructing duration from seconds",
            })
    }
    /// Constructs a duration from fractional SI seconds, rounded to the nearest nanosecond.
    pub fn from_seconds_f64(seconds: f64) -> Result<Self, Error> {
        Error::ensure_finite("duration seconds", seconds)?;
        let nanoseconds = libm::round(seconds * Self::NANOSECONDS_PER_SECOND as f64);
        Error::ensure_finite("duration nanoseconds", nanoseconds)?;
        let exclusive_upper_bound = -(i128::MIN as f64);
        if nanoseconds < i128::MIN as f64 || nanoseconds >= exclusive_upper_bound {
            return Err(Error::Overflow {
                operation: "constructing duration from fractional seconds",
            });
        }
        Ok(Self::from_nanoseconds(nanoseconds as i128))
    }

    /// Constructs a duration from whole milliseconds.
    pub fn from_milliseconds(milliseconds: i64) -> Result<Self, Error> {
        i128::from(milliseconds)
            .checked_mul(NANOSECONDS_PER_MILLISECOND)
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "constructing duration from milliseconds",
            })
    }

    /// Constructs a duration from whole microseconds.
    pub fn from_microseconds(microseconds: i64) -> Result<Self, Error> {
        i128::from(microseconds)
            .checked_mul(NANOSECONDS_PER_MICROSECOND)
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "constructing duration from microseconds",
            })
    }

    /// Constructs a duration from nominal 86,400-second days.
    pub fn from_days(days: i64) -> Result<Self, Error> {
        i128::from(days)
            .checked_mul(Self::NANOSECONDS_PER_DAY)
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "constructing duration from days",
            })
    }

    /// Constructs a duration from whole 365.25-day Julian years.
    pub fn from_julian_years(years: i64) -> Result<Self, Error> {
        i128::from(years)
            .checked_mul(Self::NANOSECONDS_PER_JULIAN_YEAR)
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "constructing duration from Julian years",
            })
    }

    /// Returns the exact signed nanosecond count.
    pub const fn as_nanoseconds(self) -> i128 {
        self.nanoseconds
    }

    /// Returns the duration as floating-point SI seconds.
    pub fn as_seconds_f64(self) -> f64 {
        self.nanoseconds as f64 / Self::NANOSECONDS_PER_SECOND as f64
    }

    /// Splits the duration into floor seconds and a non-negative nanosecond remainder.
    pub fn split_seconds(self) -> (i128, u32) {
        let seconds = self.nanoseconds.div_euclid(Self::NANOSECONDS_PER_SECOND);
        let nanoseconds = self.nanoseconds.rem_euclid(Self::NANOSECONDS_PER_SECOND) as u32;
        (seconds, nanoseconds)
    }

    /// Adds another duration with overflow checking.
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        self.nanoseconds
            .checked_add(rhs.nanoseconds)
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "adding durations",
            })
    }

    /// Subtracts another duration with overflow checking.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        self.nanoseconds
            .checked_sub(rhs.nanoseconds)
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "subtracting durations",
            })
    }

    /// Negates the duration with overflow checking.
    pub fn checked_neg(self) -> Result<Self, Error> {
        self.nanoseconds
            .checked_neg()
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "negating duration",
            })
    }

    /// Returns the absolute duration with overflow checking.
    pub fn checked_abs(self) -> Result<Self, Error> {
        self.nanoseconds
            .checked_abs()
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "taking duration absolute value",
            })
    }
}
