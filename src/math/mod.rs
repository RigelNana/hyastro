//! Strongly typed quantities, geometry, rotations, spherical algorithms, and numerical methods.

mod angle;
mod error;
mod matrix;
mod numeric;
mod photometry;
mod quantity;
mod rotation;
mod sexagesimal;
mod sphere;
mod vector;

pub use angle::{
    Altitude, Angle, Azimuth, Declination, HourAngle, Latitude, Longitude, PhaseAngle,
    PositionAngle, RightAscension, Separation, ZenithDistance,
};
pub use error::Error;
pub use matrix::Matrix3;
pub use numeric::{RootOptions, RootResult};
pub use photometry::{
    Ab, ApparentMagnitude, FluxRatio, JohnsonV, MagnitudeDifference, MagnitudeSystem,
    PhotometricPassband, St, Vega,
};
pub use quantity::{Acceleration, AngularSpeed, Coordinate, Dimensionless, Length, Speed, Squared};
pub use rotation::{Quaternion, Rotation, RotationTolerance};
pub use sexagesimal::{DegreesMinutesSeconds, HoursMinutesSeconds, SexagesimalSign};
pub use sphere::{SphericalDirection, TangentBasis};
pub use vector::{Direction, Point3, PointFrame, Vector3};
