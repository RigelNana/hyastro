use core::{fmt, marker::PhantomData};

use libm::{floor, round};

use super::{
    Calendar, Date, DateTime, Duration, Error, Instant, JulianDayNumber, TimeOfDay, TimeScale,
    TimeScaleModel, Tt,
};

/// A two-part Julian Date carrying an explicit time scale.
pub struct JulianDate<S: TimeScale> {
    first: f64,
    second: f64,
    scale: PhantomData<S>,
}

impl<S: TimeScale> JulianDate<S> {
    /// Julian Date of J2000.0 in TT.
    pub const J2000_VALUE: f64 = 2_451_545.0;

    /// Constructs a Julian Date from any finite two-part split.
    pub fn from_parts(first: f64, second: f64) -> Result<Self, Error> {
        Error::ensure_finite("Julian Date first part", first)?;
        Error::ensure_finite("Julian Date second part", second)?;
        Error::ensure_finite("Julian Date sum", first + second)?;
        Ok(Self {
            first,
            second,
            scale: PhantomData,
        })
    }
    /// Computes this target scale's Julian Date through an explicit model.
    pub fn from_instant<From, Model>(instant: Instant<From>, model: &Model) -> Result<Self, Error>
    where
        From: TimeScale,
        Model: TimeScaleModel<S>,
    {
        model.julian_date_at(instant)
    }

    /// Constructs a split preserving a small offset from J2000.0.
    pub fn from_j2000_offset_days(offset: f64) -> Result<Self, Error> {
        Self::from_parts(Self::J2000_VALUE, offset)
    }

    /// Constructs a Julian Date from a context-free uniform-scale label.
    pub fn from_datetime<C: Calendar>(value: DateTime<C, S>) -> Result<Self, Error> {
        if !S::UNIFORM_DAYS {
            return Err(Error::ContextRequired {
                scale: S::NAME,
                operation: "converting a date-time label to Julian Date",
            });
        }
        if value.time().is_leap_second() {
            return Err(Error::LeapSecondRequiresContext);
        }
        let first = value.date().to_julian_day_number().value() as f64 - 0.5;
        let second =
            value.time().nanoseconds_since_midnight() as f64 / Duration::NANOSECONDS_PER_DAY as f64;
        Self::from_parts(first, second)
    }

    /// Returns the original two parts without changing their split.
    pub const fn parts(self) -> (f64, f64) {
        (self.first, self.second)
    }

    /// Returns a single floating-point value with explicit precision loss.
    pub fn as_f64_lossy(self) -> f64 {
        self.first + self.second
    }

    /// Returns an equivalent split with an integer first part and small remainder.
    pub fn normalized(self) -> Result<Self, Error> {
        let (sum, error) = Self::two_sum(self.first, self.second);
        let first = round(sum);
        let second = (sum - first) + error;
        Self::from_parts(first, second)
    }

    /// Adds an exact duration, rounding only the sub-day conversion to `f64`.
    pub fn checked_add_duration(self, duration: Duration) -> Result<Self, Error> {
        let whole_days = duration
            .as_nanoseconds()
            .div_euclid(Duration::NANOSECONDS_PER_DAY);
        const MAX_EXACT_F64_INTEGER: i128 = 9_007_199_254_740_992;
        if !(-MAX_EXACT_F64_INTEGER..=MAX_EXACT_F64_INTEGER).contains(&whole_days) {
            return Err(Error::Overflow {
                operation: "adding an inexact whole-day count to Julian Date",
            });
        }
        let remainder = duration
            .as_nanoseconds()
            .rem_euclid(Duration::NANOSECONDS_PER_DAY);
        let fractional_day = remainder as f64 / Duration::NANOSECONDS_PER_DAY as f64;
        Self::from_parts(self.first + whole_days as f64, self.second + fractional_day)?.normalized()
    }

    /// Returns a nanosecond-rounded duration from another date in the same scale.
    pub fn duration_since_rounded(self, earlier: Self) -> Result<Duration, Error> {
        let days = (self.first - earlier.first) + (self.second - earlier.second);
        let nanoseconds = days * Duration::NANOSECONDS_PER_DAY as f64;
        Error::ensure_finite("Julian Date difference", nanoseconds)?;
        if nanoseconds < i128::MIN as f64 || nanoseconds > i128::MAX as f64 {
            return Err(Error::Overflow {
                operation: "converting Julian Date difference to duration",
            });
        }
        Ok(Duration::from_nanoseconds(round(nanoseconds) as i128))
    }

    /// Converts a uniform-scale Julian Date to a calendar date-time label.
    pub fn to_datetime<C: Calendar>(self) -> Result<DateTime<C, S>, Error> {
        if !S::UNIFORM_DAYS {
            return Err(Error::ContextRequired {
                scale: S::NAME,
                operation: "converting Julian Date to a date-time label",
            });
        }

        let (sum, error) = Self::two_sum(self.first + 0.5, self.second);
        let mut day_number = floor(sum);
        let mut fraction = (sum - day_number) + error;
        if fraction < 0.0 {
            day_number -= 1.0;
            fraction += 1.0;
        } else if fraction >= 1.0 {
            day_number += 1.0;
            fraction -= 1.0;
        }
        if day_number < i64::MIN as f64 || day_number > i64::MAX as f64 {
            return Err(Error::Overflow {
                operation: "converting Julian Date to Julian Day Number",
            });
        }

        let mut day_number = day_number as i64;
        let mut nanoseconds = round(fraction * Duration::NANOSECONDS_PER_DAY as f64) as i128;
        if nanoseconds == Duration::NANOSECONDS_PER_DAY {
            day_number = day_number.checked_add(1).ok_or(Error::Overflow {
                operation: "carrying rounded Julian Date into next day",
            })?;
            nanoseconds = 0;
        }
        let nanoseconds = u64::try_from(nanoseconds).map_err(|_| Error::Overflow {
            operation: "converting Julian Date fraction to nanoseconds",
        })?;
        let date = Date::from_julian_day_number(JulianDayNumber::new(day_number))?;
        let time = TimeOfDay::from_nanoseconds_since_midnight(nanoseconds)?;
        DateTime::new(date, time)
    }

