use core::{f64::consts::PI, fmt, marker::PhantomData};

use libm::{asin, atan2, cos, sin, sqrt};

use super::{Direction, Error, Latitude, Longitude, PositionAngle, Separation};

/// A longitude and latitude describing a unit direction in a typed frame.
pub struct SphericalDirection<F> {
    longitude: Longitude,
    latitude: Latitude,
    frame: PhantomData<F>,
}

impl<F> SphericalDirection<F> {
    /// Constructs a spherical direction from semantic longitude and latitude.
    pub const fn new(longitude: Longitude, latitude: Latitude) -> Self {
        Self {
            longitude,
            latitude,
            frame: PhantomData,
        }
    }

    /// Returns the longitude.
    pub const fn longitude(self) -> Longitude {
        self.longitude
    }

    /// Returns the latitude.
    pub const fn latitude(self) -> Latitude {
        self.latitude
    }

    /// Converts to a Cartesian unit direction.
    pub fn to_direction(self) -> Result<Direction<F>, Error> {
        let longitude = self.longitude.as_radians();
        let latitude = self.latitude.as_radians();
        let latitude_cosine = cos(latitude);
        Direction::try_from_components([
            latitude_cosine * cos(longitude),
            latitude_cosine * sin(longitude),
            sin(latitude),
        ])
    }

    /// Converts a Cartesian unit direction to spherical coordinates.
    pub fn from_direction(direction: Direction<F>) -> Result<Self, Error> {
        let [x, y, z] = direction.components();
        let horizontal = sqrt(x * x + y * y);
        if horizontal == 0.0 {
            return Err(Error::UndefinedLongitude);
        }
        Ok(Self::new(
            Longitude::wrap_radians(atan2(y, x))?,
            Latitude::try_from_radians(asin(z.clamp(-1.0, 1.0)))?,
        ))
    }

    /// Returns a stable great-circle separation from another direction.
    pub fn separation_to(self, rhs: Self) -> Result<Separation, Error> {
        let left = self.to_direction()?;
        let right = rhs.to_direction()?;
        Separation::try_from_radians(left.angle_to(right)?.as_radians())
    }

    /// Returns the initial position angle toward another direction.
    pub fn position_angle_to(self, rhs: Self) -> Result<PositionAngle, Error> {
        let source = self.to_direction()?;
        let target = rhs.to_direction()?;
        let dot = source.dot(target).clamp(-1.0, 1.0);
        if dot == 1.0 {
            return Err(Error::UndefinedPositionAngle);
        }
        if dot == -1.0 {
            return Err(Error::AntipodalDirections);
        }
        let basis = self.tangent_basis()?;
        let east = target.dot(basis.east);
        let north = target.dot(basis.north);
        if east == 0.0 && north == 0.0 {
            return Err(Error::UndefinedPositionAngle);
        }
        PositionAngle::wrap_radians(atan2(east, north))
    }

    /// Returns the destination reached along a great circle.
    pub fn destination(
        self,
        separation: Separation,
        position_angle: PositionAngle,
    ) -> Result<Self, Error> {
        let source = self.to_direction()?;
        let basis = self.tangent_basis()?;
        let distance = separation.as_radians();
        let bearing = position_angle.as_radians();
        let source_components = source.components();
        let east = basis.east.components();
        let north = basis.north.components();
        let tangent = [
            north[0] * cos(bearing) + east[0] * sin(bearing),
            north[1] * cos(bearing) + east[1] * sin(bearing),
            north[2] * cos(bearing) + east[2] * sin(bearing),
        ];
        let destination = Direction::try_from_components([
            source_components[0] * cos(distance) + tangent[0] * sin(distance),
            source_components[1] * cos(distance) + tangent[1] * sin(distance),
            source_components[2] * cos(distance) + tangent[2] * sin(distance),
        ])?;
        Self::from_direction(destination)
    }

    /// Interpolates along the unique shortest great-circle arc.
    pub fn slerp(self, rhs: Self, fraction: f64) -> Result<Self, Error> {
        Error::ensure_finite("spherical interpolation fraction", fraction)?;
        if !(0.0..=1.0).contains(&fraction) {
            return Err(Error::OutOfRange {
                field: "spherical interpolation fraction",
                value: fraction,
                interval: "[0, 1]",
                unit: "",
            });
        }

        let left = self.to_direction()?;
        let right = rhs.to_direction()?;
        let angle = left.angle_to(right)?.as_radians();
        if angle == 0.0 {
            return Ok(self);
        }
        if (PI - angle).abs() <= f64::EPSILON {
            return Err(Error::AntipodalDirections);
        }

        let denominator = sin(angle);
        let left_weight = sin((1.0 - fraction) * angle) / denominator;
        let right_weight = sin(fraction * angle) / denominator;
        let left_components = left.components();
        let right_components = right.components();
        Self::from_direction(Direction::try_from_components([
            left_weight * left_components[0] + right_weight * right_components[0],
            left_weight * left_components[1] + right_weight * right_components[1],
            left_weight * left_components[2] + right_weight * right_components[2],
        ])?)
    }

    /// Returns the local east and north tangent directions.
    pub fn tangent_basis(self) -> Result<TangentBasis<F>, Error> {
        let longitude = self.longitude.as_radians();
        let latitude = self.latitude.as_radians();
        Ok(TangentBasis {
            east: Direction::try_from_components([-sin(longitude), cos(longitude), 0.0])?,
            north: Direction::try_from_components([
                -sin(latitude) * cos(longitude),
                -sin(latitude) * sin(longitude),
                cos(latitude),
            ])?,
        })
    }
}
impl<F> Copy for SphericalDirection<F> {}

impl<F> Clone for SphericalDirection<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> PartialEq for SphericalDirection<F> {
    fn eq(&self, other: &Self) -> bool {
        self.longitude == other.longitude && self.latitude == other.latitude
    }
}

impl<F> fmt::Debug for SphericalDirection<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SphericalDirection")
            .field("longitude", &self.longitude)
            .field("latitude", &self.latitude)
            .finish()
    }
}

/// Local east and north directions tangent to the unit sphere.
pub struct TangentBasis<F> {
    east: Direction<F>,
    north: Direction<F>,
}

impl<F> TangentBasis<F> {
    /// Returns the local east direction.
    pub const fn east(self) -> Direction<F> {
        self.east
    }

    /// Returns the local north direction.
    pub const fn north(self) -> Direction<F> {
        self.north
    }
}

impl<F> Copy for TangentBasis<F> {}

impl<F> Clone for TangentBasis<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> PartialEq for TangentBasis<F> {
    fn eq(&self, other: &Self) -> bool {
        self.east == other.east && self.north == other.north
    }
}

impl<F> fmt::Debug for TangentBasis<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TangentBasis")
            .field("east", &self.east)
            .field("north", &self.north)
            .finish()
    }
}
