//! Static astronomical coordinate frames and epoch-bound state transforms.

mod error;
#[cfg(feature = "std")]
mod frames;
mod rotation;
mod state;
mod system;
mod transform;

pub use error::Error;
#[cfg(feature = "std")]
pub use frames::{Frames, StateTransformModel};
pub use rotation::FrameRotation;
pub use state::State;
pub use system::{
    Axes, Bcrs, Cirs, CoordinateFrame, EarthCenter, Equinox, FrameDefinition, Gcrs, Handedness,
    Icrs, Itrs, Origin, OriginId, ReferenceEpoch, ReferenceSystem, ReferenceSystemId,
    SolarSystemBarycenter, Tirs,
};
pub use transform::StateTransform;
