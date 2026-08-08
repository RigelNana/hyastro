use core::f64::consts::{FRAC_PI_2, PI, TAU};

use libm::{cos, sin, tan};

use crate::constants::angle::DEGREES_PER_HOUR;

use super::{
    Dimensionless, Error,
    sexagesimal::{DegreesMinutesSeconds, HoursMinutesSeconds},
};

/// A finite angle stored canonically in radians without interval semantics.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Angle(f64);

impl Angle {
    /// Constructs an angle in radians.
    pub fn from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("angle", value).map(Self)
    }
    pub(crate) const fn from_finite(value: f64) -> Self {
        Self(value)
    }

    /// Constructs an angle in degrees.
    pub fn from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("angle in degrees", value)?;
        Self::from_radians(value.to_radians())
    }

    /// Constructs an unrestricted angle from a DMS representation.
    pub fn from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::from_degrees(value.as_decimal_degrees())
    }

    /// Returns the angle in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the angle in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Converts this angle to a DMS representation.
    ///
    /// Very large angles whose whole-degree magnitude exceeds `u16` return a
    /// range error rather than truncating the representation.
    pub fn to_dms(self) -> Result<DegreesMinutesSeconds, Error> {
        DegreesMinutesSeconds::from_decimal_degrees(self.as_degrees())
    }

    /// Returns the sine of the angle.
    pub fn sin(self) -> Dimensionless {
        Dimensionless::from_finite(sin(self.0))
    }

    /// Returns the cosine of the angle.
    pub fn cos(self) -> Dimensionless {
        Dimensionless::from_finite(cos(self.0))
    }

    /// Returns the sine and cosine of the angle.
    pub fn sin_cos(self) -> (Dimensionless, Dimensionless) {
        (self.sin(), self.cos())
    }

    /// Returns the tangent of the angle when finite.
    pub fn tan(self) -> Result<Dimensionless, Error> {
        Dimensionless::new(tan(self.0))
    }

    /// Adds another angle while preserving the finite invariant.
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        Self::from_radians(self.0 + rhs.0)
    }

    /// Subtracts another angle while preserving the finite invariant.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        Self::from_radians(self.0 - rhs.0)
    }

    /// Scales the angle while preserving the finite invariant.
    pub fn checked_scale(self, factor: f64) -> Result<Self, Error> {
        Error::ensure_finite("angle scale factor", factor)?;
        Self::from_radians(self.0 * factor)
    }

    pub(crate) fn wrap_zero_tau(value: f64, field: &'static str) -> Result<f64, Error> {
        Error::ensure_finite(field, value)?;
        let wrapped = value % TAU;
        Ok(if wrapped < 0.0 {
            wrapped + TAU
        } else {
            wrapped
        })
    }

    pub(crate) fn wrap_signed(value: f64, field: &'static str) -> Result<f64, Error> {
        let wrapped = Self::wrap_zero_tau(value, field)?;
        Ok(if wrapped > PI { wrapped - TAU } else { wrapped })
    }
}

/// A right ascension in the half-open interval [0, 2π).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct RightAscension(f64);

impl RightAscension {
    /// Constructs a right ascension from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("right ascension", value)?;
        if (0.0..TAU).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "right ascension",
                value,
                interval: "[0, 2π)",
                unit: "rad",
            })
        }
    }

    /// Constructs a right ascension from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("right ascension", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Normalizes radians into the right ascension interval.
    pub fn wrap_radians(value: f64) -> Result<Self, Error> {
        Angle::wrap_zero_tau(value, "right ascension").map(Self)
    }

    /// Normalizes degrees into the right ascension interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("right ascension", value)?;
        Self::wrap_radians(value.to_radians())
    }

    /// Constructs a right ascension from decimal angular hours without normalization.
    pub fn try_from_hours(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("right ascension in hours", value)?;
        Self::try_from_degrees(value * DEGREES_PER_HOUR)
    }

    /// Constructs a right ascension from canonical HMS.
    pub fn try_from_hms(value: HoursMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_hours(value.as_decimal_hours())
    }

    /// Normalizes decimal angular hours into the right-ascension interval.
    pub fn wrap_hours(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("right ascension in hours", value)?;
        Self::wrap_degrees(value * DEGREES_PER_HOUR)
    }

    /// Returns the right ascension in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the right ascension in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the right ascension in decimal angular hours.
    pub fn as_hours(self) -> f64 {
        self.as_degrees() / DEGREES_PER_HOUR
    }

    /// Returns the canonical HMS representation.
    pub fn to_hms(self) -> HoursMinutesSeconds {
        HoursMinutesSeconds::from_valid_decimal_hours(self.as_hours())
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }
}

