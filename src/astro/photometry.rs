use core::fmt;

use libm::{asin, log10};

use crate::{
    earth::ReferenceEllipsoid,
    ephem::SphericalBodyFigure,
    math::{ApparentMagnitude, FluxRatio, JohnsonV, MagnitudeDifference, Vega},
    time::TimeScale,
};

use super::{Error, LunarIllumination};

/// Applicability of the approximate lunar V-magnitude model at one epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LunarVApplicability {
    /// The phase is outside the model's documented near-full-Moon bias region.
    Nominal,
    /// The phase angle is below seven degrees, where the model tends to be about 0.12 mag too dim.
    NearFullMoonKnownBias,
    /// Some part of the lunar disk intersects the Earth's penumbral or umbral shadow.
    ///
    /// The returned value is the hypothetical uneclipsed-Moon magnitude and must not be treated as
    /// the Moon's actual brightness during the eclipse.
    EarthShadowIntersection,
}

/// The Horizons-compatible approximate, airless integrated-disk lunar V model.
///
/// Its phase law follows Krisciunas and Schaefer (1991), with the inverse-square distance scaling
/// and `+0.23` unit-distance zero-phase reference used by the Astronomical Almanac workflow. The
/// result uses Johnson V on the Vega magnitude system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HorizonsCompatibleLunarV;

impl HorizonsCompatibleLunarV {
    /// Stable model and provenance identifier.
    pub const IDENTIFIER: &'static str =
        "Horizons-compatible lunar V; Krisciunas-Schaefer 1991 phase law";

    const REFERENCE_MAGNITUDE: f64 = 0.23;
    const NEAR_FULL_MOON_LIMIT_DEGREES: f64 = 7.0;

    /// Evaluates the geocentric airless V magnitude from coherent lunar illumination geometry.
    ///
    /// `r` is the converged Sun-to-Moon distance at the lunar reflection event, `delta` is the
    /// converged Moon-to-Earth down-leg distance, and `alpha` is the physical phase angle at the
    /// Moon. Distances are converted to astronomical units and the phase angle to degrees before
    /// evaluating
    /// `0.23 + 5 log10(r delta) + 0.026 alpha + 4e-9 alpha^4`.
    ///
    /// Local atmospheric extinction, topocentric parallax, opposition-effect correction, lunar
    /// topography, and eclipse dimming are not applied. Eclipse geometry is reported through
    /// [`GeocentricLunarVMagnitude::applicability`].
    pub fn evaluate<S: TimeScale>(
        illumination: LunarIllumination<S>,
    ) -> Result<GeocentricLunarVMagnitude<S>, Error> {
        let sun_moon_distance = illumination
            .sunlight_at_moon()
            .distance()
            .as_astronomical_units();
        let moon_earth_distance = illumination
            .apparent_moon()
            .distance()
            .as_astronomical_units();
        Self::ensure_positive_distance("Sun-to-Moon distance", sun_moon_distance)?;
        Self::ensure_positive_distance("Moon-to-Earth distance", moon_earth_distance)?;

        let distance_correction = MagnitudeDifference::from_magnitudes(
            5.0 * log10(sun_moon_distance * moon_earth_distance),
        )?;
        let phase_degrees = illumination.phase_angle().as_degrees();
        let phase_squared = phase_degrees * phase_degrees;
        let phase_correction = MagnitudeDifference::from_magnitudes(
            0.026 * phase_degrees + 4.0e-9 * phase_squared * phase_squared,
        )?;
        let magnitude = ApparentMagnitude::from_magnitudes(
            Self::REFERENCE_MAGNITUDE
                + distance_correction.as_magnitudes()
                + phase_correction.as_magnitudes(),
        )?;
        let applicability = Self::applicability(illumination);

        Ok(GeocentricLunarVMagnitude {
            magnitude,
            illumination,
            distance_correction,
            phase_correction,
            applicability,
        })
    }

    fn ensure_positive_distance(field: &'static str, value: f64) -> Result<(), Error> {
        if value > 0.0 {
            Ok(())
        } else {
            Err(crate::math::Error::OutOfRange {
                field,
                value,
                interval: "(0, +infinity)",
                unit: "AU",
            }
            .into())
        }
    }

