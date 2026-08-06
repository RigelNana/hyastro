//! Strongly typed quantities, geometry, rotations, spherical algorithms, and numerical methods.

mod angle;
mod error;
mod matrix;
mod numeric;
mod quantity;
mod rotation;
mod sphere;
mod vector;

pub use angle::{
    Altitude, Angle, Azimuth, Declination, HourAngle, Latitude, Longitude, PhaseAngle,
    PositionAngle, RightAscension, Separation, ZenithDistance,
};
pub use error::Error;
pub use matrix::Matrix3;
pub use numeric::{RootOptions, RootResult};
pub use quantity::{Acceleration, AngularSpeed, Coordinate, Dimensionless, Length, Speed, Squared};
pub use rotation::{Quaternion, Rotation, RotationTolerance};
pub use sphere::{EquatorialDirection, SphericalDirection, TangentBasis};
pub use vector::{Direction, Point3, Vector3};
