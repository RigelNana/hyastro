//! Strongly typed ephemeris queries and relative Cartesian states.

mod body;
mod error;
mod figure;
mod model;
#[cfg(feature = "std")]
mod sofa;

#[cfg(feature = "anise")]
mod anise;

#[cfg(feature = "anise")]
pub use anise::{Ephemeris, Kernel, KernelManifest};
pub use body::CelestialBody;
pub use error::Error;
pub use figure::SphericalBodyFigure;
pub use model::{Coverage, EphemerisProvenance, EphemerisProvider, EphemerisQuery, RelativeState};
#[cfg(feature = "std")]
pub use sofa::{Plan94Accuracy, SofaAnalyticEphemeris};