/// An azimuth in the half-open interval [0, 2π).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Azimuth(f64);

impl Azimuth {
    /// Constructs an azimuth from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("azimuth", value)?;
        if (0.0..TAU).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "azimuth",
                value,
                interval: "[0, 2π)",
                unit: "rad",
            })
        }
    }

    /// Constructs an azimuth from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("azimuth", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Normalizes radians into the azimuth interval.
    pub fn wrap_radians(value: f64) -> Result<Self, Error> {
        Angle::wrap_zero_tau(value, "azimuth").map(Self)
    }

    /// Normalizes degrees into the azimuth interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("azimuth", value)?;
        Self::wrap_radians(value.to_radians())
    }

    /// Returns the azimuth in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the azimuth in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }

    /// Constructs an azimuth from DMS without normalization.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Normalizes DMS into the azimuth interval.
    pub fn wrap_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::wrap_degrees(value.as_decimal_degrees())
    }

    /// Returns the azimuth as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// A position angle in the half-open interval [0, 2π).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PositionAngle(f64);

impl PositionAngle {
    /// Constructs a position angle from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("position angle", value)?;
        if (0.0..TAU).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "position angle",
                value,
                interval: "[0, 2π)",
                unit: "rad",
            })
        }
    }

    /// Constructs a position angle from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("position angle", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Normalizes radians into the position-angle interval.
    pub fn wrap_radians(value: f64) -> Result<Self, Error> {
        Angle::wrap_zero_tau(value, "position angle").map(Self)
    }

    /// Normalizes degrees into the position-angle interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("position angle", value)?;
        Self::wrap_radians(value.to_radians())
    }

    /// Returns the position angle in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the position angle in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }

    /// Constructs a position angle from DMS without normalization.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Normalizes DMS into the position-angle interval.
    pub fn wrap_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::wrap_degrees(value.as_decimal_degrees())
    }

    /// Returns the position angle as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// A longitude in the interval (-π, π].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Longitude(f64);

impl Longitude {
    /// Constructs a longitude from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("longitude", value)?;
        if value > -PI && value <= PI {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "longitude",
                value,
                interval: "(-π, π]",
                unit: "rad",
            })
        }
    }

    /// Constructs a longitude from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("longitude", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Normalizes radians into the longitude interval.
    pub fn wrap_radians(value: f64) -> Result<Self, Error> {
        Angle::wrap_signed(value, "longitude").map(Self)
    }

    /// Normalizes degrees into the longitude interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("longitude", value)?;
        Self::wrap_radians(value.to_radians())
    }

    /// Returns the longitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the longitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }

    /// Constructs a longitude from DMS without normalization.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Normalizes DMS into the longitude interval.
    pub fn wrap_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::wrap_degrees(value.as_decimal_degrees())
    }

    /// Returns the longitude as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// An hour angle in the half-open interval [0, 2π), equivalent to [0h, 24h).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct HourAngle(f64);

