use thiserror::Error;

/// Errors produced by astronomical event searches.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An astrometric evaluation failed.
    #[error(transparent)]
    Astrometry(#[from] crate::astro::Error),
    /// An Earth-reference-surface calculation failed.
    #[error(transparent)]
    Earth(#[from] crate::earth::Error),

    /// An ephemeris provider failed while identifying its provenance.
    #[error(transparent)]
    Ephemeris(#[from] crate::ephem::Error),

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

    /// An angular event tolerance was zero, negative, or too large.
    #[error("{field} must be in (0, {maximum_radians}] rad, got {radians}")]
    InvalidAngularTolerance {
        /// Name of the invalid angular tolerance.
        field: &'static str,
        /// Rejected tolerance in radians.
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

    /// The explicit astronomical-event evaluation budget was exhausted.
    #[error("astronomical-event search exhausted its {maximum} evaluation budget")]
    EvaluationLimitExceeded {
        /// Maximum permitted astrometric evaluations.
        maximum: u32,
    },

    /// A bounded extremum refinement exhausted its iteration budget.
    #[error("bounded extremum search did not converge within {iterations} iterations")]
    ExtremumSearchDidNotConverge {
        /// Maximum refinement iterations attempted.
        iterations: u32,
    },

    /// A relative event query named one body as both target and reference.
    #[error("relative event target and reference are both {body}")]
    IdenticalEventBodies {
        /// Rejected target and reference body.
        body: crate::ephem::CelestialBody,
    },

    /// A refined angular event did not meet its requested residual.
    #[error("{event} residual {residual_radians} rad exceeds tolerance {tolerance_radians} rad")]
    AngularResidualExceeded {
        /// Stable event description.
        event: &'static str,
        /// Final absolute angular residual.
        residual_radians: f64,
        /// Required maximum residual.
        tolerance_radians: f64,
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

    /// Consecutive samples did not form the required increasing lunar elongation sequence.
    #[error(
        "apparent lunar-minus-solar longitude did not increase from {previous_radians} to {current_radians} rad between {previous_tai_nanoseconds} and {current_tai_nanoseconds} TAI ns"
    )]
    MoonElongationNotIncreasing {
        /// Earlier wrapped apparent longitude difference.
        previous_radians: f64,
        /// Later wrapped apparent longitude difference.
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

    /// Consecutive geometric longitude samples did not advance on the required branch.
    #[error(
        "cycle angle did not increase from {previous_radians} to {current_radians} rad between {previous_tai_nanoseconds} and {current_tai_nanoseconds} TAI ns"
    )]
    CycleAngleNotIncreasing {
        /// Earlier wrapped angle.
        previous_radians: f64,
        /// Later wrapped angle.
        current_radians: f64,
        /// Earlier sample epoch as TAI nanoseconds since 1900-01-01 TAI.
        previous_tai_nanoseconds: i128,
        /// Later sample epoch in the same representation.
        current_tai_nanoseconds: i128,
    },

    /// Cycle statistics were requested without a complete measured cycle.
    #[error("cycle statistics require at least one complete measured cycle")]
    EmptyCycleSample,

    /// A numerical mean-cycle model was evaluated outside its recommended epoch range.
    #[error("{model} is recommended for Julian epochs [{start}, {end}], got J{epoch}")]
    ModelEpochOutsideValidity {
        /// Stable numerical-model identifier.
        model: &'static str,
        /// Rejected Julian epoch.
        epoch: f64,
        /// Inclusive first recommended Julian epoch.
        start: f64,
        /// Inclusive last recommended Julian epoch.
        end: f64,
    },

    /// A local solar-eclipse model assigned the wrong physical body to one limb.
    #[error("{role} eclipse figure represents {actual}, expected {expected}")]
    InvalidSolarEclipseFigure {
        /// Stable role name in the eclipse geometry.
        role: &'static str,
        /// Required physical body.
        expected: crate::ephem::CelestialBody,
        /// Body carried by the rejected figure.
        actual: crate::ephem::CelestialBody,
    },

    /// A local solar-eclipse contact remained inside the selected limb boundary for twelve hours.
    #[error(
        "{contact} could not be bracketed within twelve hours of maximum at {maximum_tai_nanoseconds} TAI ns"
    )]
    SolarEclipseContactNotBracketed {
        /// Stable contact description.
        contact: &'static str,
        /// Greatest-eclipse instant as TAI nanoseconds since 1900-01-01 TAI.
        maximum_tai_nanoseconds: i128,
    },

    /// A lunar-eclipse model assigned the wrong physical body to one spherical figure.
    #[error("{role} lunar-eclipse figure represents {actual}, expected {expected}")]
    InvalidLunarEclipseFigure {
        /// Stable role name in the eclipse geometry.
        role: &'static str,
        /// Required physical body.
        expected: crate::ephem::CelestialBody,
        /// Body carried by the rejected figure.
        actual: crate::ephem::CelestialBody,
    },

    /// A global lunar-eclipse contact remained inside its shadow boundary for eight hours.
    #[error(
        "{contact} could not be bracketed within eight hours of maximum at {maximum_tai_nanoseconds} TAI ns"
    )]
    LunarEclipseContactNotBracketed {
        /// Stable contact description.
        contact: &'static str,
        /// Greatest-eclipse instant as TAI nanoseconds since 1900-01-01 TAI.
        maximum_tai_nanoseconds: i128,
    },

    /// Fixed-site visibility was requested with a different ephemeris than the global eclipse.
    #[error("lunar-eclipse visibility ephemeris does not match the global eclipse")]
    LunarEclipseVisibilityEphemerisMismatch,

    /// Fixed-site visibility combined a site and global eclipse using different Earth ellipsoids.
    #[error("lunar-eclipse visibility site Earth does not match the global eclipse")]
    LunarEclipseVisibilityEarthMismatch,

    /// A central shadow-axis limit remained on the ellipsoid for twelve hours from maximum.
    #[error(
        "global solar-eclipse central-path {limit} could not be bracketed within twelve hours of maximum at {maximum_tai_nanoseconds} TAI ns"
    )]
    GlobalSolarEclipsePathLimitNotBracketed {
        /// Stable identity of the missing path limit.
        limit: &'static str,
        /// Greatest-eclipse instant as TAI nanoseconds since 1900-01-01 TAI.
        maximum_tai_nanoseconds: i128,
    },

    /// A central-path calculation requested a surface radius where the axis missed Earth.
    #[error(
        "lunar shadow axis does not intersect the selected Earth ellipsoid at {epoch_tai_nanoseconds} TAI ns"
    )]
    ShadowAxisDoesNotIntersectEarth {
        /// Failed epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
    },
    /// A Besselian limb model omitted its stable provenance identifier.
    #[error("Besselian limb model identifier must not be empty")]
    EmptyBesselianLimbModelIdentifier,

    /// A Besselian limb model omitted its source citation.
    #[error("Besselian limb model source must not be empty")]
    EmptyBesselianLimbModelSource,

    /// A Besselian limb-model radius was non-positive or internally inconsistent.
    #[error("invalid Besselian limb-model {field}: {value}")]
    InvalidBesselianLimbModelValue {
        /// Rejected model field.
        field: &'static str,
        /// Rejected value in the field's documented units.
        value: f64,
    },

    /// A Besselian polynomial was evaluated outside its closed validity interval.
    #[error(
        "Besselian polynomial epoch {epoch_tai_nanoseconds} lies outside [{start_tai_nanoseconds}, {end_tai_nanoseconds}] TAI ns"
    )]
    BesselianPolynomialOutsideValidity {
        /// Rejected epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
        /// Inclusive validity start in the same representation.
        start_tai_nanoseconds: i128,
        /// Inclusive validity end in the same representation.
        end_tai_nanoseconds: i128,
    },
    /// Geographic circumstances were requested for a non-central solar eclipse.
    #[error("solar eclipse has no central path")]
    SolarEclipseHasNoCentralPath,

    /// A geographic path combined different reference ellipsoids.
    #[error("solar-eclipse path Earth does not match the Besselian polynomial Earth")]
    BesselianPathEarthMismatch,

    /// A geographic path combined different ephemeris provenance.
    #[error("solar-eclipse path ephemeris does not match the Events ephemeris")]
    BesselianPathEphemerisMismatch,

    /// Delta T was resolved at an epoch other than the polynomial reference epoch.
    #[error(
        "solar-eclipse path Delta T epoch {actual_tai_nanoseconds} does not match polynomial reference epoch {expected_tai_nanoseconds} TAI ns"
    )]
    BesselianPathDeltaTEpochMismatch {
        /// Polynomial reference epoch as TAI nanoseconds since 1900-01-01 TAI.
        expected_tai_nanoseconds: i128,
        /// Delta T epoch in the same representation.
        actual_tai_nanoseconds: i128,
    },

    /// A six-hour polynomial did not cover the complete central-path interval.
    #[error(
        "Besselian polynomial validity [{validity_start_tai_nanoseconds}, {validity_end_tai_nanoseconds}] does not cover central path [{path_start_tai_nanoseconds}, {path_end_tai_nanoseconds}] TAI ns"
    )]
    BesselianPathValidityTooShort {
        /// Polynomial validity start as TAI nanoseconds since 1900-01-01 TAI.
        validity_start_tai_nanoseconds: i128,
        /// Polynomial validity end in the same representation.
        validity_end_tai_nanoseconds: i128,
        /// Central-path start in the same representation.
        path_start_tai_nanoseconds: i128,
        /// Central-path end in the same representation.
        path_end_tai_nanoseconds: i128,
    },

    /// The northern and southern path-envelope roots could not both be isolated.
    #[error(
        "found {found} solar-eclipse path-limit roots at {epoch_tai_nanoseconds} TAI ns; expected at least two"
    )]
    BesselianPathLimitsUnavailable {
        /// Failed epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
        /// Number of distinct roots found around the shadow cone.
        found: usize,
    },

    /// Second or third contact could not be bracketed inside polynomial validity.
    #[error(
        "could not bracket {contact} central contact at fixed site around {epoch_tai_nanoseconds} TAI ns"
    )]
    BesselianCentralContactNotBracketed {
        /// `"C2"` or `"C3"`.
        contact: &'static str,
        /// Centre-line epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
    },
    /// Ellipsoidal inverse-geodesic iteration failed for one path cross-section.
    #[error(
        "solar-eclipse path-width inverse geodesic did not converge at {epoch_tai_nanoseconds} TAI ns"
    )]
    BesselianPathWidthDidNotConverge {
        /// Failed epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
    },
    /// A geographic path sampling cadence was outside the supported range.
    #[error(
        "solar-eclipse path sample step must be in [{minimum_nanoseconds}, {maximum_nanoseconds}] ns, got {nanoseconds} ns"
    )]
    InvalidBesselianPathSampleStep {
        /// Rejected exact duration.
        nanoseconds: i128,
        /// Inclusive minimum accepted duration.
        minimum_nanoseconds: i128,
        /// Inclusive maximum accepted duration.
        maximum_nanoseconds: i128,
    },
}
