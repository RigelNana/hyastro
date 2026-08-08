use crate::math::{Latitude, Length, Longitude};

use super::Error;

/// East-positive geodetic longitude in the interval `(-π, π]`.
///
/// The angle locates the normal section of a reference ellipsoid; it is not a
/// generic spherical or celestial longitude.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GeodeticLongitude(Longitude);

impl GeodeticLongitude {
    /// Constructs a geodetic longitude from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Longitude::try_from_radians(value)
            .map(Self)
            .map_err(Error::from)
    }

    /// Constructs a geodetic longitude from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Longitude::try_from_degrees(value)
            .map(Self)
            .map_err(Error::from)
    }

    /// Normalizes radians into `(-π, π]` and constructs a longitude.
    pub fn wrap_radians(value: f64) -> Result<Self, Error> {
        Longitude::wrap_radians(value)
            .map(Self)
            .map_err(Error::from)
    }

    /// Normalizes degrees into `(-180°, 180°]` and constructs a longitude.
    pub fn wrap_degrees(value: f64) -> Result<Self, Error> {
        Longitude::wrap_degrees(value)
            .map(Self)
            .map_err(Error::from)
    }

    /// Returns the east-positive longitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0.as_radians()
    }

    /// Returns the east-positive longitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.as_degrees()
    }

    /// Returns the underlying bounded longitude.
    pub const fn as_longitude(self) -> Longitude {
        self.0
    }
}

/// Geodetic latitude in the closed interval `[-π/2, π/2]`.
///
/// This is the angle between the reference-ellipsoid normal and the equatorial
/// plane, not the angle of the geocentric position vector.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GeodeticLatitude(Latitude);

impl GeodeticLatitude {
    /// Constructs a geodetic latitude from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Latitude::try_from_radians(value)
            .map(Self)
            .map_err(Error::from)
    }

    /// Constructs a geodetic latitude from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Latitude::try_from_degrees(value)
            .map(Self)
            .map_err(Error::from)
    }

    /// Returns the geodetic latitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0.as_radians()
    }

    /// Returns the geodetic latitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.as_degrees()
    }

    /// Returns the underlying bounded latitude.
    pub const fn as_latitude(self) -> Latitude {
        self.0
    }
}

/// Geocentric latitude in the closed interval `[-π/2, π/2]`.
///
/// This is the angle between an ITRS position vector and the equatorial plane;
/// it is undefined at the geocentric origin.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GeocentricLatitude(Latitude);

impl GeocentricLatitude {
    /// Constructs a geocentric latitude from radians.
    pub fn try_from_radians(value: f64) -> Result<Self, Error> {
        Latitude::try_from_radians(value)
            .map(Self)
            .map_err(Error::from)
    }

    /// Constructs a geocentric latitude from degrees.
    pub fn try_from_degrees(value: f64) -> Result<Self, Error> {
        Latitude::try_from_degrees(value)
            .map(Self)
            .map_err(Error::from)
    }

    /// Returns the geocentric latitude in radians.
    pub const fn as_radians(self) -> f64 {
        self.0.as_radians()
    }

    /// Returns the geocentric latitude in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.as_degrees()
    }

    /// Returns the underlying bounded latitude.
    pub const fn as_latitude(self) -> Latitude {
        self.0
    }
}

/// Signed height along the ellipsoid normal relative to its reference surface.
///
/// This is ellipsoidal height, not orthometric height above a geoid or mean sea
/// level. Negative values represent points below the ellipsoid surface.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EllipsoidalHeight(Length);

impl EllipsoidalHeight {
    /// Constructs an ellipsoidal height from metres.
    pub fn from_metres(value: f64) -> Result<Self, Error> {
        Length::from_metres(value).map(Self).map_err(Error::from)
    }

    /// Constructs an ellipsoidal height from a finite signed length.
    pub const fn from_length(value: Length) -> Self {
        Self(value)
    }

    /// Returns the ellipsoidal height as a length.
    pub const fn as_length(self) -> Length {
        self.0
    }

    /// Returns the ellipsoidal height in metres.
    pub const fn as_metres(self) -> f64 {
        self.0.as_metres()
    }
}

/// A strongly typed geodetic position relative to a reference ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodeticPosition {
    longitude: GeodeticLongitude,
    latitude: GeodeticLatitude,
    height: EllipsoidalHeight,
}

impl GeodeticPosition {
    /// Constructs a geodetic position from validated semantic coordinates.
    pub const fn new(
        longitude: GeodeticLongitude,
        latitude: GeodeticLatitude,
        height: EllipsoidalHeight,
    ) -> Self {
        Self {
            longitude,
            latitude,
            height,
        }
    }

    /// Returns the east-positive geodetic longitude.
    pub const fn longitude(self) -> GeodeticLongitude {
        self.longitude
    }

    /// Returns the geodetic latitude.
    pub const fn latitude(self) -> GeodeticLatitude {
        self.latitude
    }

    /// Returns the ellipsoidal height.
    pub const fn height(self) -> EllipsoidalHeight {
        self.height
    }
}
