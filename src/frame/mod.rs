//! Static astronomical coordinate frames and epoch-bound state transforms.

#[cfg(feature = "std")]
mod celestial;
#[cfg(feature = "std")]
mod earth_orientation;
mod ecliptic;
mod equatorial;
mod error;
#[cfg(feature = "std")]
mod frames;
mod galactic;
mod rotation;
mod state;
mod system;
mod transform;

#[cfg(feature = "std")]
pub use celestial::CelestialOrientationSolution;
#[cfg(feature = "std")]
pub use earth_orientation::{
    CelestialIntermediatePole, EarthOrientationSolution, FukushimaWilliamsAngles,
    PrecessionNutation, SiderealTimeSolution,
};
pub use ecliptic::{EclipticDirection, EclipticDirectionAt, EclipticLatitude, EclipticLongitude};
pub use equatorial::{EquatorialDirection, EquatorialDirectionAt};
pub use error::Error;
#[cfg(feature = "std")]
pub use frames::{Frames, StateTransformModel};
pub use galactic::{GalacticDirection, GalacticLatitude, GalacticLongitude};
pub use rotation::FrameRotation;
pub use state::State;
pub use system::{
    Axes, Bcrs, Cirs, CoordinateFrame, EarthCenter, EclipticAxes, EquatorialAxes, Equinox,
    FrameDefinition, Galactic, Gcrs, Handedness, Icrs, Itrs, MeanEclipticEquinoxJ2000,
    MeanEclipticEquinoxOfDate, MeanEquatorEquinoxJ2000, MeanEquatorEquinoxOfDate, Origin, OriginId,
    ReferenceEpoch, ReferenceSystem, ReferenceSystemId, SolarSystemBarycenter, Tirs,
    TrueEclipticEquinoxOfDate, TrueEquatorEquinoxOfDate,
};
pub use transform::StateTransform;
