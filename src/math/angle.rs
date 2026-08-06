use core::f64::consts::{FRAC_PI_2, PI, TAU};

use libm::{cos, sin, tan};

use super::{Dimensionless, Error};

/// A finite angle stored canonically in radians without interval semantics.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Angle(f64);

impl Angle {
    /// Constructs an angle in radians.
    pub fn from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("angle", value).map(Self)
    }

    /// Constructs an angle in degrees.
    pub fn from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("angle in degrees", value)?;
        Self::from_radians(value.to_radians())
    }

    /// Returns the angle in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the angle in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
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

    /// Returns the right ascension in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the right ascension in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
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
}

/// An hour angle in the interval (-π, π].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct HourAngle(f64);

impl HourAngle {
    /// Constructs an hour angle from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("hour angle", value)?;
        if value > -PI && value <= PI {
            Ok(Self(value))
        } else {
            Err(Error::OutOfRange {
                field: "hour angle",
                value,
                interval: "(-π, π]",
                unit: "rad",
            })
        }
    }

    /// Constructs an hour angle from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("hour angle", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Normalizes radians into the hour-angle interval.
    pub fn wrap_radians(value: f64) -> Result<Self, Error> {
        Angle::wrap_signed(value, "hour angle").map(Self)
    }

    /// Normalizes degrees into the hour-angle interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("hour angle", value)?;
        Self::wrap_radians(value.to_radians())
    }

    /// Returns the hour angle in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the hour angle in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
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
}
