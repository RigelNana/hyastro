use thiserror::Error;

/// Errors produced by astronomical event searches.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An astrometric evaluation failed.
    #[error(transparent)]
    Astrometry(#[from] crate::astro::Error),

    /// A mathematical value or root refinement was invalid.
    #[error(transparent)]
    Math(#[from] crate::math::Error),

    /// A time value or civil conversion was invalid.
    #[error(transparent)]
    Time(#[from] crate::time::Error),

    /// A duration-valued search option was zero, negative, or inconsistent.
    #[error(
        "{field} must be positive and no greater than {maximum_nanoseconds} ns, got {nanoseconds} ns"
    )]
    InvalidSearchDuration {
        /// Name of the invalid option.
        field: &'static str,
        /// Rejected exact duration.
        nanoseconds: i128,
        /// Inclusive maximum accepted duration.
        maximum_nanoseconds: i128,
    },

    /// The angular event tolerance was zero, negative, or too large.
    #[error("solar-term longitude tolerance must be in (0, {maximum_radians}] rad, got {radians}")]
    InvalidLongitudeTolerance {
        /// Rejected angular tolerance.
        radians: f64,
        /// Inclusive maximum accepted tolerance.
        maximum_radians: f64,
    },

    /// An iteration or evaluation budget was zero.
    #[error("{field} must be positive, got {value}")]
    InvalidSearchLimit {
        /// Name of the invalid budget.
        field: &'static str,
        /// Rejected budget.
        value: u32,
    },

    /// The explicit search evaluation budget was exhausted.
    #[error("solar-term search exhausted its {maximum} evaluation budget")]
    EvaluationLimitExceeded {
        /// Maximum permitted astrometric evaluations.
        maximum: u32,
    },

    /// Consecutive solar samples did not form the required increasing longitude sequence.
    #[error(
        "apparent solar longitude did not increase from {previous_radians} to {current_radians} rad between {previous_tai_nanoseconds} and {current_tai_nanoseconds} TAI ns"
    )]
    SolarLongitudeNotIncreasing {
        /// Earlier wrapped apparent longitude.
        previous_radians: f64,
        /// Later wrapped apparent longitude.
        current_radians: f64,
        /// Earlier sample epoch as TAI nanoseconds since 1900-01-01 TAI.
        previous_tai_nanoseconds: i128,
        /// Later sample epoch in the same representation.
        current_tai_nanoseconds: i128,
    },

    /// Time refinement converged without meeting the requested angular residual.
    #[error(
        "{term} residual {residual_radians} rad exceeds longitude tolerance {tolerance_radians} rad"
    )]
    SolarTermResidualExceeded {
        /// Stable English solar-term name.
        term: &'static str,
        /// Final absolute apparent-longitude residual.
        residual_radians: f64,
        /// Required maximum residual.
        tolerance_radians: f64,
    },

    /// A local Gregorian year did not contain exactly one occurrence of every solar term.
    #[error("fixed-offset Gregorian year {year} produced {found} solar terms instead of 24")]
    IncompleteSolarTermYear {
        /// Requested astronomical Gregorian year.
        year: i32,
        /// Number of terms found in that local year.
        found: usize,
    },

    /// A yearly result contained a solar term in an unexpected chronological position.
    #[error(
        "fixed-offset Gregorian year {year} expected {expected} at index {index}, found {actual}"
    )]
    UnexpectedSolarTermSequence {
        /// Requested Gregorian year.
        year: i32,
        /// Zero-based chronological index.
        index: usize,
        /// Expected English solar-term name.
        expected: &'static str,
        /// Actual English solar-term name.
        actual: &'static str,
    },
}
