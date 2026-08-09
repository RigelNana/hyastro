use thiserror::Error;

use crate::ephem::CelestialBody;

/// Errors produced by astrometric correction chains.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A mathematical value or geometry operation was invalid.
    #[error(transparent)]
    Math(#[from] crate::math::Error),

    /// A catalog value or space-motion operation was invalid.
    #[error(transparent)]
    Catalog(#[from] crate::catalog::Error),

    /// A time model could not represent a required epoch.
    #[error(transparent)]
    Time(#[from] crate::time::Error),

    /// A coordinate-frame operation failed.
    #[error(transparent)]
    Frame(#[from] crate::frame::Error),

    /// An ephemeris query failed.
    #[error(transparent)]
    Ephemeris(#[from] crate::ephem::Error),

    /// A fixed-site or topocentric-frame operation failed.
    #[error(transparent)]
    Earth(#[from] crate::earth::Error),

    /// An atmospheric model input was NaN or infinite.
    #[error("{field} must be finite, got {value}")]
    NonFiniteAtmosphericValue {
        /// Name of the rejected atmospheric field.
        field: &'static str,
        /// Rejected value.
        value: f64,
    },

    /// An atmospheric model input was outside its explicit accepted interval.
    #[error("{field} must be in [{minimum}, {maximum}], got {value}")]
    AtmosphericValueOutOfRange {
        /// Name of the rejected atmospheric field.
        field: &'static str,
        /// Rejected value.
        value: f64,
        /// Inclusive lower bound.
        minimum: f64,
        /// Inclusive upper bound.
        maximum: f64,
    },

    /// A body figure was applied to a different observed target.
    #[error("body figure for {figure_body} cannot describe observed target {target}")]
    BodyFigureTargetMismatch {
        /// Body at the centre of the observed place.
        target: CelestialBody,
        /// Body represented by the supplied figure.
        figure_body: CelestialBody,
    },

    /// The observer was not outside the supplied spherical body figure.
    #[error(
        "observer distance {distance_metres} m from {body} must exceed spherical radius {radius_metres} m"
    )]
    ObserverNotOutsideBodyFigure {
        /// Body whose apparent disk was requested.
        body: CelestialBody,
        /// Figure radius in metres.
        radius_metres: f64,
        /// Target-centre distance in metres.
        distance_metres: f64,
    },

    /// Two apparent disks were evaluated at different reception epochs.
    #[error(
        "apparent-disk reception epochs differ: {left_tai_nanoseconds} and {right_tai_nanoseconds} TAI ns"
    )]
    ApparentDiskEpochMismatch {
        /// Left disk epoch.
        left_tai_nanoseconds: i128,
        /// Right disk epoch.
        right_tai_nanoseconds: i128,
    },

    /// Two apparent disks were evaluated for different topocentric observers.
    #[error("apparent-disk separation requires one shared topocentric observer")]
    ApparentDiskObserverMismatch,

    /// A light-time tolerance was zero or negative.
    #[error("light-time tolerance must be positive, got {nanoseconds} ns")]
    InvalidLightTimeTolerance {
        /// Rejected exact tolerance in nanoseconds.
        nanoseconds: i128,
    },

    /// A light-time iteration budget was zero.
    #[error("light-time maximum iterations must be positive, got {max_iterations}")]
    InvalidLightTimeIterationLimit {
        /// Rejected iteration budget.
        max_iterations: u32,
    },

    /// A body was requested as both observed target and observer.
    #[error("light-time direction is undefined when target and observer are both {body}")]
    UndefinedIdentityObservation {
        /// Body used for both roles.
        body: CelestialBody,
    },

    /// Distinct target and observer states produced a zero line of sight.
    #[error("line of sight from {observer} to {target} is zero at {epoch_tai_nanoseconds} TAI ns")]
    UndefinedLineOfSight {
        /// Observed target.
        target: CelestialBody,
        /// Receiving observer.
        observer: CelestialBody,
        /// Evaluation epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
    },

    /// The observer velocity was not physically valid for aberration.
    #[error(
        "barycentric speed of {observer} is {speed_metres_per_second} m/s, at or above light speed"
    )]
    ObserverAtOrAboveLightSpeed {
        /// Receiving observer.
        observer: CelestialBody,
        /// Rejected barycentric speed.
        speed_metres_per_second: f64,
    },

    /// A fixed observer's barycentric velocity was not physically valid.
    #[error(
        "fixed observer barycentric speed is {speed_metres_per_second} m/s, at or above light speed"
    )]
    FixedObserverAtOrAboveLightSpeed {
        /// Rejected barycentric speed.
        speed_metres_per_second: f64,
    },

    /// An astrometric catalog place and fixed observer were evaluated at different epochs.
    #[error(
        "catalog-place epoch {catalog_tai_nanoseconds} differs from fixed-observer epoch {observer_tai_nanoseconds} TAI ns"
    )]
    CatalogPlaceEpochMismatch {
        /// Astrometric catalog-place epoch as TAI nanoseconds since 1900-01-01 TAI.
        catalog_tai_nanoseconds: i128,
        /// Fixed-observer epoch as TAI nanoseconds since 1900-01-01 TAI.
        observer_tai_nanoseconds: i128,
    },

    /// Reception light-time iteration exhausted its explicit budget.
    #[error(
        "reception light time from {target} to {observer} did not converge after {iterations} iterations: residual {residual_nanoseconds} ns at emission epoch {emission_tai_nanoseconds} TAI ns"
    )]
    LightTimeDidNotConverge {
        /// Observed target.
        target: CelestialBody,
        /// Receiving observer.
        observer: CelestialBody,
        /// Number of completed iterations.
        iterations: u32,
        /// Final absolute fixed-point residual.
        residual_nanoseconds: i128,
        /// Final attempted emission epoch as TAI nanoseconds since 1900-01-01 TAI.
        emission_tai_nanoseconds: i128,
    },

    /// A finite target and fixed site produced a zero line of sight.
    #[error(
        "line of sight from the fixed site to {target} is zero at {epoch_tai_nanoseconds} TAI ns"
    )]
    UndefinedFixedSiteLineOfSight {
        /// Observed target.
        target: CelestialBody,
        /// Evaluation epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
    },

    /// Fixed-site reception light-time iteration exhausted its budget.
    #[error(
        "reception light time from {target} to the fixed site did not converge after {iterations} iterations: residual {residual_nanoseconds} ns at emission epoch {emission_tai_nanoseconds} TAI ns"
    )]
    FixedSiteLightTimeDidNotConverge {
        /// Observed target.
        target: CelestialBody,
        /// Number of completed iterations.
        iterations: u32,
        /// Final absolute fixed-point residual.
        residual_nanoseconds: i128,
        /// Final attempted emission epoch as TAI nanoseconds since 1900-01-01 TAI.
        emission_tai_nanoseconds: i128,
    },
}

impl Error {
    pub(crate) fn ensure_atmospheric_range(
        field: &'static str,
        value: f64,
        minimum: f64,
        maximum: f64,
    ) -> Result<f64, Self> {
        if !value.is_finite() {
            return Err(Self::NonFiniteAtmosphericValue { field, value });
        }
        if !(minimum..=maximum).contains(&value) {
            return Err(Self::AtmosphericValueOutOfRange {
                field,
                value,
                minimum,
                maximum,
            });
        }
        Ok(value)
    }
}
