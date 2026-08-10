use core::{
    f64::consts::{FRAC_PI_2, TAU},
    fmt,
    marker::PhantomData,
};

use libm::{asin, atan2, cos, sin, sqrt};

use crate::{
    math::{Angle, DegreesMinutesSeconds, Direction, Error as MathError, Separation},
    time::{Instant, TimeScale},
};

#[cfg(feature = "std")]
use crate::time::{JulianDate, Tt};

use super::EclipticAxes;
#[cfg(feature = "std")]
use super::{EquatorialDirection, Icrs, MeanEclipticEquinoxJ2000};

/// An ecliptic longitude in the half-open interval [0, 2π).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EclipticLongitude(f64);

impl EclipticLongitude {
    #[cfg(feature = "std")]
    pub(crate) const fn from_validated_radians(value: f64) -> Self {
        Self(value)
    }

    /// Constructs an ecliptic longitude from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("ecliptic longitude", value)?;
        if (0.0..TAU).contains(&value) {
            Ok(Self(value))
        } else {
            Err(MathError::OutOfRange {
                field: "ecliptic longitude",
                value,
                interval: "[0, 2π)",
                unit: "rad",
            })
        }
    }

    /// Constructs an ecliptic longitude from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("ecliptic longitude", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Normalizes radians into the ecliptic-longitude interval.
    pub fn wrap_radians(value: f64) -> Result<Self, MathError> {
        Angle::wrap_zero_tau(value, "ecliptic longitude").map(Self)
    }

    /// Normalizes degrees into the ecliptic-longitude interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("ecliptic longitude", value)?;
        Self::wrap_radians(value.to_radians())
    }

    /// Constructs an ecliptic longitude from DMS without normalization.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, MathError> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the ecliptic longitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the ecliptic longitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle::from_finite(self.0)
    }

    /// Returns the ecliptic longitude as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// An ecliptic latitude in the closed interval [-π/2, π/2].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EclipticLatitude(f64);

impl EclipticLatitude {
    /// Constructs an ecliptic latitude from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("ecliptic latitude", value)?;
        if (-FRAC_PI_2..=FRAC_PI_2).contains(&value) {
            Ok(Self(value))
        } else {
            Err(MathError::OutOfRange {
                field: "ecliptic latitude",
                value,
                interval: "[-π/2, π/2]",
                unit: "rad",
            })
        }
    }

    /// Constructs an ecliptic latitude from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("ecliptic latitude", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Constructs an ecliptic latitude from DMS.
    pub fn try_from_dms(value: DegreesMinutesSeconds) -> Result<Self, MathError> {
        Self::try_from_degrees(value.as_decimal_degrees())
    }

    /// Returns the ecliptic latitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the ecliptic latitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle::from_finite(self.0)
    }

    /// Returns the ecliptic latitude as DMS.
    pub fn to_dms(self) -> DegreesMinutesSeconds {
        DegreesMinutesSeconds::from_semantic_decimal_degrees(self.as_degrees())
    }
}

/// An ecliptic longitude and latitude describing a unit direction on specified ecliptic axes.
pub struct EclipticDirection<F: EclipticAxes> {
    longitude: EclipticLongitude,
    latitude: EclipticLatitude,
    axes: PhantomData<F>,
}

impl<F: EclipticAxes> EclipticDirection<F> {
    /// Constructs an ecliptic direction.
    pub const fn new(longitude: EclipticLongitude, latitude: EclipticLatitude) -> Self {
        Self {
            longitude,
            latitude,
            axes: PhantomData,
        }
    }

    /// Returns the ecliptic longitude.
    pub const fn longitude(self) -> EclipticLongitude {
        self.longitude
    }

    /// Returns the ecliptic latitude.
    pub const fn latitude(self) -> EclipticLatitude {
        self.latitude
    }

    /// Converts to a Cartesian unit direction on the same axes.
    pub fn to_direction(self) -> Result<Direction<F>, MathError> {
        let longitude = self.longitude.as_radians();
        let latitude = self.latitude.as_radians();
        let latitude_cosine = cos(latitude);
        Direction::try_from_components([
            latitude_cosine * cos(longitude),
            latitude_cosine * sin(longitude),
            sin(latitude),
        ])
    }

