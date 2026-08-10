use core::fmt;

use libm::{atan2, cos, sin};

use crate::{
    math::{Angle, AngularSpeed, Declination, Error as MathError, HourAngle, Latitude},
    time::{Duration, Instant, TimeScale},
};

use super::Error;

/// A signed parallactic angle in the interval `(-π, π]`.
///
/// Positive angles run eastward from celestial north toward the local zenith.
/// This is the standard SOFA `iauHd2pa` convention.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ParallacticAngle(f64);

impl ParallacticAngle {
    /// Constructs a parallactic angle from typed hour angle, declination, and site latitude.
    pub fn from_equatorial(
        hour_angle: HourAngle,
        declination: Declination,
        site_latitude: Latitude,
    ) -> Result<Self, MathError> {
        let latitude = site_latitude.as_radians();
        Self::from_components(
            hour_angle.as_radians(),
            declination.as_radians(),
            sin(latitude),
            cos(latitude),
        )
    }

    /// Normalizes radians into the signed parallactic-angle interval.
    pub fn wrap_radians(value: f64) -> Result<Self, MathError> {
        Angle::wrap_signed(value, "parallactic angle").map(Self)
    }

    /// Normalizes degrees into the signed parallactic-angle interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, MathError> {
        Self::wrap_radians(value.to_radians())
    }

    pub(crate) fn from_components(
        hour_angle: f64,
        declination: f64,
        latitude_sine: f64,
        latitude_cosine: f64,
    ) -> Result<Self, MathError> {
        let numerator = latitude_cosine * sin(hour_angle);
        let denominator =
            latitude_sine * cos(declination) - latitude_cosine * sin(declination) * cos(hour_angle);
        let value = if numerator == 0.0 && denominator == 0.0 {
            0.0
        } else {
            atan2(numerator, denominator)
        };
        Self::wrap_radians(value)
    }

    /// Returns the signed angle in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the signed angle in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns this value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle::from_finite(self.0)
    }
}

/// A parallactic angle tied to one physical observation epoch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParallacticAngleAt<S: TimeScale> {
    epoch: Instant<S>,
    angle: ParallacticAngle,
}

impl<S: TimeScale> ParallacticAngleAt<S> {
    /// Constructs one epoch-bound parallactic-angle sample.
    pub const fn new(epoch: Instant<S>, angle: ParallacticAngle) -> Self {
        Self { epoch, angle }
    }

    /// Returns the sample epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the signed parallactic angle.
    pub const fn angle(self) -> ParallacticAngle {
        self.angle
    }
}

/// Sign of the observed parallactic-angle rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldRotationDirection {
    /// Position angle is increasing eastward on the sky.
    IncreasingPositionAngle,
    /// Position angle is decreasing westward on the sky.
    DecreasingPositionAngle,
    /// The symmetric samples have no resolved position-angle change.
    Stationary,
}

/// A signed observed field-rotation rate.
///
/// Positive values mean increasing eastward position angle; negative values
/// mean decreasing position angle. This sign convention avoids the ambiguous
/// clockwise/counter-clockwise wording that reverses between sky and image views.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FieldRotationRate(f64);

impl FieldRotationRate {
    /// Constructs a finite signed rate in radians per SI second.
    pub fn from_radians_per_second(value: f64) -> Result<Self, MathError> {
        Angle::from_radians(value).map(|_| Self(value))
    }

    /// Constructs a finite signed rate in degrees per SI second.
    pub fn from_degrees_per_second(value: f64) -> Result<Self, MathError> {
        Self::from_radians_per_second(value.to_radians())
    }

    /// Returns the signed rate in radians per SI second.
    pub const fn as_radians_per_second(self) -> f64 {
        self.0
    }

    /// Returns the signed rate in degrees per SI second.
    pub fn as_degrees_per_second(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the signed rate in arcseconds per SI second.
    pub fn as_arcseconds_per_second(self) -> f64 {
        self.as_degrees_per_second() * 3_600.0
    }

    /// Returns the unsigned magnitude as a strongly typed angular speed.
    pub fn magnitude(self) -> AngularSpeed {
        AngularSpeed::from_radians_per_second(self.0.abs())
            .expect("the absolute value of a finite field-rotation rate is finite")
    }

    /// Returns the rate's explicit direction classification.
    pub const fn direction(self) -> FieldRotationDirection {
        if self.0 > 0.0 {
            FieldRotationDirection::IncreasingPositionAngle
        } else if self.0 < 0.0 {
            FieldRotationDirection::DecreasingPositionAngle
        } else {
            FieldRotationDirection::Stationary
        }
    }
}

/// Symmetric finite-difference controls for an instantaneous field-rotation estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldRotationOptions {
    sample_offset: Duration,
}

