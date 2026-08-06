use core::f64::consts::PI;

use crate::math::Angle;

use super::{Duration, Error, Instant, LeapSeconds, Tai, TimeScale, Utc};

const RADIANS_PER_ARCSECOND: f64 = PI / (180.0 * 3_600.0);

/// The observed difference `UT1−UTC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ut1MinusUtc(Duration);

impl Ut1MinusUtc {
    /// Constructs `UT1−UTC` from fractional SI seconds.
    ///
    /// Values outside `[-1 s, +1 s]` are rejected because UTC is steered to
    /// keep the magnitude below one second.
    pub fn from_seconds(seconds: f64) -> Result<Self, Error> {
        let duration = Duration::from_seconds_f64(seconds)?;
        let nanoseconds = duration.as_nanoseconds();
        if !(-Duration::NANOSECONDS_PER_SECOND..=Duration::NANOSECONDS_PER_SECOND)
            .contains(&nanoseconds)
        {
            return Err(Error::component(
                "UT1−UTC nanoseconds",
                nanoseconds,
                -Duration::NANOSECONDS_PER_SECOND,
                Duration::NANOSECONDS_PER_SECOND,
            ));
        }
        Ok(Self(duration))
    }

    /// Constructs `UT1−UTC` from an exact duration.
    pub fn from_duration(duration: Duration) -> Result<Self, Error> {
        let nanoseconds = duration.as_nanoseconds();
        if !(-Duration::NANOSECONDS_PER_SECOND..=Duration::NANOSECONDS_PER_SECOND)
            .contains(&nanoseconds)
        {
            return Err(Error::component(
                "UT1−UTC nanoseconds",
                nanoseconds,
                -Duration::NANOSECONDS_PER_SECOND,
                Duration::NANOSECONDS_PER_SECOND,
            ));
        }
        Ok(Self(duration))
    }

    /// Returns the exact duration `UT1−UTC`.
    pub const fn as_duration(self) -> Duration {
        self.0
    }

    /// Returns `UT1−UTC` in SI seconds.
    pub fn as_seconds(self) -> f64 {
        self.0.as_seconds_f64()
    }
}

/// The observed excess of the length of day over 86,400 SI seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExcessLengthOfDay(Duration);

impl ExcessLengthOfDay {
    /// Constructs excess length of day from fractional milliseconds.
    pub fn from_milliseconds(milliseconds: f64) -> Result<Self, Error> {
        Self::from_duration(Duration::from_seconds_f64(milliseconds / 1_000.0)?)
    }

    /// Constructs excess length of day from an exact duration.
    pub fn from_duration(duration: Duration) -> Result<Self, Error> {
        if duration.as_nanoseconds() <= -Duration::NANOSECONDS_PER_DAY {
            return Err(Error::component(
                "excess length of day nanoseconds",
                duration.as_nanoseconds(),
                -Duration::NANOSECONDS_PER_DAY + 1,
                i128::MAX,
            ));
        }
        Ok(Self(duration))
    }

    /// Returns the exact excess duration.
    pub const fn as_duration(self) -> Duration {
        self.0
    }

    /// Returns excess length of day in milliseconds.
    pub fn as_milliseconds(self) -> f64 {
        self.0.as_seconds_f64() * 1_000.0
    }
}

/// Polar motion coordinate $x_p$.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PolarMotionX(Angle);

impl PolarMotionX {
    /// Constructs $x_p$ from arcseconds.
    pub fn from_arcseconds(arcseconds: f64) -> Result<Self, crate::math::Error> {
        Angle::from_radians(arcseconds * RADIANS_PER_ARCSECOND).map(Self)
    }

    /// Returns $x_p$ as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        self.0
    }

    /// Returns $x_p$ in arcseconds.
    pub fn as_arcseconds(self) -> f64 {
        self.0.as_radians() / RADIANS_PER_ARCSECOND
    }

    fn from_angle(angle: Angle) -> Self {
        Self(angle)
    }
}

/// Polar motion coordinate $y_p$.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PolarMotionY(Angle);

impl PolarMotionY {
    /// Constructs $y_p$ from arcseconds.
    pub fn from_arcseconds(arcseconds: f64) -> Result<Self, crate::math::Error> {
        Angle::from_radians(arcseconds * RADIANS_PER_ARCSECOND).map(Self)
    }

    /// Returns $y_p$ as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        self.0
    }

    /// Returns $y_p$ in arcseconds.
    pub fn as_arcseconds(self) -> f64 {
        self.0.as_radians() / RADIANS_PER_ARCSECOND
    }

    fn from_angle(angle: Angle) -> Self {
        Self(angle)
    }
}

