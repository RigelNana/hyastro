use thiserror::Error;

/// Errors produced by time value construction, conversion, and adapters.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A numeric component was outside its valid interval.
    #[error("{component} must be in [{minimum}, {maximum}], got {value}")]
    ComponentOutOfRange {
        /// Name of the invalid component.
        component: &'static str,
        /// Invalid component value.
        value: i128,
        /// Inclusive lower bound.
        minimum: i128,
        /// Inclusive upper bound.
        maximum: i128,
    },
    /// A calendar date did not exist.
    #[error("{year}-{month:02}-{day:02} is not a valid {calendar} date")]
    InvalidDate {
        /// Astronomically numbered year.
        year: i32,
        /// One-based month.
        month: u8,
        /// One-based day.
        day: u8,
        /// Calendar name.
        calendar: &'static str,
    },
    /// A value required to be finite was NaN or infinite.
    #[error("{field} must be finite, got {value}")]
    NonFinite {
        /// Name of the invalid value.
        field: &'static str,
        /// Invalid floating-point value.
        value: f64,
    },
    /// Checked arithmetic exceeded the representation range.
    #[error("time arithmetic overflow while {operation}")]
    Overflow {
        /// Operation that overflowed.
        operation: &'static str,
    },
    /// Leap-second data had invalid metadata or ordering.
    #[error("invalid leap-second data: {reason}")]
    InvalidLeapSeconds {
        /// Violated data invariant.
        reason: &'static str,
    },
    /// A leap-second entry violated the table invariants.
    #[error("invalid leap-second entry {index}: {reason}")]
    InvalidLeapSecond {
        /// Zero-based entry index.
        index: usize,
        /// Violated entry invariant.
        reason: &'static str,
    },
    /// Leap-second data started after a requested label or instant.
    #[error("leap-second data is unavailable before {year}-{month:02}-{day:02}")]
    LeapSecondsUnavailable {
        /// First covered year.
        year: i32,
        /// First covered month.
        month: u8,
        /// First covered day.
        day: u8,
    },
    /// Leap-second data had expired before a requested label or instant.
    #[error("leap-second data expired on {year}-{month:02}-{day:02}")]
    LeapSecondsExpired {
        /// Expiration year.
        year: i32,
        /// Expiration month.
        month: u8,
        /// Expiration day.
        day: u8,
    },
    /// A UTC leap-second label was used without a leap-second-aware backend.
    #[error("UTC leap-second label requires a leap-second-aware time context")]
    LeapSecondRequiresContext,
    /// A date was labeled as a UTC leap second without an offset transition.
    #[error("{year}-{month:02}-{day:02} does not end with a positive UTC leap second")]
    InvalidLeapSecondDate {
        /// Astronomically numbered year.
        year: i32,
        /// One-based month.
        month: u8,
        /// One-based day.
        day: u8,
    },
    /// A UTC label was removed by a negative leap second.
    #[error("{year}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02} does not exist in UTC")]
    NonexistentUtcLabel {
        /// Astronomically numbered year.
        year: i32,
        /// One-based month.
        month: u8,
        /// One-based day.
        day: u8,
        /// Hour.
        hour: u8,
        /// Minute.
        minute: u8,
        /// Second.
        second: u8,
    },
    /// An adapter cannot represent a UTC leap-second label.
    #[error("{target} cannot represent UTC leap-second labels")]
    LeapSecondNotRepresentable {
        /// Target library or representation.
        target: &'static str,
    },
    /// A non-uniform scale was used by a context-free conversion.
    #[error("{scale} requires an explicit time context for {operation}")]
    ContextRequired {
        /// Time scale requiring contextual data.
        scale: &'static str,
        /// Requested operation.
        operation: &'static str,
    },
    /// A backend does not implement an operation for a time scale.
    #[error("{backend} does not support {operation} for {scale}")]
    UnsupportedScale {
        /// Backend name.
        backend: &'static str,
        /// Requested operation.
        operation: &'static str,
        /// Time scale name.
        scale: &'static str,
    },
    /// Earth-orientation data had invalid metadata or coverage.
    #[error("invalid Earth-orientation data: {reason}")]
    InvalidEarthOrientationData {
        /// Violated data invariant.
        reason: &'static str,
    },
    /// An Earth-orientation sample violated the table invariants.
    #[error("invalid Earth-orientation sample {index}: {reason}")]
    InvalidEarthOrientationSample {
        /// Zero-based sample index.
        index: usize,
        /// Violated sample invariant.
        reason: &'static str,
    },
    /// An IERS Earth-orientation text record could not be parsed.
    #[cfg(feature = "std")]
    #[error("invalid Earth-orientation {field} on source line {line}")]
    InvalidEarthOrientationText {
        /// One-based source line.
        line: usize,
        /// Field or structural element that failed validation.
        field: &'static str,
    },
    /// A record's calendar label and supplied Modified Julian Date disagreed.
    #[cfg(feature = "std")]
    #[error(
        "Earth-orientation source line {line} has MJD {actual}, expected {expected} from its UTC calendar label"
    )]
    EarthOrientationMjdMismatch {
        /// One-based source line.
        line: usize,
        /// MJD derived from the parsed UTC calendar label.
        expected: f64,
        /// MJD carried by the source record.
        actual: f64,
    },
    /// A normalized record lacked a value required by a complete EOP sample.
    #[cfg(feature = "std")]
    #[error("Earth-orientation record at {epoch_tai_nanoseconds} TAI ns lacks required {field}")]
    MissingEarthOrientationValue {
        /// Missing semantic field.
        field: &'static str,
        /// Record epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
    },
    /// A parsed EOP value is less authoritative than the caller permits.
    #[cfg(feature = "std")]
    #[error(
        "Earth-orientation source line {line} has {provenance} {field}, rejected by {acceptance}"
    )]
    EarthOrientationValueRejected {
        /// One-based source line.
        line: usize,
        /// Semantic value group that failed the policy.
        field: &'static str,
        /// Parsed provenance class.
        provenance: &'static str,
        /// Active acceptance policy.
        acceptance: &'static str,
    },
    /// Earth-orientation samples do not cover a requested physical instant.
    #[error(
        "Earth-orientation data covers [{coverage_start}, {coverage_end}] TAI ns, not requested instant {requested} TAI ns"
    )]
    EarthOrientationUnavailable {
        /// Requested instant as TAI nanoseconds since 1900-01-01 TAI.
        requested: i128,
        /// First covered instant in the same representation.
        coverage_start: i128,
        /// Last covered instant in the same representation.
        coverage_end: i128,
    },
    /// Earth-orientation data expired before a requested physical instant.
    #[error(
        "Earth-orientation data expired at {expires} TAI ns before requested instant {requested} TAI ns"
    )]
    EarthOrientationExpired {
        /// Requested instant as TAI nanoseconds since 1900-01-01 TAI.
        requested: i128,
        /// Exclusive expiration instant in the same representation.
        expires: i128,
    },
    /// Jiff rejected an adapter conversion.
    #[cfg(feature = "jiff")]
    #[error("jiff failed while {operation}: {reason}")]
    Jiff {
        /// Adapter operation.
        operation: &'static str,
        /// Upstream error.
        #[cfg_attr(feature = "std", source)]
        reason: jiff::Error,
    },
    /// Hifitime rejected an adapter conversion.
    #[cfg(feature = "hifitime")]
    #[error("hifitime failed while {operation}: {reason}")]
    Hifitime {
        /// Adapter operation.
        operation: &'static str,
        /// Upstream error.
        #[cfg_attr(feature = "std", source)]
        reason: hifitime::errors::HifitimeError,
    },
}

impl Error {
    pub(crate) fn ensure_finite(field: &'static str, value: f64) -> Result<f64, Self> {
        if value.is_finite() {
            Ok(value)
        } else {
            Err(Self::NonFinite { field, value })
        }
    }

    pub(crate) fn component(
        component: &'static str,
        value: i128,
        minimum: i128,
        maximum: i128,
    ) -> Self {
        Self::ComponentOutOfRange {
            component,
            value,
            minimum,
            maximum,
        }
    }
}
