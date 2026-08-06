use super::{
    Calendar, Date, DateTime, Duration, EarthOrientation, EarthOrientationTable, Error, Gps,
    Gregorian, Instant, JulianDate, LeapSeconds, Tai, TimeOfDay, TimeScale, Tt, Ut1, Utc,
};

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Marker for a time context without Earth-orientation observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoEarthOrientation;

/// A source of numerical coordinates in target time scale `S`.
///
/// This trait is sealed. Implementations identify contexts that can prove a
/// target scale conversion; callers use
/// [`Instant::from_instant`](super::Instant::from_instant) and
/// [`JulianDate::from_instant`](super::JulianDate::from_instant).
pub trait TimeScaleModel<S: TimeScale>: sealed::Sealed {
    /// Checks that the model covers a physical instant.
    #[doc(hidden)]
    fn validate_instant<From: TimeScale>(&self, instant: Instant<From>) -> Result<(), Error>;

    /// Computes the target scale's two-part Julian Date.
    #[doc(hidden)]
    fn julian_date_at<From: TimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<S>, Error>;
}

/// Explicit context for resolving and representing typed time scales.
///
/// The core context owns leap-second policy. Its second type parameter records
/// whether Earth-orientation observations are present, so UT1 conversion is
/// unavailable at compile time on [`NoEarthOrientation`].
#[derive(Debug, Clone, Copy)]
pub struct TimeContext<'a, E = NoEarthOrientation> {
    leap_seconds: LeapSeconds<'a>,
    earth_orientation: E,
}

impl TimeContext<'static, NoEarthOrientation> {
    /// Constructs a context using bundled IERS Bulletin C 72 leap-second data.
    pub const fn builtin() -> Self {
        Self::new(LeapSeconds::builtin())
    }
}

impl<'a> TimeContext<'a, NoEarthOrientation> {
    /// Constructs a context with explicit leap-second data.
    pub const fn new(leap_seconds: LeapSeconds<'a>) -> Self {
        Self {
            leap_seconds,
            earth_orientation: NoEarthOrientation,
        }
    }

    /// Adds a validated Earth-orientation table to this context's type.
    pub const fn with_earth_orientation<'e>(
        self,
        earth_orientation: EarthOrientationTable<'e>,
    ) -> TimeContext<'a, EarthOrientationTable<'e>> {
        TimeContext {
            leap_seconds: self.leap_seconds,
            earth_orientation,
        }
    }
}

impl<'a, E> TimeContext<'a, E> {
    /// Returns the context's leap-second data.
    pub const fn leap_seconds(&self) -> LeapSeconds<'a> {
        self.leap_seconds
    }

    /// Resolves a calendar label to a physical instant.
    ///
    /// UTC uses this context's leap-second data. TAI, TT, and GPS use their
    /// exact fixed offsets from TAI. Other scales require a dedicated model.
    pub fn resolve<C: Calendar, S: TimeScale>(
        &self,
        value: DateTime<C, S>,
    ) -> Result<Instant<S>, Error> {
        if S::LEAP_SECOND_LABELS {
            let utc = DateTime::<C, Utc>::new(value.date(), value.time())?;
            return Ok(self.leap_seconds.resolve(utc)?.retag());
        }

        let offset = S::TAI_OFFSET_NANOSECONDS.ok_or(Error::UnsupportedScale {
            backend: "core time context",
            operation: "resolving calendar label",
            scale: S::NAME,
        })?;
        let date: Date<Gregorian> = value.date().convert()?;
        let nominal = Self::nominal_nanoseconds(date, value.time())?;
        let tai_nanoseconds = nominal.checked_sub(offset).ok_or(Error::Overflow {
            operation: "applying fixed time-scale offset",
        })?;
        Ok(Instant::from_tai_nanoseconds(tai_nanoseconds))
    }

    /// Represents a physical instant as a calendar label in its typed scale.
    ///
    /// UTC uses this context's leap-second data. TAI, TT, and GPS use their
    /// exact fixed offsets from TAI. Model-dependent calendar coordinates are
    /// provided by their dedicated adapter.
    pub fn represent<C: Calendar, S: TimeScale>(
        &self,
        instant: Instant<S>,
    ) -> Result<DateTime<C, S>, Error> {
        if S::LEAP_SECOND_LABELS {
            let utc = self.leap_seconds.represent::<C>(instant.retag())?;
            return DateTime::new(utc.date(), utc.time());
        }

        let offset = S::TAI_OFFSET_NANOSECONDS.ok_or(Error::UnsupportedScale {
            backend: "core time context",
            operation: "representing calendar label",
            scale: S::NAME,
        })?;
        let nominal = instant
            .tai_nanoseconds_since_1900()
            .checked_add(offset)
            .ok_or(Error::Overflow {
                operation: "applying fixed time-scale offset",
            })?;
        Self::label_from_nominal(nominal)
    }

    fn uniform_julian_date<From: TimeScale, Target: TimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<Target>, Error> {
        JulianDate::from_datetime(self.represent::<Gregorian, Target>(instant.retag())?)
    }

    fn nominal_nanoseconds(date: Date<Gregorian>, time: TimeOfDay) -> Result<i128, Error> {
        let epoch = Date::<Gregorian>::from_valid_components(1900, 1, 1);
        let days = date.days_since(epoch)?;
        let day_nanoseconds = i128::from(days)
            .checked_mul(Duration::NANOSECONDS_PER_DAY)
            .ok_or(Error::Overflow {
                operation: "converting calendar date to nominal nanoseconds",
            })?;
        day_nanoseconds
            .checked_add(i128::from(time.nanoseconds_since_midnight()))
            .ok_or(Error::Overflow {
                operation: "converting calendar label to nominal nanoseconds",
            })
    }

    fn label_from_nominal<C: Calendar, S: TimeScale>(
        nominal_nanoseconds: i128,
    ) -> Result<DateTime<C, S>, Error> {
        let days = nominal_nanoseconds.div_euclid(Duration::NANOSECONDS_PER_DAY);
        let days = i64::try_from(days).map_err(|_| Error::Overflow {
            operation: "converting nominal nanoseconds to a date",
        })?;
        let within_day = nominal_nanoseconds.rem_euclid(Duration::NANOSECONDS_PER_DAY);
        let within_day = u64::try_from(within_day).map_err(|_| Error::Overflow {
            operation: "converting nominal nanoseconds to time of day",
        })?;
        let epoch = Date::<Gregorian>::from_valid_components(1900, 1, 1);
        let date = epoch.checked_add_days(days)?.convert()?;
        let time = TimeOfDay::from_nanoseconds_since_midnight(within_day)?;
        DateTime::new(date, time)
    }
}