/// Celestial-pole correction $dX$.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CelestialPoleOffsetX(Angle);

impl CelestialPoleOffsetX {
    /// Constructs $dX$ from milliarcseconds.
    pub fn from_milliarcseconds(milliarcseconds: f64) -> Result<Self, crate::math::Error> {
        Angle::from_radians(milliarcseconds * RADIANS_PER_ARCSECOND / 1_000.0).map(Self)
    }

    /// Returns $dX$ as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        self.0
    }

    /// Returns $dX$ in milliarcseconds.
    pub fn as_milliarcseconds(self) -> f64 {
        self.0.as_radians() / RADIANS_PER_ARCSECOND * 1_000.0
    }

    fn from_angle(angle: Angle) -> Self {
        Self(angle)
    }
}

/// Celestial-pole correction $dY$.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CelestialPoleOffsetY(Angle);

impl CelestialPoleOffsetY {
    /// Constructs $dY$ from milliarcseconds.
    pub fn from_milliarcseconds(milliarcseconds: f64) -> Result<Self, crate::math::Error> {
        Angle::from_radians(milliarcseconds * RADIANS_PER_ARCSECOND / 1_000.0).map(Self)
    }

    /// Returns $dY$ as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        self.0
    }

    /// Returns $dY$ in milliarcseconds.
    pub fn as_milliarcseconds(self) -> f64 {
        self.0.as_radians() / RADIANS_PER_ARCSECOND * 1_000.0
    }

    fn from_angle(angle: Angle) -> Self {
        Self(angle)
    }
}

/// One Earth-orientation observation at an exact UTC-tagged physical instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthOrientationSample {
    epoch: Instant<Utc>,
    ut1_minus_utc: Ut1MinusUtc,
    excess_length_of_day: ExcessLengthOfDay,
    polar_motion_x: PolarMotionX,
    polar_motion_y: PolarMotionY,
    celestial_pole_offset_x: CelestialPoleOffsetX,
    celestial_pole_offset_y: CelestialPoleOffsetY,
}

impl EarthOrientationSample {
    /// Constructs one sample from validated semantic values.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        epoch: Instant<Utc>,
        ut1_minus_utc: Ut1MinusUtc,
        excess_length_of_day: ExcessLengthOfDay,
        polar_motion_x: PolarMotionX,
        polar_motion_y: PolarMotionY,
        celestial_pole_offset_x: CelestialPoleOffsetX,
        celestial_pole_offset_y: CelestialPoleOffsetY,
    ) -> Self {
        Self {
            epoch,
            ut1_minus_utc,
            excess_length_of_day,
            polar_motion_x,
            polar_motion_y,
            celestial_pole_offset_x,
            celestial_pole_offset_y,
        }
    }

    /// Returns the sample epoch.
    pub const fn epoch(self) -> Instant<Utc> {
        self.epoch
    }

    /// Returns observed `UT1−UTC`.
    pub const fn ut1_minus_utc(self) -> Ut1MinusUtc {
        self.ut1_minus_utc
    }

    /// Returns excess length of day.
    pub const fn excess_length_of_day(self) -> ExcessLengthOfDay {
        self.excess_length_of_day
    }

    /// Returns polar motion $x_p$.
    pub const fn polar_motion_x(self) -> PolarMotionX {
        self.polar_motion_x
    }

    /// Returns polar motion $y_p$.
    pub const fn polar_motion_y(self) -> PolarMotionY {
        self.polar_motion_y
    }

    /// Returns celestial-pole correction $dX$.
    pub const fn celestial_pole_offset_x(self) -> CelestialPoleOffsetX {
        self.celestial_pole_offset_x
    }

    /// Returns celestial-pole correction $dY$.
    pub const fn celestial_pole_offset_y(self) -> CelestialPoleOffsetY {
        self.celestial_pole_offset_y
    }
}

/// Validated, linearly interpolated Earth-orientation observations and metadata.
#[derive(Debug, Clone, Copy)]
pub struct EarthOrientationTable<'a> {
    samples: &'a [EarthOrientationSample],
    version: &'a str,
    expires: Instant<Utc>,
}