    /// Converts a Cartesian unit direction on the same axes to ecliptic coordinates.
    pub fn from_direction(direction: Direction<F>) -> Result<Self, MathError> {
        let [x, y, z] = direction.components();
        let horizontal = sqrt(x * x + y * y);
        if horizontal == 0.0 {
            return Err(MathError::UndefinedLongitude);
        }
        Ok(Self::new(
            EclipticLongitude::wrap_radians(atan2(y, x))?,
            EclipticLatitude::try_from_radians(asin(z.clamp(-1.0, 1.0)))?,
        ))
    }

    /// Returns the stable great-circle separation from another direction on the same axes.
    pub fn separation_to(self, rhs: Self) -> Result<Separation, MathError> {
        Separation::try_from_radians(
            self.to_direction()?
                .angle_to(rhs.to_direction()?)?
                .as_radians(),
        )
    }
}

#[cfg(feature = "std")]
impl EclipticDirection<MeanEclipticEquinoxJ2000> {
    /// Converts an ICRS equatorial direction to IAU 2006 mean J2000.0 ecliptic coordinates.
    pub fn from_icrs(source: EquatorialDirection<Icrs>) -> Result<Self, super::Error> {
        let (longitude, latitude) = sofars::coords::eqec06(
            JulianDate::<Tt>::J2000_VALUE,
            0.0,
            source.right_ascension().as_radians(),
            source.declination().as_radians(),
        );
        Ok(Self::new(
            EclipticLongitude::wrap_radians(longitude)?,
            EclipticLatitude::try_from_radians(latitude)?,
        ))
    }

    /// Converts IAU 2006 mean J2000.0 ecliptic coordinates to an ICRS equatorial direction.
    pub fn to_icrs(self) -> Result<EquatorialDirection<Icrs>, super::Error> {
        let (right_ascension, declination) = sofars::coords::eceq06(
            JulianDate::<Tt>::J2000_VALUE,
            0.0,
            self.longitude.as_radians(),
            self.latitude.as_radians(),
        );
        Ok(EquatorialDirection::new(
            crate::math::RightAscension::wrap_radians(right_ascension)?,
            crate::math::Declination::try_from_radians(declination)?,
        ))
    }
}

impl<F: EclipticAxes> Copy for EclipticDirection<F> {}

impl<F: EclipticAxes> Clone for EclipticDirection<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F: EclipticAxes> PartialEq for EclipticDirection<F> {
    fn eq(&self, other: &Self) -> bool {
        self.longitude == other.longitude && self.latitude == other.latitude
    }
}

impl<F: EclipticAxes> fmt::Debug for EclipticDirection<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EclipticDirection")
            .field("axes", &F::NAME)
            .field("longitude", &self.longitude)
            .field("latitude", &self.latitude)
            .finish()
    }
}

/// An ecliptic direction associated with one physical evaluation epoch.
pub struct EclipticDirectionAt<F, S>
where
    F: EclipticAxes,
    S: TimeScale,
{
    epoch: Instant<S>,
    coordinates: EclipticDirection<F>,
}

impl<F, S> EclipticDirectionAt<F, S>
where
    F: EclipticAxes,
    S: TimeScale,
{
    /// Associates ecliptic coordinates with their physical evaluation epoch.
    pub const fn new(epoch: Instant<S>, coordinates: EclipticDirection<F>) -> Self {
        Self { epoch, coordinates }
    }

    /// Returns the physical evaluation epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the ecliptic coordinates.
    pub const fn coordinates(self) -> EclipticDirection<F> {
        self.coordinates
    }

    /// Decomposes the result into its epoch and coordinates.
    pub const fn into_parts(self) -> (Instant<S>, EclipticDirection<F>) {
        (self.epoch, self.coordinates)
    }
}

impl<F, S> Copy for EclipticDirectionAt<F, S>
where
    F: EclipticAxes,
    S: TimeScale,
{
}

impl<F, S> Clone for EclipticDirectionAt<F, S>
where
    F: EclipticAxes,
    S: TimeScale,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<F, S> PartialEq for EclipticDirectionAt<F, S>
where
    F: EclipticAxes,
    S: TimeScale,
{
    fn eq(&self, other: &Self) -> bool {
        self.epoch == other.epoch && self.coordinates == other.coordinates
    }
}

impl<F, S> fmt::Debug for EclipticDirectionAt<F, S>
where
    F: EclipticAxes,
    S: TimeScale,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EclipticDirectionAt")
            .field("epoch", &self.epoch)
            .field("coordinates", &self.coordinates)
            .finish()
    }
}
