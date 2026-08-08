use core::{fmt, marker::PhantomData};

use libm::{asin, atan2, cos, sin, sqrt};

#[cfg(feature = "std")]
use crate::time::{JulianDate, Tt};
use crate::{
    math::{
        Declination, Direction, Error as MathError, Latitude, Longitude, RightAscension,
        Separation, SphericalDirection,
    },
    time::{Instant, TimeScale},
};

use super::EquatorialAxes;
#[cfg(feature = "std")]
use super::{Gcrs, MeanEquatorEquinoxJ2000};

/// A right ascension and declination describing a unit direction on specified equatorial axes.
pub struct EquatorialDirection<F: EquatorialAxes> {
    right_ascension: RightAscension,
    declination: Declination,
    axes: PhantomData<F>,
}

impl<F: EquatorialAxes> EquatorialDirection<F> {
    /// Constructs an equatorial direction.
    pub const fn new(right_ascension: RightAscension, declination: Declination) -> Self {
        Self {
            right_ascension,
            declination,
            axes: PhantomData,
        }
    }

    /// Returns the right ascension.
    pub const fn right_ascension(self) -> RightAscension {
        self.right_ascension
    }

    /// Returns the declination.
    pub const fn declination(self) -> Declination {
        self.declination
    }

    /// Converts to a Cartesian unit direction on the same axes.
    pub fn to_direction(self) -> Result<Direction<F>, MathError> {
        let right_ascension = self.right_ascension.as_radians();
        let declination = self.declination.as_radians();
        let declination_cosine = cos(declination);
        Direction::try_from_components([
            declination_cosine * cos(right_ascension),
            declination_cosine * sin(right_ascension),
            sin(declination),
        ])
    }

    /// Converts a Cartesian unit direction on the same axes to equatorial coordinates.
    pub fn from_direction(direction: Direction<F>) -> Result<Self, MathError> {
        let [x, y, z] = direction.components();
        let horizontal = sqrt(x * x + y * y);
        if horizontal == 0.0 {
            return Err(MathError::UndefinedLongitude);
        }
        Ok(Self::new(
            RightAscension::wrap_radians(atan2(y, x))?,
            Declination::try_from_radians(asin(z.clamp(-1.0, 1.0)))?,
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

    /// Converts to generic longitude and latitude semantics without changing the axes.
    pub fn to_spherical(self) -> Result<SphericalDirection<F>, MathError> {
        Ok(SphericalDirection::new(
            Longitude::wrap_radians(self.right_ascension.as_radians())?,
            Latitude::try_from_radians(self.declination.as_radians())?,
        ))
    }
}
#[cfg(feature = "std")]
impl EquatorialDirection<MeanEquatorEquinoxJ2000> {
    /// Converts a GCRS direction to IAU 2006 mean equator and equinox of J2000.0.
    pub fn from_gcrs(source: EquatorialDirection<Gcrs>) -> Result<Self, super::Error> {
        let (frame_bias, _, _) = sofars::pnp::bp06(JulianDate::<Tt>::J2000_VALUE, 0.0);
        let mut output = [0.0; 3];
        sofars::vm::rxp(
            &frame_bias,
            &source.to_direction()?.components(),
            &mut output,
        );
        Ok(Self::from_direction(Direction::try_from_components(
            output,
        )?)?)
    }

    /// Converts mean equator and equinox of J2000.0 coordinates back to GCRS.
    pub fn to_gcrs(self) -> Result<EquatorialDirection<Gcrs>, super::Error> {
        let (frame_bias, _, _) = sofars::pnp::bp06(JulianDate::<Tt>::J2000_VALUE, 0.0);
        let mut output = [0.0; 3];
        sofars::vm::trxp(&frame_bias, &self.to_direction()?.components(), &mut output);
        Ok(EquatorialDirection::from_direction(
            Direction::try_from_components(output)?,
        )?)
    }
}

impl<F: EquatorialAxes> Copy for EquatorialDirection<F> {}

impl<F: EquatorialAxes> Clone for EquatorialDirection<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F: EquatorialAxes> PartialEq for EquatorialDirection<F> {
    fn eq(&self, other: &Self) -> bool {
        self.right_ascension == other.right_ascension && self.declination == other.declination
    }
}

impl<F: EquatorialAxes> fmt::Debug for EquatorialDirection<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EquatorialDirection")
            .field("axes", &F::NAME)
            .field("right_ascension", &self.right_ascension)
            .field("declination", &self.declination)
            .finish()
    }
}

/// An equatorial direction associated with one physical evaluation epoch.
pub struct EquatorialDirectionAt<F, S>
where
    F: EquatorialAxes,
    S: TimeScale,
{
    epoch: Instant<S>,
    coordinates: EquatorialDirection<F>,
}

impl<F, S> EquatorialDirectionAt<F, S>
where
    F: EquatorialAxes,
    S: TimeScale,
{
    /// Associates equatorial coordinates with their physical evaluation epoch.
    pub const fn new(epoch: Instant<S>, coordinates: EquatorialDirection<F>) -> Self {
        Self { epoch, coordinates }
    }

    /// Returns the physical evaluation epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the equatorial coordinates.
    pub const fn coordinates(self) -> EquatorialDirection<F> {
        self.coordinates
    }

    /// Decomposes the result into its epoch and coordinates.
    pub const fn into_parts(self) -> (Instant<S>, EquatorialDirection<F>) {
        (self.epoch, self.coordinates)
    }
}

impl<F, S> Copy for EquatorialDirectionAt<F, S>
where
    F: EquatorialAxes,
    S: TimeScale,
{
}

impl<F, S> Clone for EquatorialDirectionAt<F, S>
where
    F: EquatorialAxes,
    S: TimeScale,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<F, S> PartialEq for EquatorialDirectionAt<F, S>
where
    F: EquatorialAxes,
    S: TimeScale,
{
    fn eq(&self, other: &Self) -> bool {
        self.epoch == other.epoch && self.coordinates == other.coordinates
    }
}

impl<F, S> fmt::Debug for EquatorialDirectionAt<F, S>
where
    F: EquatorialAxes,
    S: TimeScale,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EquatorialDirectionAt")
            .field("epoch", &self.epoch)
            .field("coordinates", &self.coordinates)
            .finish()
    }
}