impl<'a> EarthOrientationTable<'a> {
    /// Validates a non-empty, strictly ordered Earth-orientation table.
    ///
    /// `expires` is the exclusive metadata expiration instant and must follow
    /// the final sample. Numerical coverage remains the closed interval from
    /// the first through the last sample; the table never extrapolates.
    pub fn new(
        samples: &'a [EarthOrientationSample],
        version: &'a str,
        expires: Instant<Utc>,
    ) -> Result<Self, Error> {
        if samples.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "at least one sample is required",
            });
        }
        if version.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "version must not be empty",
            });
        }
        for (index, pair) in samples.windows(2).enumerate() {
            if pair[0].epoch.tai_nanoseconds_since_1900()
                >= pair[1].epoch.tai_nanoseconds_since_1900()
            {
                return Err(Error::InvalidEarthOrientationSample {
                    index: index + 1,
                    reason: "sample epochs must be strictly increasing",
                });
            }
        }
        let last = samples[samples.len() - 1]
            .epoch
            .tai_nanoseconds_since_1900();
        if expires.tai_nanoseconds_since_1900() <= last {
            return Err(Error::InvalidEarthOrientationData {
                reason: "expiration must follow the final sample",
            });
        }
        Ok(Self {
            samples,
            version,
            expires,
        })
    }

    /// Returns the original validated samples.
    pub const fn samples(self) -> &'a [EarthOrientationSample] {
        self.samples
    }

    /// Returns the provider's version identifier.
    pub const fn version(self) -> &'a str {
        self.version
    }

    /// Returns the closed physical interval covered by interpolation.
    pub fn coverage(self) -> (Instant<Utc>, Instant<Utc>) {
        (
            self.samples[0].epoch,
            self.samples[self.samples.len() - 1].epoch,
        )
    }

    /// Returns the exclusive metadata expiration instant.
    pub const fn expires(self) -> Instant<Utc> {
        self.expires
    }

    pub(crate) fn at<S: TimeScale>(
        self,
        epoch: Instant<S>,
        leap_seconds: LeapSeconds<'_>,
    ) -> Result<EarthOrientation<S>, Error> {
        let requested = epoch.tai_nanoseconds_since_1900();
        let first = self.samples[0].epoch.tai_nanoseconds_since_1900();
        let last = self.samples[self.samples.len() - 1]
            .epoch
            .tai_nanoseconds_since_1900();
        let expires = self.expires.tai_nanoseconds_since_1900();
        if requested >= expires {
            return Err(Error::EarthOrientationExpired { requested, expires });
        }
        if requested < first || requested > last {
            return Err(Error::EarthOrientationUnavailable {
                requested,
                coverage_start: first,
                coverage_end: last,
            });
        }

        let query_tai_minus_utc = leap_seconds.offset(epoch.retag::<Tai>())?;
        match self.samples.binary_search_by_key(&requested, |sample| {
            sample.epoch.tai_nanoseconds_since_1900()
        }) {
            Ok(index) => Ok(EarthOrientation::from_sample(epoch, self.samples[index])),
            Err(right_index) => {
                let left = self.samples[right_index - 1];
                let right = self.samples[right_index];
                let left_epoch = left.epoch.tai_nanoseconds_since_1900();
                let right_epoch = right.epoch.tai_nanoseconds_since_1900();
                let elapsed = requested.checked_sub(left_epoch).ok_or(Error::Overflow {
                    operation: "interpolating Earth-orientation epoch",
                })?;
                let span = right_epoch.checked_sub(left_epoch).ok_or(Error::Overflow {
                    operation: "interpolating Earth-orientation span",
                })?;
                let fraction = elapsed as f64 / span as f64;

                let left_tai_minus_utc = leap_seconds.offset(left.epoch.retag::<Tai>())?;
                let right_tai_minus_utc = leap_seconds.offset(right.epoch.retag::<Tai>())?;
                let left_ut1_minus_tai = left
                    .ut1_minus_utc
                    .as_duration()
                    .checked_sub(left_tai_minus_utc)?;
                let right_ut1_minus_tai = right
                    .ut1_minus_utc
                    .as_duration()
                    .checked_sub(right_tai_minus_utc)?;
                let ut1_minus_tai = Duration::from_nanoseconds(Self::interpolate_nanoseconds(
                    left_ut1_minus_tai.as_nanoseconds(),
                    right_ut1_minus_tai.as_nanoseconds(),
                    fraction,
                )?);
                let ut1_minus_utc =
                    Ut1MinusUtc::from_duration(ut1_minus_tai.checked_add(query_tai_minus_utc)?)?;
                let excess_length_of_day = ExcessLengthOfDay::from_duration(
                    Duration::from_nanoseconds(Self::interpolate_nanoseconds(
                        left.excess_length_of_day.as_duration().as_nanoseconds(),
                        right.excess_length_of_day.as_duration().as_nanoseconds(),
                        fraction,
                    )?),
                )?;
                let polar_motion_x = PolarMotionX::from_angle(Self::interpolate_angle(
                    "interpolated polar motion x",
                    left.polar_motion_x.as_angle(),
                    right.polar_motion_x.as_angle(),
                    fraction,
                )?);
                let polar_motion_y = PolarMotionY::from_angle(Self::interpolate_angle(
                    "interpolated polar motion y",
                    left.polar_motion_y.as_angle(),
                    right.polar_motion_y.as_angle(),
                    fraction,
                )?);
                let celestial_pole_offset_x =
                    CelestialPoleOffsetX::from_angle(Self::interpolate_angle(
                        "interpolated celestial pole offset x",
                        left.celestial_pole_offset_x.as_angle(),
                        right.celestial_pole_offset_x.as_angle(),
                        fraction,
                    )?);
                let celestial_pole_offset_y =
                    CelestialPoleOffsetY::from_angle(Self::interpolate_angle(
                        "interpolated celestial pole offset y",
                        left.celestial_pole_offset_y.as_angle(),
                        right.celestial_pole_offset_y.as_angle(),
                        fraction,
                    )?);

                Ok(EarthOrientation {
                    epoch,
                    ut1_minus_utc,
                    excess_length_of_day,
                    polar_motion_x,
                    polar_motion_y,
                    celestial_pole_offset_x,
                    celestial_pole_offset_y,
                })
            }
        }
    }

    fn interpolate_nanoseconds(left: i128, right: i128, fraction: f64) -> Result<i128, Error> {
        let value = (1.0 - fraction) * left as f64 + fraction * right as f64;
        Error::ensure_finite("interpolated Earth-orientation duration", value)?;
        Ok(libm::round(value) as i128)
    }

    fn interpolate_angle(
        field: &'static str,
        left: Angle,
        right: Angle,
        fraction: f64,
    ) -> Result<Angle, Error> {
        let value = (1.0 - fraction) * left.as_radians() + fraction * right.as_radians();
        Error::ensure_finite(field, value)?;
        Angle::from_radians(value).map_err(|_| Error::NonFinite { field, value })
    }
}