impl HourAngle {
    /// Constructs an hour angle from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("hour angle", value)?;
        if (0.0..TAU).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "hour angle",
                value,
                interval: "[0, 2π)",
                unit: "rad",
            })
        }
    }

    /// Constructs an hour angle from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("hour angle", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Constructs an hour angle from decimal angular hours without normalization.
    pub fn try_from_hours(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("hour angle in hours", value)?;
        Self::try_from_degrees(value * DEGREES_PER_HOUR)
    }

    /// Constructs an hour angle from canonical HMS.
    pub fn try_from_hms(value: HoursMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_hours(value.as_decimal_hours())
    }

    /// Normalizes radians into the hour-angle interval.
    pub fn wrap_radians(value: f64) -> Result<Self, Error> {
        Angle::wrap_zero_tau(value, "hour angle").map(Self)
    }

    /// Normalizes degrees into the hour-angle interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("hour angle", value)?;
        Self::wrap_radians(value.to_radians())
    }

    /// Normalizes decimal angular hours into the hour-angle interval.
    pub fn wrap_hours(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("hour angle in hours", value)?;
        Self::wrap_degrees(value * DEGREES_PER_HOUR)
    }

    /// Returns the hour angle in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the hour angle in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the hour angle in decimal angular hours.
    pub fn as_hours(self) -> f64 {
        self.as_degrees() / DEGREES_PER_HOUR
    }

    /// Returns the canonical HMS representation.
    pub fn to_hms(self) -> HoursMinutesSeconds {
        HoursMinutesSeconds::from_valid_decimal_hours(self.as_hours())
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }
}

/// A latitude in the closed interval [-π/2, π/2].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Latitude(f64);

impl Latitude {
    /// Constructs a latitude from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("latitude", value)?;
        if (-FRAC_PI_2..=FRAC_PI_2).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "latitude",
                value,
                interval: "[-π/2, π/2]",
                unit: "rad",
            })
        }
    }

    /// Constructs a latitude from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("latitude", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Returns the latitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the latitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }

    /// Constructs a latitude from DMS.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the latitude as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// A declination in the closed interval [-π/2, π/2].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Declination(f64);

impl Declination {
    /// Constructs a declination from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("declination", value)?;
        if (-FRAC_PI_2..=FRAC_PI_2).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "declination",
                value,
                interval: "[-π/2, π/2]",
                unit: "rad",
            })
        }
    }

    /// Constructs a declination from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("declination", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Returns the declination in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the declination in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }

    /// Constructs a declination from DMS.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the declination as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// An altitude in the closed interval [-π/2, π/2].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Altitude(f64);

impl Altitude {
    /// Constructs an altitude from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("altitude", value)?;
        if (-FRAC_PI_2..=FRAC_PI_2).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "altitude",
                value,
                interval: "[-π/2, π/2]",
                unit: "rad",
            })
        }
    }

    /// Constructs an altitude from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("altitude", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Returns the altitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the altitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }

    /// Constructs an altitude from DMS.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the altitude as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// A zenith distance in the closed interval [0, π].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ZenithDistance(f64);

impl ZenithDistance {
    /// Constructs a zenith distance from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("zenith distance", value)?;
        if (0.0..=PI).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "zenith distance",
                value,
                interval: "[0, π]",
                unit: "rad",
            })
        }
    }

    /// Constructs a zenith distance from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("zenith distance", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Returns the zenith distance in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the zenith distance in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }

    /// Constructs a zenith distance from DMS.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the zenith distance as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// An angular separation in the closed interval [0, π].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Separation(f64);

impl Separation {
    /// Constructs an angular separation from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("separation", value)?;
        if (0.0..=PI).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "separation",
                value,
                interval: "[0, π]",
                unit: "rad",
            })
        }
    }

    /// Constructs an angular separation from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("separation", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Returns the separation in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the separation in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }

    /// Constructs an angular separation from DMS.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the angular separation as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// A phase angle in the closed interval [0, π].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PhaseAngle(f64);

impl PhaseAngle {
    /// Constructs a phase angle from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("phase angle", value)?;
        if (0.0..=PI).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "phase angle",
                value,
                interval: "[0, π]",
                unit: "rad",
            })
        }
    }

    /// Constructs a phase angle from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("phase angle", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Returns the phase angle in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the phase angle in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle(self.0)
    }

    /// Constructs a phase angle from DMS.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, Error> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the phase angle as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}
