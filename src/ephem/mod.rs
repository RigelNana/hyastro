//! Strongly typed ephemeris queries and relative Cartesian states.

mod body;
mod error;
mod model;

#[cfg(feature = "anise")]
mod anise;

#[cfg(feature = "anise")]
pub use anise::{Ephemeris, Kernel, KernelManifest};
pub use body::CelestialBody;
pub use error::Error;
pub use model::{Coverage, EphemerisQuery, RelativeState};
