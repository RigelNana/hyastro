use core::f64::consts::{FRAC_PI_2, TAU};

use libm::{asin, atan2, cos, sin, sqrt};

use crate::math::{Angle, DegreesMinutesSeconds, Direction, Error as MathError, Separation};

use super::Galactic;
#[cfg(feature = "std")]
use super::{EquatorialDirection, Icrs};

/// A Galactic longitude in the half-open interval [0, 2π).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GalacticLongitude(f64);

impl GalacticLongitude {
    /// Constructs a Galactic longitude from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("Galactic longitude", value)?;
        if (0.0..TAU).contains(&value) {
            Ok(Self(value))
        } else {
            Err(MathError::OutOfRange {
                field: "Galactic longitude",
                value,
                interval: "[0, 2π)",
                unit: "rad",
            })
        }
    }

    /// Constructs a Galactic longitude from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("Galactic longitude", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Normalizes radians into the Galactic-longitude interval.
    pub fn wrap_radians(value: f64) -> Result<Self, MathError> {
        Angle::wrap_zero_tau(value, "Galactic longitude").map(Self)
    }

    /// Normalizes degrees into the Galactic-longitude interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("Galactic longitude", value)?;
        Self::wrap_radians(value.to_radians())
    }

    /// Constructs a Galactic longitude from DMS without normalization.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, MathError> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the Galactic longitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the Galactic longitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle::from_finite(self.0)
    }

    /// Returns the Galactic longitude as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// A Galactic latitude in the closed interval [-π/2, π/2].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GalacticLatitude(f64);

impl GalacticLatitude {
    /// Constructs a Galactic latitude from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("Galactic latitude", value)?;
        if (-FRAC_PI_2..=FRAC_PI_2).contains(&value) {
            Ok(Self(value))
        } else {
            Err(MathError::OutOfRange {
                field: "Galactic latitude",
                value,
                interval: "[-π/2, π/2]",
                unit: "rad",
            })
        }
    }

    /// Constructs a Galactic latitude from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("Galactic latitude", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Constructs a Galactic latitude from DMS.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, MathError> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the Galactic latitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the Galactic latitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle::from_finite(self.0)
    }

    /// Returns the Galactic latitude as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// A direction in the canonical IAU 1958 Galactic system's Hipparcos ICRS realization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GalacticDirection {
    longitude: GalacticLongitude,
    latitude: GalacticLatitude,
}

impl GalacticDirection {
    /// Constructs a Galactic direction.
    pub const fn new(longitude: GalacticLongitude, latitude: GalacticLatitude) -> Self {
        Self {
            longitude,
            latitude,
        }
    }

    /// Returns the Galactic longitude.
    pub const fn longitude(self) -> GalacticLongitude {
        self.longitude
    }

    /// Returns the Galactic latitude.
    pub const fn latitude(self) -> GalacticLatitude {
        self.latitude
    }

    /// Converts to a Cartesian unit direction on the canonical Galactic axes.
    pub fn to_direction(self) -> Result<Direction<Galactic>, MathError> {
        let longitude = self.longitude.as_radians();
        let latitude = self.latitude.as_radians();
        let latitude_cosine = cos(latitude);
        Direction::try_from_components([
            latitude_cosine * cos(longitude),
            latitude_cosine * sin(longitude),
            sin(latitude),
        ])
    }

    /// Converts a Cartesian unit direction on the canonical Galactic axes to angular coordinates.
    pub fn from_direction(direction: Direction<Galactic>) -> Result<Self, MathError> {
        let [x, y, z] = direction.components();
        let horizontal = sqrt(x * x + y * y);
        if horizontal == 0.0 {
            return Err(MathError::UndefinedLongitude);
        }
        Ok(Self::new(
            GalacticLongitude::wrap_radians(atan2(y, x))?,
            GalacticLatitude::try_from_radians(asin(z.clamp(-1.0, 1.0)))?,
        ))
    }

    /// Returns the stable great-circle separation from another Galactic direction.
    pub fn separation_to(self, rhs: Self) -> Result<Separation, MathError> {
        Separation::try_from_radians(
            self.to_direction()?
                .angle_to(rhs.to_direction()?)?
                .as_radians(),
        )
    }

    /// Converts an ICRS equatorial direction using the canonical Hipparcos rotation.
    #[cfg(feature = "std")]
    pub fn from_icrs(source: EquatorialDirection<Icrs>) -> Result<Self, super::Error> {
        let (longitude, latitude) = sofars::coords::icrs2g(
            source.right_ascension().as_radians(),
            source.declination().as_radians(),
        );
        Ok(Self::new(
            GalacticLongitude::wrap_radians(longitude)?,
            GalacticLatitude::try_from_radians(latitude)?,
        ))
    }

    /// Converts to an ICRS equatorial direction using the inverse canonical Hipparcos rotation.
    #[cfg(feature = "std")]
    pub fn to_icrs(self) -> Result<EquatorialDirection<Icrs>, super::Error> {
        let (right_ascension, declination) =
            sofars::coords::g2icrs(self.longitude.as_radians(), self.latitude.as_radians());
        Ok(EquatorialDirection::new(
            crate::math::RightAscension::wrap_radians(right_ascension)?,
            crate::math::Declination::try_from_radians(declination)?,
        ))
    }
}