    fn applicability<S: TimeScale>(illumination: LunarIllumination<S>) -> LunarVApplicability {
        let moon_earth_distance_metres = illumination.apparent_moon().distance().as_metres();
        let sun_moon_distance_metres = illumination.sunlight_at_moon().distance().as_metres();
        let earth_radius = ReferenceEllipsoid::WGS84.semi_major_axis().as_metres();
        let sun_radius = SphericalBodyFigure::IAU_2015_NOMINAL_SUN
            .radius()
            .as_metres();
        let moon_radius = SphericalBodyFigure::IAU_WGCCRE_2015_MOON
            .radius()
            .as_metres();
        let shadow_intersection_limit = asin(earth_radius / moon_earth_distance_metres)
            + asin(sun_radius / sun_moon_distance_metres)
            + asin(moon_radius / moon_earth_distance_metres);

        if illumination.phase_angle().as_radians() <= shadow_intersection_limit {
            LunarVApplicability::EarthShadowIntersection
        } else if illumination.phase_angle().as_degrees() < Self::NEAR_FULL_MOON_LIMIT_DEGREES {
            LunarVApplicability::NearFullMoonKnownBias
        } else {
            LunarVApplicability::Nominal
        }
    }
}

/// One geocentric airless lunar V-magnitude estimate with retained geometry and model evidence.
pub struct GeocentricLunarVMagnitude<S: TimeScale> {
    magnitude: ApparentMagnitude<JohnsonV, Vega>,
    illumination: LunarIllumination<S>,
    distance_correction: MagnitudeDifference,
    phase_correction: MagnitudeDifference,
    applicability: LunarVApplicability,
}

impl<S: TimeScale> GeocentricLunarVMagnitude<S> {
    /// Returns the approximate integrated-disk Johnson V magnitude on the Vega system.
    pub const fn magnitude(self) -> ApparentMagnitude<JohnsonV, Vega> {
        self.magnitude
    }

    /// Returns the coherent light-time and phase geometry used by the model.
    pub const fn illumination(self) -> LunarIllumination<S> {
        self.illumination
    }

    /// Returns the `5 log10(r delta)` distance contribution in magnitudes.
    pub const fn distance_correction(self) -> MagnitudeDifference {
        self.distance_correction
    }

    /// Returns the empirical phase-law contribution in magnitudes.
    pub const fn phase_correction(self) -> MagnitudeDifference {
        self.phase_correction
    }

    /// Returns whether the model is nominal or in a documented limitation region.
    pub const fn applicability(self) -> LunarVApplicability {
        self.applicability
    }

    /// Returns the stable model and provenance identifier.
    pub const fn model_identifier(self) -> &'static str {
        HorizonsCompatibleLunarV::IDENTIFIER
    }

    /// Returns the modeled flux relative to a zero-magnitude Johnson V source.
    pub fn flux_ratio_to_zero_magnitude(self) -> Result<FluxRatio, crate::math::Error> {
        self.magnitude
            .flux_ratio_to(ApparentMagnitude::from_magnitudes(0.0)?)
    }
}

impl<S: TimeScale> Copy for GeocentricLunarVMagnitude<S> {}

impl<S: TimeScale> Clone for GeocentricLunarVMagnitude<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for GeocentricLunarVMagnitude<S> {
    fn eq(&self, other: &Self) -> bool {
        self.magnitude == other.magnitude
            && self.illumination == other.illumination
            && self.distance_correction == other.distance_correction
            && self.phase_correction == other.phase_correction
            && self.applicability == other.applicability
    }
}

impl<S: TimeScale> fmt::Debug for GeocentricLunarVMagnitude<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeocentricLunarVMagnitude")
            .field("magnitude", &self.magnitude)
            .field("illumination", &self.illumination)
            .field("distance_correction", &self.distance_correction)
            .field("phase_correction", &self.phase_correction)
            .field("applicability", &self.applicability)
            .field("model_identifier", &HorizonsCompatibleLunarV::IDENTIFIER)
            .finish()
    }
}