/// Earth-orientation values resolved at one physical instant.
#[derive(Debug, Clone, Copy)]
pub struct EarthOrientation<S: TimeScale> {
    epoch: Instant<S>,
    ut1_minus_utc: Ut1MinusUtc,
    excess_length_of_day: ExcessLengthOfDay,
    polar_motion_x: PolarMotionX,
    polar_motion_y: PolarMotionY,
    celestial_pole_offset_x: CelestialPoleOffsetX,
    celestial_pole_offset_y: CelestialPoleOffsetY,
}

impl<S: TimeScale> EarthOrientation<S> {
    fn from_sample(epoch: Instant<S>, sample: EarthOrientationSample) -> Self {
        Self {
            epoch,
            ut1_minus_utc: sample.ut1_minus_utc,
            excess_length_of_day: sample.excess_length_of_day,
            polar_motion_x: sample.polar_motion_x,
            polar_motion_y: sample.polar_motion_y,
            celestial_pole_offset_x: sample.celestial_pole_offset_x,
            celestial_pole_offset_y: sample.celestial_pole_offset_y,
        }
    }

    /// Returns the resolved physical epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns interpolated `UT1−UTC`.
    pub const fn ut1_minus_utc(self) -> Ut1MinusUtc {
        self.ut1_minus_utc
    }

    /// Returns interpolated excess length of day.
    pub const fn excess_length_of_day(self) -> ExcessLengthOfDay {
        self.excess_length_of_day
    }

    /// Returns interpolated polar motion $x_p$.
    pub const fn polar_motion_x(self) -> PolarMotionX {
        self.polar_motion_x
    }

    /// Returns interpolated polar motion $y_p$.
    pub const fn polar_motion_y(self) -> PolarMotionY {
        self.polar_motion_y
    }

    /// Returns interpolated celestial-pole correction $dX$.
    pub const fn celestial_pole_offset_x(self) -> CelestialPoleOffsetX {
        self.celestial_pole_offset_x
    }

    /// Returns interpolated celestial-pole correction $dY$.
    pub const fn celestial_pole_offset_y(self) -> CelestialPoleOffsetY {
        self.celestial_pole_offset_y
    }
}
