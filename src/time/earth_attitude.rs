use crate::math::Angle;

use super::{
    CelestialPoleOffsetX, CelestialPoleOffsetY, Duration, Error, Instant, LeapSeconds,
    PolarMotionX, PolarMotionY, Tai, TimeScale, Ut1MinusUtc, Utc,
};

/// One Earth-attitude observation at an exact UTC-tagged physical instant.
///
/// Attitude samples contain every value needed for observed celestial-to-
/// terrestrial direction rotations, but deliberately do not require length of
/// day or angular rates needed by position-velocity state transforms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthAttitudeSample {
    epoch: Instant<Utc>,
    ut1_minus_utc: Ut1MinusUtc,
    polar_motion_x: PolarMotionX,
    polar_motion_y: PolarMotionY,
    celestial_pole_offset_x: CelestialPoleOffsetX,
    celestial_pole_offset_y: CelestialPoleOffsetY,
}

impl EarthAttitudeSample {
    /// Constructs a sample from validated Earth-attitude values.
    pub const fn new(
        epoch: Instant<Utc>,
        ut1_minus_utc: Ut1MinusUtc,
        polar_motion_x: PolarMotionX,
        polar_motion_y: PolarMotionY,
        celestial_pole_offset_x: CelestialPoleOffsetX,
        celestial_pole_offset_y: CelestialPoleOffsetY,
    ) -> Self {
        Self {
            epoch,
            ut1_minus_utc,
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

/// Validated Earth-attitude observations used for direction-frame rotations.
///
/// Unlike [`EarthOrientationTable`](super::EarthOrientationTable), this table
/// does not require length of day or angular rates. It can drive observed
/// GCRS/CIRS/TIRS/ITRS direction rotations, but cannot prove a full state
/// transform with measured frame velocity.
#[derive(Debug, Clone, Copy)]
pub struct EarthAttitudeTable<'a> {
    samples: &'a [EarthAttitudeSample],
    version: &'a str,
    expires: Instant<Utc>,
}

impl<'a> EarthAttitudeTable<'a> {
    /// Validates a non-empty, strictly ordered Earth-attitude table.
    pub fn new(
        samples: &'a [EarthAttitudeSample],
        version: &'a str,
        expires: Instant<Utc>,
    ) -> Result<Self, Error> {
        if samples.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "at least one Earth-attitude sample is required",
            });
        }
        if version.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "Earth-attitude version must not be empty",
            });
        }
        for (index, pair) in samples.windows(2).enumerate() {
            if pair[0].epoch.tai_nanoseconds_since_1900()
                >= pair[1].epoch.tai_nanoseconds_since_1900()
            {
                return Err(Error::InvalidEarthOrientationSample {
                    index: index + 1,
                    reason: "Earth-attitude sample epochs must be strictly increasing",
                });
            }
        }
        let last = samples[samples.len() - 1]
            .epoch
            .tai_nanoseconds_since_1900();
        if expires.tai_nanoseconds_since_1900() <= last {
            return Err(Error::InvalidEarthOrientationData {
                reason: "Earth-attitude expiration must follow the final sample",
            });
        }
        Ok(Self {
            samples,
            version,
            expires,
        })
    }

    /// Returns the original validated samples.
    pub const fn samples(self) -> &'a [EarthAttitudeSample] {
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
    ) -> Result<EarthAttitude<S>, Error> {
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

        if self.samples.len() == 1 {
            return Ok(EarthAttitude::from_sample(epoch, self.samples[0]));
        }

        let (left_index, right_index) =
            match self.samples.binary_search_by_key(&requested, |sample| {
                sample.epoch.tai_nanoseconds_since_1900()
            }) {
                Ok(index) if index + 1 < self.samples.len() => (index, index + 1),
                Ok(index) => (index - 1, index),
                Err(right_index) => (right_index - 1, right_index),
            };
        let left = self.samples[left_index];
        let right = self.samples[right_index];
        let left_epoch = left.epoch.tai_nanoseconds_since_1900();
        let right_epoch = right.epoch.tai_nanoseconds_since_1900();
        let elapsed = requested.checked_sub(left_epoch).ok_or(Error::Overflow {
            operation: "interpolating Earth-attitude epoch",
        })?;
        let span = right_epoch.checked_sub(left_epoch).ok_or(Error::Overflow {
            operation: "interpolating Earth-attitude span",
        })?;
        let fraction = elapsed as f64 / span as f64;
        Error::ensure_finite("Earth-attitude interpolation fraction", fraction)?;

        let query_tai_minus_utc = leap_seconds.offset(epoch.retag::<Tai>())?;
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
        let interpolated = (1.0 - fraction) * left_ut1_minus_tai.as_nanoseconds() as f64
            + fraction * right_ut1_minus_tai.as_nanoseconds() as f64;
        Error::ensure_finite("interpolated UT1−TAI nanoseconds", interpolated)?;
        let ut1_minus_tai = Duration::from_nanoseconds(libm::round(interpolated) as i128);
        let ut1_minus_utc =
            Ut1MinusUtc::from_duration(ut1_minus_tai.checked_add(query_tai_minus_utc)?)?;

        Ok(EarthAttitude {
            epoch,
            ut1_minus_utc,
            polar_motion_x: PolarMotionX::from_angle(Self::interpolate_angle(
                "interpolated polar motion x",
                left.polar_motion_x.as_angle(),
                right.polar_motion_x.as_angle(),
                fraction,
            )?),
            polar_motion_y: PolarMotionY::from_angle(Self::interpolate_angle(
                "interpolated polar motion y",
                left.polar_motion_y.as_angle(),
                right.polar_motion_y.as_angle(),
                fraction,
            )?),
            celestial_pole_offset_x: CelestialPoleOffsetX::from_angle(Self::interpolate_angle(
                "interpolated celestial-pole offset x",
                left.celestial_pole_offset_x.as_angle(),
                right.celestial_pole_offset_x.as_angle(),
                fraction,
            )?),
            celestial_pole_offset_y: CelestialPoleOffsetY::from_angle(Self::interpolate_angle(
                "interpolated celestial-pole offset y",
                left.celestial_pole_offset_y.as_angle(),
                right.celestial_pole_offset_y.as_angle(),
                fraction,
            )?),
        })
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

/// Earth-attitude values resolved at one physical instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthAttitude<S: TimeScale> {
    epoch: Instant<S>,
    ut1_minus_utc: Ut1MinusUtc,
    polar_motion_x: PolarMotionX,
    polar_motion_y: PolarMotionY,
    celestial_pole_offset_x: CelestialPoleOffsetX,
    celestial_pole_offset_y: CelestialPoleOffsetY,
}

impl<S: TimeScale> EarthAttitude<S> {
    fn from_sample(epoch: Instant<S>, sample: EarthAttitudeSample) -> Self {
        Self {
            epoch,
            ut1_minus_utc: sample.ut1_minus_utc,
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
