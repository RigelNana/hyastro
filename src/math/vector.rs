use core::{fmt, marker::PhantomData};

use libm::{atan2, sqrt};

use super::{Angle, Coordinate, Dimensionless, Error, Length, Speed};

/// A three-dimensional vector whose frame and scalar quantity are type checked.
pub struct Vector3<F, Q: Coordinate> {
    components: [Q; 3],
    frame: PhantomData<F>,
}

impl<F, Q: Coordinate> Vector3<F, Q> {
    /// Constructs a vector from three coordinates.
    pub const fn new(x: Q, y: Q, z: Q) -> Self {
        Self {
            components: [x, y, z],
            frame: PhantomData,
        }
    }

    /// Constructs a vector from an array.
    pub const fn from_array(components: [Q; 3]) -> Self {
        Self {
            components,
            frame: PhantomData,
        }
    }

    /// Returns all components in x, y, z order.
    pub const fn components(self) -> [Q; 3] {
        self.components
    }

    /// Returns the x component.
    pub const fn x(self) -> Q {
        self.components[0]
    }

    /// Returns the y component.
    pub const fn y(self) -> Q {
        self.components[1]
    }

    /// Returns the z component.
    pub const fn z(self) -> Q {
        self.components[2]
    }

    /// Adds a vector with the same frame and quantity.
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        Self::from_canonical([
            self.x().canonical() + rhs.x().canonical(),
            self.y().canonical() + rhs.y().canonical(),
            self.z().canonical() + rhs.z().canonical(),
        ])
    }

    /// Subtracts a vector with the same frame and quantity.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        Self::from_canonical([
            self.x().canonical() - rhs.x().canonical(),
            self.y().canonical() - rhs.y().canonical(),
            self.z().canonical() - rhs.z().canonical(),
        ])
    }

    /// Scales the vector by a finite unitless factor.
    pub fn checked_scale(self, factor: f64) -> Result<Self, Error> {
        Error::ensure_finite("vector scale factor", factor)?;
        Self::from_canonical([
            self.x().canonical() * factor,
            self.y().canonical() * factor,
            self.z().canonical() * factor,
        ])
    }

    /// Computes a dimensionally typed dot product.
    pub fn dot(self, rhs: Self) -> Result<Q::Product, Error> {
        Q::Product::try_from_canonical(
            self.x().canonical() * rhs.x().canonical()
                + self.y().canonical() * rhs.y().canonical()
                + self.z().canonical() * rhs.z().canonical(),
        )
    }

    /// Computes a dimensionally typed cross product.
    pub fn cross(self, rhs: Self) -> Result<Vector3<F, Q::Product>, Error> {
        Vector3::from_canonical([
            self.y().canonical() * rhs.z().canonical() - self.z().canonical() * rhs.y().canonical(),
            self.z().canonical() * rhs.x().canonical() - self.x().canonical() * rhs.z().canonical(),
            self.x().canonical() * rhs.y().canonical() - self.y().canonical() * rhs.x().canonical(),
        ])
    }

    /// Returns the magnitude in the vector's scalar quantity.
    pub fn magnitude(self) -> Result<Q, Error> {
        let squared = self.x().canonical() * self.x().canonical()
            + self.y().canonical() * self.y().canonical()
            + self.z().canonical() * self.z().canonical();
        Q::try_from_canonical(sqrt(squared))
    }

    /// Returns the normalized direction of a non-zero vector.
    pub fn direction(self) -> Result<Direction<F>, Error> {
        Direction::try_from_components([
            self.x().canonical(),
            self.y().canonical(),
            self.z().canonical(),
        ])
    }

    /// Returns the stable angle to another non-zero vector.
    pub fn angle_to(self, rhs: Self) -> Result<Angle, Error> {
        let left = self.direction()?;
        let right = rhs.direction()?;
        left.angle_to(right)
    }

    /// Projects the vector onto a direction in the same frame.
    pub fn project_onto(self, direction: Direction<F>) -> Result<Self, Error> {
        let unit = direction.components();
        let scale = self.x().canonical() * unit[0]
            + self.y().canonical() * unit[1]
            + self.z().canonical() * unit[2];
        Self::from_canonical([unit[0] * scale, unit[1] * scale, unit[2] * scale])
    }

    /// Rejects the component parallel to a direction in the same frame.
    pub fn reject_from(self, direction: Direction<F>) -> Result<Self, Error> {
        self.checked_sub(self.project_onto(direction)?)
    }

    pub(crate) fn from_canonical(components: [f64; 3]) -> Result<Self, Error> {
        Ok(Self::new(
            Q::try_from_canonical(components[0])?,
            Q::try_from_canonical(components[1])?,
            Q::try_from_canonical(components[2])?,
        ))
    }

    pub(crate) fn canonical_components(self) -> [f64; 3] {
        [
            self.x().canonical(),
            self.y().canonical(),
            self.z().canonical(),
        ]
    }
}

impl<F, Q: Coordinate> Copy for Vector3<F, Q> {}

impl<F, Q: Coordinate> Clone for Vector3<F, Q> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F, Q: Coordinate> PartialEq for Vector3<F, Q> {
    fn eq(&self, other: &Self) -> bool {
        self.components == other.components
    }
}

