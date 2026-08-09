use core::{
    f64::consts::{PI, TAU},
    fmt,
};

use libm::round;

use crate::{
    frame::{
        EclipticDirectionAt, EclipticLatitude, EclipticLongitude, EquatorialDirectionAt, Gcrs,
        SiderealTimeSolution, TrueEclipticEquinoxOfDate, TrueEquatorEquinoxOfDate,
    },
    math::{Angle, Declination, HourAngle, Length, Longitude, RightAscension},
    time::{Duration, Instant, JulianDate, TimeOfDay, TimeScale, Ut1},
};

use super::{Error, GeocentricApparentPlace, ReceptionLightTime, SolarLightDeflection};

/// The geocentric apparent place of the Sun at one reception epoch.
///
/// This solar-specific view preserves the general finite-target apparent
/// place, including converged reception light time, annual aberration, and the
/// explicit decision not to deflect the Sun by its own point-mass model. It
/// excludes station parallax, atmospheric refraction, solar-limb geometry, and
/// Shapiro delay.
pub struct SolarApparentPlace<S: TimeScale> {
    geocentric: GeocentricApparentPlace<S>,
}

impl<S: TimeScale> SolarApparentPlace<S> {
    pub(super) const fn new(geocentric: GeocentricApparentPlace<S>) -> Self {
        Self { geocentric }
    }

    /// Returns the general finite-target apparent place.
    pub const fn geocentric(self) -> GeocentricApparentPlace<S> {
        self.geocentric
    }

    /// Returns the complete dual-epoch reception light-time solution.
    pub const fn reception_light_time(self) -> ReceptionLightTime<S> {
        self.geocentric.reception_light_time()
    }

    /// Returns the geocentric reception epoch.
    pub const fn reception_epoch(self) -> Instant<S> {
        self.geocentric.reception_epoch()
    }

    /// Returns the retarded solar emission epoch.
    pub const fn emission_epoch(self) -> Instant<S> {
        self.geocentric.emission_epoch()
    }

    /// Returns the converged one-way solar light time.
    pub const fn light_time(self) -> Duration {
        self.geocentric.light_time()
    }

    /// Returns the epoch-bound apparent GCRS direction.
    pub const fn gcrs_direction(self) -> EquatorialDirectionAt<Gcrs, S> {
        self.geocentric.gcrs_direction()
    }

    /// Returns the apparent direction on true equator and equinox of date axes.
    pub const fn true_equatorial(self) -> EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S> {
        self.geocentric.true_equatorial()
    }

    /// Returns the apparent direction on true ecliptic and equinox of date axes.
    pub const fn true_ecliptic(self) -> EclipticDirectionAt<TrueEclipticEquinoxOfDate, S> {
        self.geocentric.true_ecliptic()
    }

    /// Returns the apparent geocentric right ascension of the Sun.
    pub const fn right_ascension(self) -> RightAscension {
        self.geocentric.right_ascension()
    }

    /// Returns the apparent geocentric declination of the Sun.
    pub const fn declination(self) -> Declination {
        self.geocentric.declination()
    }

    /// Returns the apparent geocentric solar longitude.
    pub const fn longitude(self) -> EclipticLongitude {
        self.geocentric.longitude()
    }

    /// Returns the apparent geocentric solar latitude.
    pub const fn latitude(self) -> EclipticLatitude {
        self.geocentric.latitude()
    }

    /// Returns the Sun-at-emission to Earth-at-reception distance.
    pub const fn distance(self) -> Length {
        self.geocentric.distance()
    }

    /// Returns the explicit no-self-deflection diagnostics for the Sun.
    pub const fn solar_light_deflection(self) -> SolarLightDeflection<S> {
        self.geocentric.solar_light_deflection()
    }

    /// Returns the number of completed light-time iterations.
    pub const fn iterations(self) -> u32 {
        self.geocentric.iterations()
    }

    /// Returns the final absolute light-time fixed-point residual.
    pub const fn light_time_residual(self) -> Duration {
        self.geocentric.light_time_residual()
    }
}

impl<S: TimeScale> Copy for SolarApparentPlace<S> {}

impl<S: TimeScale> Clone for SolarApparentPlace<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for SolarApparentPlace<S> {
    fn eq(&self, other: &Self) -> bool {
        self.geocentric == other.geocentric
    }
}

