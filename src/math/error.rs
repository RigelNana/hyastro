use thiserror::Error;

/// Errors produced by mathematical value construction and algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[non_exhaustive]
pub enum Error {
    /// A value that must be finite was NaN or infinite.
    #[error("{field} must be finite, got {value}")]
    NonFinite {
        /// Name of the invalid field.
        field: &'static str,
        /// Invalid value.
        value: f64,
    },
    /// A value was outside its semantic interval.
    #[error("{field} must be in {interval} {unit}, got {value}")]
    OutOfRange {
        /// Name of the invalid field.
        field: &'static str,
        /// Invalid value.
        value: f64,
        /// Human-readable interval.
        interval: &'static str,
        /// Unit used by the interval and value.
        unit: &'static str,
    },
    /// A sexagesimal string did not match a supported syntax.
    #[error("invalid {format} sexagesimal syntax")]
    InvalidSexagesimalSyntax {
        /// Expected sexagesimal representation.
        format: &'static str,
    },
    /// A vector required to have a direction had zero magnitude.
    #[error("a zero vector has no direction")]
    ZeroVector,
    /// Two directions were coincident where a unique arc was required.
    #[error("coincident directions do not define a unique arc")]
    CoincidentDirections,
    /// Two directions were antipodal where a unique arc was required.
    #[error("antipodal directions do not define a unique arc")]
    AntipodalDirections,
    /// Longitude was requested at a coordinate pole.
    #[error("longitude is undefined at a pole")]
    UndefinedLongitude,
    /// Position angle was requested for coincident directions.
    #[error("position angle is undefined for coincident directions")]
    UndefinedPositionAngle,
    /// A matrix was singular at the requested operation.
    #[error("matrix is singular; determinant is {determinant}")]
    SingularMatrix {
        /// Matrix determinant.
        determinant: f64,
    },
    /// A matrix failed the rotation invariants.
    #[error(
        "invalid rotation: orthogonality residual {orthogonality_residual}, determinant {determinant}, tolerances ({orthogonality_tolerance}, {determinant_tolerance})"
    )]
    InvalidRotation {
        /// Maximum absolute residual in RᵀR - I.
        orthogonality_residual: f64,
        /// Matrix determinant.
        determinant: f64,
        /// Allowed orthogonality residual.
        orthogonality_tolerance: f64,
        /// Allowed absolute determinant error from one.
        determinant_tolerance: f64,
    },
    /// A quaternion failed the unit-norm invariant.
    #[error("quaternion norm must be non-zero and finite, got {norm}")]
    InvalidQuaternion {
        /// Quaternion norm.
        norm: f64,
    },
    /// An algorithm tolerance was non-finite, subnormal, zero, or negative.
    #[error("{field} tolerance must be finite, normal, and positive, got {value}")]
    InvalidTolerance {
        /// Name of the invalid tolerance.
        field: &'static str,
        /// Invalid tolerance.
        value: f64,
    },
    /// A numerical interval had unordered or equal endpoints.
    #[error("interval lower bound {lower} must be less than upper bound {upper}")]
    InvalidInterval {
        /// Lower interval bound.
        lower: f64,
        /// Upper interval bound.
        upper: f64,
    },
    /// A root interval did not bracket a sign change.
    #[error("root is not bracketed on [{lower}, {upper}]: f(lower)={f_lower}, f(upper)={f_upper}")]
    NotBracketed {
        /// Lower interval bound.
        lower: f64,
        /// Upper interval bound.
        upper: f64,
        /// Function value at the lower bound.
        f_lower: f64,
        /// Function value at the upper bound.
        f_upper: f64,
    },
    /// A numerical method exhausted its iteration budget.
    #[error(
        "method did not converge after {iterations} iterations; residual {residual}, bracket [{lower}, {upper}]"
    )]
    NonConvergent {
        /// Number of completed iterations.
        iterations: u32,
        /// Final absolute residual.
        residual: f64,
        /// Final lower interval bound.
        lower: f64,
        /// Final upper interval bound.
        upper: f64,
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

    pub(crate) fn ensure_positive_tolerance(field: &'static str, value: f64) -> Result<f64, Self> {
        Self::ensure_finite(field, value)?;
        if value.is_normal() && value > 0.0 {
            Ok(value)
        } else {
            Err(Self::InvalidTolerance { field, value })
        }
    }
}