impl<'a, 'e> TimeContext<'a, EarthOrientationTable<'e>> {
    /// Returns the context's Earth-orientation table.
    pub const fn earth_orientation(&self) -> EarthOrientationTable<'e> {
        self.earth_orientation
    }

    /// Resolves linearly interpolated Earth-orientation values at an instant.
    pub fn earth_orientation_at<S: TimeScale>(
        &self,
        instant: Instant<S>,
    ) -> Result<EarthOrientation<S>, Error> {
        self.earth_orientation.at(instant, self.leap_seconds)
    }

    pub(crate) fn julian_date_from_orientation<From: TimeScale>(
        &self,
        instant: Instant<From>,
        orientation: EarthOrientation<From>,
    ) -> Result<JulianDate<Ut1>, Error> {
        let tai_minus_utc = self.leap_seconds.offset(instant.retag::<Tai>())?;
        let ut1_minus_tai = orientation
            .ut1_minus_utc()
            .as_duration()
            .checked_sub(tai_minus_utc)?;
        let tai = self.uniform_julian_date::<From, Tai>(instant)?;
        let shifted = tai.checked_add_duration(ut1_minus_tai)?;
        let (first, second) = shifted.parts();
        JulianDate::from_parts(first, second)
    }
}

impl<E> sealed::Sealed for TimeContext<'_, E> {}

impl<E> TimeScaleModel<Tai> for TimeContext<'_, E> {
    fn validate_instant<From: TimeScale>(&self, _instant: Instant<From>) -> Result<(), Error> {
        Ok(())
    }

    fn julian_date_at<From: TimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<Tai>, Error> {
        self.uniform_julian_date(instant)
    }
}

impl<E> TimeScaleModel<Tt> for TimeContext<'_, E> {
    fn validate_instant<From: TimeScale>(&self, _instant: Instant<From>) -> Result<(), Error> {
        Ok(())
    }

    fn julian_date_at<From: TimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<Tt>, Error> {
        self.uniform_julian_date(instant)
    }
}

impl<E> TimeScaleModel<Gps> for TimeContext<'_, E> {
    fn validate_instant<From: TimeScale>(&self, _instant: Instant<From>) -> Result<(), Error> {
        Ok(())
    }

    fn julian_date_at<From: TimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<Gps>, Error> {
        self.uniform_julian_date(instant)
    }
}

impl<E> TimeScaleModel<Utc> for TimeContext<'_, E> {
    fn validate_instant<From: TimeScale>(&self, instant: Instant<From>) -> Result<(), Error> {
        self.leap_seconds.offset(instant.retag::<Tai>()).map(|_| ())
    }

    fn julian_date_at<From: TimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<Utc>, Error> {
        self.leap_seconds.julian_date(instant.retag())
    }
}

impl TimeScaleModel<Ut1> for TimeContext<'_, EarthOrientationTable<'_>> {
    fn validate_instant<From: TimeScale>(&self, instant: Instant<From>) -> Result<(), Error> {
        self.earth_orientation_at(instant).map(|_| ())
    }

    fn julian_date_at<From: TimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<Ut1>, Error> {
        let orientation = self.earth_orientation_at(instant)?;
        self.julian_date_from_orientation(instant, orientation)
    }
}