impl<F, Q: Coordinate> fmt::Debug for Vector3<F, Q> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Vector3")
            .field(&self.components)
            .finish()
    }
}

/// A finite unit direction associated with a coordinate frame.
pub struct Direction<F> {
    components: [f64; 3],
    frame: PhantomData<F>,
}

impl<F> Direction<F> {
    /// Constructs and normalizes a direction from Cartesian components.
    pub fn try_from_components(components: [f64; 3]) -> Result<Self, Error> {
        Error::ensure_finite("direction x", components[0])?;
        Error::ensure_finite("direction y", components[1])?;
        Error::ensure_finite("direction z", components[2])?;
        let norm = sqrt(
            components[0] * components[0]
                + components[1] * components[1]
                + components[2] * components[2],
        );
        if norm == 0.0 {
            return Err(Error::ZeroVector);
        }
        Error::ensure_finite("direction norm", norm)?;
        Ok(Self {
            components: [
                components[0] / norm,
                components[1] / norm,
                components[2] / norm,
            ],
            frame: PhantomData,
        })
    }

    /// Returns the unit Cartesian components.
    pub const fn components(self) -> [f64; 3] {
        self.components
    }

    /// Returns the direction as a dimensionless vector.
    pub fn as_vector(self) -> Vector3<F, Dimensionless> {
        Vector3::new(
            Dimensionless::from_finite(self.components[0]),
            Dimensionless::from_finite(self.components[1]),
            Dimensionless::from_finite(self.components[2]),
        )
    }

    /// Returns the dot product with another direction in the same frame.
    pub fn dot(self, rhs: Self) -> f64 {
        self.components[0] * rhs.components[0]
            + self.components[1] * rhs.components[1]
            + self.components[2] * rhs.components[2]
    }

    /// Returns the stable angle to another direction.
    pub fn angle_to(self, rhs: Self) -> Result<Angle, Error> {
        let cross = [
            self.components[1] * rhs.components[2] - self.components[2] * rhs.components[1],
            self.components[2] * rhs.components[0] - self.components[0] * rhs.components[2],
            self.components[0] * rhs.components[1] - self.components[1] * rhs.components[0],
        ];
        let cross_norm = sqrt(cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]);
        Angle::from_radians(atan2(cross_norm, self.dot(rhs)))
    }
}

impl<F> Copy for Direction<F> {}

impl<F> Clone for Direction<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> PartialEq for Direction<F> {
    fn eq(&self, other: &Self) -> bool {
        self.components == other.components
    }
}

impl<F> fmt::Debug for Direction<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Direction")
            .field(&self.components)
            .finish()
    }
}

/// A Cartesian point whose frame and origin are encoded in its type.
pub struct Point3<F, O> {
    coordinates: Vector3<F, Length>,
    origin: PhantomData<O>,
}

impl<F, O> Point3<F, O> {
    /// Constructs a point from Cartesian coordinates.
    pub const fn new(x: Length, y: Length, z: Length) -> Self {
        Self {
            coordinates: Vector3::new(x, y, z),
            origin: PhantomData,
        }
    }

    /// Returns the position vector relative to the typed origin.
    pub const fn position(self) -> Vector3<F, Length> {
        self.coordinates
    }

    /// Translates the point by a displacement in the same frame.
    pub fn checked_translate(self, displacement: Vector3<F, Length>) -> Result<Self, Error> {
        let translated = self.coordinates.checked_add(displacement)?;
        Ok(Self {
            coordinates: translated,
            origin: PhantomData,
        })
    }

    /// Returns the displacement from another point with the same origin.
    pub fn displacement_from(self, other: Self) -> Result<Vector3<F, Length>, Error> {
        self.coordinates.checked_sub(other.coordinates)
    }
}

impl<F, O> Copy for Point3<F, O> {}

impl<F, O> Clone for Point3<F, O> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F, O> PartialEq for Point3<F, O> {
    fn eq(&self, other: &Self) -> bool {
        self.coordinates == other.coordinates
    }
}

impl<F, O> fmt::Debug for Point3<F, O> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Point3")
            .field(&self.coordinates)
            .finish()
    }
}

/// A position and velocity in one frame, origin, and epoch type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State<F, O, E> {
    position: Point3<F, O>,
    velocity: Vector3<F, Speed>,
    epoch: E,
}

impl<F, O, E> State<F, O, E> {
    /// Constructs a state from position, velocity, and epoch.
    pub const fn new(position: Point3<F, O>, velocity: Vector3<F, Speed>, epoch: E) -> Self {
        Self {
            position,
            velocity,
            epoch,
        }
    }

    /// Returns the state position.
    pub const fn position(&self) -> &Point3<F, O> {
        &self.position
    }

    /// Returns the state velocity.
    pub const fn velocity(&self) -> &Vector3<F, Speed> {
        &self.velocity
    }

    /// Returns the state epoch.
    pub const fn epoch(&self) -> &E {
        &self.epoch
    }

    /// Decomposes the state into position, velocity, and epoch.
    pub fn into_parts(self) -> (Point3<F, O>, Vector3<F, Speed>, E) {
        (self.position, self.velocity, self.epoch)
    }
}
