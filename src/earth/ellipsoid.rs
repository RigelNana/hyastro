use crate::math::Length;

use super::Error;

/// An identified rotational reference ellipsoid.
///
/// The model is defined by its semi-major axis `a` and flattening `f`.
/// It is a conventional reference surface, not topography or a geoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReferenceEllipsoid {
    identifier: &'static str,
    semi_major_axis: Length,
    flattening: f64,
}

impl ReferenceEllipsoid {
    /// World Geodetic System 1984 reference ellipsoid.
    pub const WGS84: Self = Self {
        identifier: "WGS 84",
        semi_major_axis: Length::from_finite(6_378_137.0),
        flattening: 1.0 / 298.257_223_563,
    };

    /// Geodetic Reference System 1980 reference ellipsoid.
    pub const GRS80: Self = Self {
        identifier: "GRS 80",
        semi_major_axis: Length::from_finite(6_378_137.0),
        flattening: 1.0 / 298.257_222_101,
    };

    /// Constructs an identified rotational reference ellipsoid.
    pub fn new(
        identifier: &'static str,
        semi_major_axis: Length,
        flattening: f64,
    ) -> Result<Self, Error> {
        if identifier.trim().is_empty() {
            return Err(Error::EmptyEllipsoidIdentifier);
        }
        if semi_major_axis.as_metres() <= 0.0 {
            return Err(Error::InvalidEllipsoid {
                field: "semi-major axis",
                value: semi_major_axis.as_metres(),
                requirement: "greater than zero metres",
            });
        }
        if !flattening.is_finite() || !(0.0..1.0).contains(&flattening) {
            return Err(Error::InvalidEllipsoid {
                field: "flattening",
                value: flattening,
                requirement: "finite and in [0, 1)",
            });
        }
        Ok(Self {
            identifier,
            semi_major_axis,
            flattening,
        })
    }

    /// Returns the model identifier.
    pub const fn identifier(self) -> &'static str {
        self.identifier
    }

    /// Returns the equatorial semi-major axis `a`.
    pub const fn semi_major_axis(self) -> Length {
        self.semi_major_axis
    }

    /// Returns the flattening `f = (a - b) / a`.
    pub const fn flattening(self) -> f64 {
        self.flattening
    }

    /// Returns the inverse flattening `1 / f`, or infinity for a sphere.
    pub fn inverse_flattening(self) -> f64 {
        1.0 / self.flattening
    }

    /// Returns the polar semi-minor axis `b = a(1 - f)`.
    pub fn semi_minor_axis(self) -> Length {
        Length::from_finite(self.semi_major_axis.as_metres() * (1.0 - self.flattening))
    }

    /// Returns the first eccentricity squared, `e² = f(2 - f)`.
    pub fn first_eccentricity_squared(self) -> f64 {
        self.flattening * (2.0 - self.flattening)
    }

    /// Returns the second eccentricity squared, `e'² = e² / (1 - e²)`.
    pub fn second_eccentricity_squared(self) -> f64 {
        let first = self.first_eccentricity_squared();
        first / (1.0 - first)
    }
}
