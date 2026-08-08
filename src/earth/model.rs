use crate::{
    frame::Itrs,
    math::{Length, Point3},
};

use super::{
    EllipsoidalHeight, Error, GeocentricLatitude, GeodeticLatitude, GeodeticLongitude,
    GeodeticPosition, ReferenceEllipsoid,
};

/// Earth-shape and geodetic-coordinate algorithms bound to one reference ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Earth {
    ellipsoid: ReferenceEllipsoid,
}

impl Earth {
    /// Constructs Earth algorithms using the WGS 84 reference ellipsoid.
    pub const fn wgs84() -> Self {
        Self::new(ReferenceEllipsoid::WGS84)
    }

    /// Constructs Earth algorithms using the GRS 80 reference ellipsoid.
    pub const fn grs80() -> Self {
        Self::new(ReferenceEllipsoid::GRS80)
    }

    /// Constructs Earth algorithms using a validated reference ellipsoid.
    pub const fn new(ellipsoid: ReferenceEllipsoid) -> Self {
        Self { ellipsoid }
    }

    /// Returns the reference ellipsoid used by these algorithms.
    pub const fn reference_ellipsoid(self) -> ReferenceEllipsoid {
        self.ellipsoid
    }

    /// Converts geodetic coordinates to an ITRS Cartesian position.
    ///
    /// The result is expressed in metres. Longitude is east-positive and
    /// ellipsoidal height is measured along the ellipsoid normal.
    pub fn itrs_position(self, position: GeodeticPosition) -> Result<Point3<Itrs>, Error> {
        let ellipsoid = self.ellipsoid;
        let components = sofars::coords::gd2gce(
            ellipsoid.semi_major_axis().as_metres(),
            ellipsoid.flattening(),
            position.longitude().as_radians(),
            position.latitude().as_radians(),
            position.height().as_metres(),
        )
        .map_err(|status| Error::GeodeticConversionFailed {
            operation: "converting geodetic coordinates to ITRS",
            status,
        })?;

        Ok(Point3::new(
            Length::from_metres(components[0])?,
            Length::from_metres(components[1])?,
            Length::from_metres(components[2])?,
        ))
    }

    /// Converts an ITRS Cartesian position to geodetic coordinates.
    ///
    /// The Fukushima (2006) algorithm used by SOFA remains stable at the poles,
    /// for points below the ellipsoid, and for high-altitude points. The exact
    /// geocentric origin is rejected because its longitude and latitude are not
    /// unique.
    pub fn geodetic_position(self, position: Point3<Itrs>) -> Result<GeodeticPosition, Error> {
        let components = position.position().components().map(Length::as_metres);
        Self::ensure_not_origin(components)?;

        let ellipsoid = self.ellipsoid;
        let (longitude, latitude, height) = sofars::coords::gc2gde(
            ellipsoid.semi_major_axis().as_metres(),
            ellipsoid.flattening(),
            components,
        )
        .map_err(|status| Error::GeodeticConversionFailed {
            operation: "converting an ITRS position to geodetic coordinates",
            status,
        })?;

        Ok(GeodeticPosition::new(
            GeodeticLongitude::wrap_radians(longitude)?,
            GeodeticLatitude::try_from_radians(latitude)?,
            EllipsoidalHeight::from_metres(height)?,
        ))
    }

    /// Returns the geocentric latitude of a non-origin ITRS position.
    pub fn geocentric_latitude(self, position: Point3<Itrs>) -> Result<GeocentricLatitude, Error> {
        let [x, y, z] = position.position().components().map(Length::as_metres);
        Self::ensure_not_origin([x, y, z])?;
        GeocentricLatitude::try_from_radians(z.atan2(x.hypot(y)))
    }

    fn ensure_not_origin(components: [f64; 3]) -> Result<(), Error> {
        if components == [0.0, 0.0, 0.0] {
            Err(Error::UndefinedGeodeticPosition)
        } else {
            Ok(())
        }
    }
}