    /// Converts to a two-part Modified Julian Date without collapsing the split.
    pub fn to_modified(self) -> Result<ModifiedJulianDate<S>, Error> {
        ModifiedJulianDate::from_parts(self.first - 2_400_000.5, self.second)
    }

    fn two_sum(left: f64, right: f64) -> (f64, f64) {
        let sum = left + right;
        let right_virtual = sum - left;
        let error = (left - (sum - right_virtual)) + (right - right_virtual);
        (sum, error)
    }
}

impl<S: TimeScale> Copy for JulianDate<S> {}

impl<S: TimeScale> Clone for JulianDate<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for JulianDate<S> {
    fn eq(&self, other: &Self) -> bool {
        self.first == other.first && self.second == other.second
    }
}

impl<S: TimeScale> fmt::Debug for JulianDate<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JulianDate")
            .field("scale", &S::NAME)
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

/// A two-part Modified Julian Date carrying an explicit time scale.
pub struct ModifiedJulianDate<S: TimeScale> {
    first: f64,
    second: f64,
    scale: PhantomData<S>,
}

impl<S: TimeScale> ModifiedJulianDate<S> {
    /// Constructs a Modified Julian Date from any finite two-part split.
    pub fn from_parts(first: f64, second: f64) -> Result<Self, Error> {
        Error::ensure_finite("Modified Julian Date first part", first)?;
        Error::ensure_finite("Modified Julian Date second part", second)?;
        Error::ensure_finite("Modified Julian Date sum", first + second)?;
        Ok(Self {
            first,
            second,
            scale: PhantomData,
        })
    }

    /// Returns the original two parts without changing their split.
    pub const fn parts(self) -> (f64, f64) {
        (self.first, self.second)
    }

    /// Returns a single floating-point value with explicit precision loss.
    pub fn as_f64_lossy(self) -> f64 {
        self.first + self.second
    }

    /// Converts to a two-part Julian Date without collapsing the split.
    pub fn to_julian(self) -> Result<JulianDate<S>, Error> {
        JulianDate::from_parts(self.first + 2_400_000.5, self.second)
    }
}

impl<S: TimeScale> Copy for ModifiedJulianDate<S> {}

impl<S: TimeScale> Clone for ModifiedJulianDate<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for ModifiedJulianDate<S> {
    fn eq(&self, other: &Self) -> bool {
        self.first == other.first && self.second == other.second
    }
}

impl<S: TimeScale> fmt::Debug for ModifiedJulianDate<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModifiedJulianDate")
            .field("scale", &S::NAME)
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

/// A Julian epoch based on 365.25-day years relative to J2000.0 TT.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct JulianEpoch(f64);

impl JulianEpoch {
    /// The standard J2000.0 epoch.
    pub const J2000: Self = Self(2000.0);
    /// The Gaia reference epoch J2016.0.
    pub const J2016: Self = Self(2016.0);

    /// Constructs a finite Julian epoch value.
    pub fn new(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("Julian epoch", value).map(Self)
    }

    /// Constructs a Julian epoch from a TT Julian Date.
    pub fn from_tt(value: JulianDate<Tt>) -> Result<Self, Error> {
        Self::new(2000.0 + (value.as_f64_lossy() - JulianDate::<Tt>::J2000_VALUE) / 365.25)
    }

    /// Converts the epoch to a split TT Julian Date.
    pub fn to_tt(self) -> Result<JulianDate<Tt>, Error> {
        JulianDate::from_j2000_offset_days((self.0 - 2000.0) * 365.25)
    }

    /// Returns the conventional epoch number.
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// A Besselian epoch using the conventional B1900 tropical-year definition.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct BesselianEpoch(f64);

impl BesselianEpoch {
    /// The standard B1950.0 epoch.
    pub const B1950: Self = Self(1950.0);

    /// Constructs a finite Besselian epoch value.
    pub fn new(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("Besselian epoch", value).map(Self)
    }

    /// Constructs a Besselian epoch from a TT Julian Date.
    pub fn from_tt(value: JulianDate<Tt>) -> Result<Self, Error> {
        Self::new(1900.0 + (value.as_f64_lossy() - 2_415_020.313_52) / 365.242_198_781)
    }

    /// Converts the epoch to a split TT Julian Date.
    pub fn to_tt(self) -> Result<JulianDate<Tt>, Error> {
        JulianDate::from_parts(2_415_020.313_52, (self.0 - 1900.0) * 365.242_198_781)
    }

    /// Returns the conventional epoch number.
    pub const fn value(self) -> f64 {
        self.0
    }
}