impl<S: TimeScale> fmt::Debug for SolarApparentPlace<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SolarApparentPlace")
            .field("geocentric", &self.geocentric)
            .finish()
    }
}

/// Mean solar clock time at one longitude.
///
/// This is a nominal 24-hour clock reading driven by UT1 and longitude. It is
/// not UTC, a time zone, or a civil date-time label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeanSolarTime(HourAngle);

impl MeanSolarTime {
    fn from_hour_angle(value: HourAngle) -> Self {
        Self(value)
    }

    fn time_of_day_from_hour_angle(value: HourAngle) -> TimeOfDay {
        let nanoseconds =
            round(value.as_radians() * Duration::NANOSECONDS_PER_DAY as f64 / TAU) as i128;
        let wrapped = nanoseconds.rem_euclid(Duration::NANOSECONDS_PER_DAY) as u64;
        TimeOfDay::from_valid_nanoseconds_since_midnight(wrapped)
    }

    /// Returns the solar clock reading as an angle in `[0, 2π)`.
    pub const fn as_hour_angle(self) -> HourAngle {
        self.0
    }

    /// Returns the solar clock reading in decimal hours in `[0, 24)`.
    pub fn as_decimal_hours(self) -> f64 {
        self.0.as_hours()
    }

    /// Returns the nominal 24-hour time-of-day reading, rounded to one nanosecond.
    pub fn as_time_of_day(self) -> TimeOfDay {
        Self::time_of_day_from_hour_angle(self.0)
    }
}

/// Apparent solar clock time at one longitude.
///
/// The reading is `12h +` the apparent Sun's local hour angle. It is not a
/// uniform time scale, a time zone, or a civil date-time label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApparentSolarTime(HourAngle);

impl ApparentSolarTime {
    fn from_hour_angle(value: HourAngle) -> Self {
        Self(value)
    }

    /// Returns the solar clock reading as an angle in `[0, 2π)`.
    pub const fn as_hour_angle(self) -> HourAngle {
        self.0
    }

    /// Returns the solar clock reading in decimal hours in `[0, 24)`.
    pub fn as_decimal_hours(self) -> f64 {
        self.0.as_hours()
    }

    /// Returns the nominal 24-hour time-of-day reading, rounded to one nanosecond.
    pub fn as_time_of_day(self) -> TimeOfDay {
        MeanSolarTime::time_of_day_from_hour_angle(self.0)
    }
}

/// Apparent solar time minus mean solar time.
///
/// Positive values mean that the apparent Sun is ahead of the fictitious mean
/// Sun. The value is represented by the shortest signed clock difference in
/// `(-12h, 12h]` and is independent of longitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EquationOfTime(Duration);

impl EquationOfTime {
    fn from_duration(value: Duration) -> Self {
        Self(value)
    }

    /// Returns the signed physical duration.
    pub const fn duration(self) -> Duration {
        self.0
    }

    /// Returns the signed value in SI seconds.
    pub fn as_seconds(self) -> f64 {
        self.0.as_seconds_f64()
    }

    /// Returns the signed value in minutes.
    pub fn as_minutes(self) -> f64 {
        self.as_seconds() / 60.0
    }
}

/// Mean and apparent solar clock readings at one east-positive longitude.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarTimeAtLongitude {
    longitude: Longitude,
    mean_solar_time: MeanSolarTime,
    apparent_solar_time: ApparentSolarTime,
    equation_of_time: EquationOfTime,
}

impl SolarTimeAtLongitude {
    fn from_greenwich(greenwich: Self, longitude: Longitude) -> Result<Self, Error> {
        let offset = longitude.as_radians();
        Ok(Self {
            longitude,
            mean_solar_time: MeanSolarTime::from_hour_angle(HourAngle::wrap_radians(
                greenwich.mean_solar_time.as_hour_angle().as_radians() + offset,
            )?),
            apparent_solar_time: ApparentSolarTime::from_hour_angle(HourAngle::wrap_radians(
                greenwich.apparent_solar_time.as_hour_angle().as_radians() + offset,
            )?),
            equation_of_time: greenwich.equation_of_time,
        })
    }

