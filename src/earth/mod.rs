//! Reference ellipsoids, geodetic coordinates, fixed sites, and local tangent frames.
//!
//! [`Earth`] is the primary entry point. It binds every conversion and site to
//! one explicit [`ReferenceEllipsoid`], preventing accidental WGS 84/GRS 80 or
//! custom-model mixing.

mod ellipsoid;
mod error;
mod model;
mod position;
mod site;

pub use ellipsoid::ReferenceEllipsoid;
pub use error::Error;
pub use model::Earth;
pub use position::{
    EllipsoidalHeight, GeocentricLatitude, GeodeticLatitude, GeodeticLongitude, GeodeticPosition,
};
pub use site::{EastNorthUp, FixedSite, NorthEastDown, SiteVelocityModel, TopocentricFrame};