impl FieldRotationOptions {
    /// Constructs controls from the positive offset on each side of the requested epoch.
    ///
    /// The complete differencing baseline is twice this duration.
    pub fn new(sample_offset: Duration) -> Result<Self, Error> {
        if sample_offset <= Duration::ZERO {
            return Err(Error::InvalidFieldRotationSampleOffset {
                nanoseconds: sample_offset.as_nanoseconds(),
            });
        }
        Ok(Self { sample_offset })
    }

    /// Returns the standard one-second offset and two-second baseline.
    pub const fn standard() -> Self {
        Self {
            sample_offset: Duration::from_nanoseconds(Duration::NANOSECONDS_PER_SECOND),
        }
    }

    /// Returns the positive offset sampled on each side of the requested epoch.
    pub const fn sample_offset(self) -> Duration {
        self.sample_offset
    }
}

impl Default for FieldRotationOptions {
    fn default() -> Self {
        Self::standard()
    }
}

/// Parallactic angle and symmetric observed field-rotation estimate at one epoch.
#[derive(Clone, Copy, PartialEq)]
pub struct FieldRotation<S: TimeScale> {
    previous: ParallacticAngleAt<S>,
    current: ParallacticAngleAt<S>,
    next: ParallacticAngleAt<S>,
    position_angle_change: Angle,
    rate: FieldRotationRate,
}

impl<S: TimeScale> FieldRotation<S> {
    /// Constructs a central field-rotation estimate from three symmetric samples.
    pub fn from_symmetric_samples(
        previous: ParallacticAngleAt<S>,
        current: ParallacticAngleAt<S>,
        next: ParallacticAngleAt<S>,
    ) -> Result<Self, Error> {
        let before = current.epoch().duration_since(previous.epoch())?;
        let after = next.epoch().duration_since(current.epoch())?;
        if before <= Duration::ZERO || after <= Duration::ZERO {
            return Err(Error::InvalidFieldRotationSampleOrder);
        }
        if before != after {
            return Err(Error::AsymmetricFieldRotationSamples {
                before_nanoseconds: before.as_nanoseconds(),
                after_nanoseconds: after.as_nanoseconds(),
            });
        }
        let baseline = before.checked_add(after)?;
        let change_radians = Angle::wrap_signed(
            next.angle().as_radians() - previous.angle().as_radians(),
            "field-rotation position-angle change",
        )?;
        let position_angle_change = Angle::from_radians(change_radians)?;
        let rate =
            FieldRotationRate::from_radians_per_second(change_radians / baseline.as_seconds_f64())?;
        Ok(Self {
            previous,
            current,
            next,
            position_angle_change,
            rate,
        })
    }

    /// Returns the central physical epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.current.epoch()
    }

    /// Returns the parallactic angle at the central epoch.
    pub const fn parallactic_angle(self) -> ParallacticAngle {
        self.current.angle()
    }

    /// Returns the positive offset used on each side of the central epoch.
    pub fn sample_offset(self) -> Result<Duration, Error> {
        Ok(self.current.epoch().duration_since(self.previous.epoch())?)
    }

    /// Returns the previous parallactic-angle sample.
    pub const fn previous(self) -> ParallacticAngleAt<S> {
        self.previous
    }

    /// Returns the central parallactic-angle sample.
    pub const fn current(self) -> ParallacticAngleAt<S> {
        self.current
    }

    /// Returns the next parallactic-angle sample.
    pub const fn next(self) -> ParallacticAngleAt<S> {
        self.next
    }

    /// Returns the shortest signed position-angle change across the full baseline.
    pub const fn position_angle_change(self) -> Angle {
        self.position_angle_change
    }

    /// Returns the signed observed field-rotation rate.
    pub const fn rate(self) -> FieldRotationRate {
        self.rate
    }

    /// Returns whether the observed position angle increases, decreases, or is stationary.
    pub const fn direction(self) -> FieldRotationDirection {
        self.rate.direction()
    }
}

impl<S: TimeScale> fmt::Debug for FieldRotation<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldRotation")
            .field("previous", &self.previous)
            .field("current", &self.current)
            .field("next", &self.next)
            .field("position_angle_change", &self.position_angle_change)
            .field("rate", &self.rate)
            .field("direction", &self.direction())
            .finish()
    }
}