    /// Returns the east-positive longitude of both clock readings.
    pub const fn longitude(self) -> Longitude {
        self.longitude
    }

    /// Returns local mean solar time.
    pub const fn mean_solar_time(self) -> MeanSolarTime {
        self.mean_solar_time
    }

    /// Returns local apparent solar time.
    pub const fn apparent_solar_time(self) -> ApparentSolarTime {
        self.apparent_solar_time
    }

    /// Returns apparent solar time minus mean solar time.
    pub const fn equation_of_time(self) -> EquationOfTime {
        self.equation_of_time
    }
}

/// A coherent apparent-Sun, UT1, sidereal-time, and solar-time solution.
pub struct SolarTimeSolution<S: TimeScale> {
    apparent_sun: SolarApparentPlace<S>,
    sidereal_time: SiderealTimeSolution<S>,
    greenwich: SolarTimeAtLongitude,
}

impl<S: TimeScale> SolarTimeSolution<S> {
    /// Exact conventions used to relate apparent and mean solar time.
    pub const MODEL: &'static str =
        "apparent geocentric Sun, IAU 2006/2000A GAST, and UT1 mean solar time";

    pub(super) fn new(
        apparent_sun: SolarApparentPlace<S>,
        sidereal_time: SiderealTimeSolution<S>,
    ) -> Result<Self, Error> {
        let universal_time = sidereal_time.universal_time();
        let mean_angle =
            HourAngle::wrap_radians(universal_time.nominal_fraction_since_midnight() * TAU)?;
        let apparent_angle = HourAngle::wrap_radians(
            sidereal_time
                .greenwich_apparent_sidereal_time()
                .as_radians()
                - apparent_sun.right_ascension().as_radians()
                + PI,
        )?;
        let difference_radians = Angle::wrap_signed(
            apparent_angle.as_radians() - mean_angle.as_radians(),
            "equation of time",
        )?;
        let difference_seconds = difference_radians * Duration::NANOSECONDS_PER_DAY as f64
            / Duration::NANOSECONDS_PER_SECOND as f64
            / TAU;
        let equation_of_time =
            EquationOfTime::from_duration(Duration::from_seconds_f64(difference_seconds)?);
        let greenwich = SolarTimeAtLongitude {
            longitude: Longitude::try_from_radians(0.0)?,
            mean_solar_time: MeanSolarTime::from_hour_angle(mean_angle),
            apparent_solar_time: ApparentSolarTime::from_hour_angle(apparent_angle),
            equation_of_time,
        };

        Ok(Self {
            apparent_sun,
            sidereal_time,
            greenwich,
        })
    }

    /// Returns the physical epoch represented by the complete solution.
    pub const fn epoch(self) -> Instant<S> {
        self.apparent_sun.reception_epoch()
    }

    /// Returns the apparent geocentric Sun used by the calculation.
    pub const fn apparent_sun(self) -> SolarApparentPlace<S> {
        self.apparent_sun
    }

    /// Returns the coherent UT1, GAST, and Earth-rotation solution.
    pub const fn sidereal_time(self) -> SiderealTimeSolution<S> {
        self.sidereal_time
    }

    /// Returns the two-part UT1 date used for mean solar time.
    pub const fn universal_time(self) -> JulianDate<Ut1> {
        self.sidereal_time.universal_time()
    }

    /// Returns mean and apparent solar time at Greenwich.
    pub const fn greenwich(self) -> SolarTimeAtLongitude {
        self.greenwich
    }

    /// Returns apparent solar time minus mean solar time.
    pub const fn equation_of_time(self) -> EquationOfTime {
        self.greenwich.equation_of_time()
    }

    /// Returns mean and apparent solar time at an east-positive longitude.
    pub fn at_longitude(self, longitude: Longitude) -> Result<SolarTimeAtLongitude, Error> {
        SolarTimeAtLongitude::from_greenwich(self.greenwich, longitude)
    }
}

impl<S: TimeScale> Copy for SolarTimeSolution<S> {}

impl<S: TimeScale> Clone for SolarTimeSolution<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> fmt::Debug for SolarTimeSolution<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SolarTimeSolution")
            .field("apparent_sun", &self.apparent_sun)
            .field("sidereal_time", &self.sidereal_time)
            .field("greenwich", &self.greenwich)
            .finish()
    }
}
