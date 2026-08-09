use thiserror::Error;

/// Errors produced by catalog values and barycentric space-motion algorithms.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A mathematical value or operation was invalid.
    #[error(transparent)]
    Math(#[from] crate::math::Error),

    /// An uncertainty or correlation value was invalid.
    #[error(transparent)]
    Uncertainty(#[from] crate::uncertainty::Error),

    /// A physical parallax was zero or negative.
    #[error("physical parallax must be positive, got {arcseconds} arcsec")]
    InvalidPhysicalParallax {
        /// Rejected parallax in arcseconds.
        arcseconds: f64,
    },

    /// A non-zero correlation was attached to a parameter with zero uncertainty.
    #[error(
        "correlation between {parameter} and {other_parameter} is undefined because {parameter} has zero standard uncertainty, got {coefficient}"
    )]
    UndefinedCorrelationForZeroUncertainty {
        /// Zero-uncertainty parameter.
        parameter: &'static str,
        /// Correlated parameter.
        other_parameter: &'static str,
        /// Rejected coefficient.
        coefficient: f64,
    },

    /// A tangent-plane right-ascension coordinate was requested at a pole.
    #[error(
        "right-ascension tangent-plane covariance is undefined at declination {declination_radians} rad"
    )]
    UndefinedCatalogTangentPlane {
        /// Declination at which the tangent basis is singular.
        declination_radians: f64,
    },

    /// Numerical covariance propagation produced a negative or non-finite variance.
    #[error("propagated variance for {parameter} is invalid: {variance}")]
    InvalidPropagatedVariance {
        /// Catalog parameter whose variance was rejected.
        parameter: &'static str,
        /// Rejected canonical variance.
        variance: f64,
    },

    /// A non-zero `mu_alpha*` could not be converted to `d(alpha)/dt` at a pole.
    #[error(
        "right-ascension proper motion is undefined at declination {declination_radians} rad for mu_alpha*={right_ascension_cos_declination_radians_per_year} rad/year"
    )]
    UndefinedRightAscensionMotion {
        /// Catalog declination in radians.
        declination_radians: f64,
        /// Rejected `mu_alpha*` in radians per TCB Julian year.
        right_ascension_cos_declination_radians_per_year: f64,
    },

    /// A barycentric catalog state had a zero position vector.
    #[error("barycentric catalog position must be non-zero")]
    NullBarycentricPosition,

    /// A barycentric catalog state moved at or above light speed.
    #[error("barycentric catalog speed {metres_per_second} m/s must be below light speed")]
    SuperluminalSpaceMotion {
        /// Rejected speed in metres per second.
        metres_per_second: f64,
    },

    /// SOFA rejected a catalog space-motion conversion.
    #[cfg(feature = "std")]
    #[error("SOFA failed while {operation} with status {status}")]
    SpaceMotionConversionFailed {
        /// Conversion being evaluated.
        operation: &'static str,
        /// SOFA integer status.
        status: i32,
    },

    /// SOFA would have silently replaced invalid space-motion data.
    #[cfg(feature = "std")]
    #[error("SOFA reported a lossy fallback while {operation} with status {status}")]
    SpaceMotionFallbackRejected {
        /// Conversion being evaluated.
        operation: &'static str,
        /// SOFA warning bit set.
        status: i32,
    },
}
