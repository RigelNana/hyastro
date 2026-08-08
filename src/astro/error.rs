use thiserror::Error;

use crate::ephem::CelestialBody;

/// Errors produced by astrometric correction chains.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A mathematical value or geometry operation was invalid.
    #[error(transparent)]
    Math(#[from] crate::math::Error),

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
