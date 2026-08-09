use crate::{
    constants::body::{IAU_2015_NOMINAL_SOLAR_RADIUS_METRES, IAU_WGCCRE_2015_LUNAR_RADIUS_METRES},
    math::Length,
};

use super::{CelestialBody, Error};

/// An identified spherical surface model for one physical celestial body.
///
/// A sphere is an explicit approximation. Oblate and triaxial figures belong
/// to separate models rather than being reduced to an undocumented mean radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphericalBodyFigure {
    body: CelestialBody,
    identifier: &'static str,
    radius: Length,
}

impl SphericalBodyFigure {
    /// IAU 2015 Resolution B3 exact nominal solar-radius conversion constant.
    pub const IAU_2015_NOMINAL_SUN: Self = Self {
        body: CelestialBody::Sun,
        identifier: "IAU 2015 Resolution B3 nominal solar radius",
        radius: Length::from_finite(IAU_2015_NOMINAL_SOLAR_RADIUS_METRES),
    };

    /// IAU WGCCRE 2015 lunar reference sphere.
    pub const IAU_WGCCRE_2015_MOON: Self = Self {
        body: CelestialBody::Moon,
        identifier: "IAU WGCCRE 2015 lunar reference sphere",
        radius: Length::from_finite(IAU_WGCCRE_2015_LUNAR_RADIUS_METRES),
    };

    /// Constructs an identified positive-radius spherical figure.
    pub fn new(
        body: CelestialBody,
        identifier: &'static str,
        radius: Length,
    ) -> Result<Self, Error> {
        if !body.has_physical_surface() {
            return Err(Error::BodyHasNoPhysicalSurface { body });
        }
        if identifier.trim().is_empty() {
            return Err(Error::EmptyBodyFigureIdentifier);
        }
        if radius.as_metres() <= 0.0 {
            return Err(Error::InvalidSphericalBodyRadius {
                body,
                metres: radius.as_metres(),
            });
        }
        Ok(Self {
            body,
            identifier,
            radius,
        })
    }

    /// Returns the physical body represented by this figure.
    pub const fn body(self) -> CelestialBody {
        self.body
    }

    /// Returns the model and version identifier.
    pub const fn identifier(self) -> &'static str {
        self.identifier
    }

    /// Returns the sphere radius.
    pub const fn radius(self) -> Length {
        self.radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_figures_retain_body_radius_and_model() {
        let sun = SphericalBodyFigure::IAU_2015_NOMINAL_SUN;
        assert_eq!(sun.body(), CelestialBody::Sun);
        assert_eq!(sun.radius().as_metres(), 695_700_000.0);
        assert!(sun.identifier().contains("IAU 2015 Resolution B3"));

        let moon = SphericalBodyFigure::IAU_WGCCRE_2015_MOON;
        assert_eq!(moon.body(), CelestialBody::Moon);
        assert_eq!(moon.radius().as_metres(), 1_737_400.0);
        assert!(moon.identifier().contains("WGCCRE 2015"));
    }

    #[test]
    fn custom_figure_rejects_non_surface_identity_and_non_positive_radius() {
        assert!(matches!(
            SphericalBodyFigure::new(
                CelestialBody::EarthMoonBarycenter,
                "invalid barycentre sphere",
                Length::from_metres(1.0).unwrap(),
            ),
            Err(Error::BodyHasNoPhysicalSurface {
                body: CelestialBody::EarthMoonBarycenter
            })
        ));
        assert!(matches!(
            SphericalBodyFigure::new(
                CelestialBody::Earth,
                "invalid zero-radius sphere",
                Length::from_metres(0.0).unwrap(),
            ),
            Err(Error::InvalidSphericalBodyRadius { .. })
        ));
    }
}
