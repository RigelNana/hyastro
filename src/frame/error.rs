use thiserror::Error;

/// Errors produced by coordinate-frame state operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A numerical geometry operation failed while applying a frame transform.
    #[error(transparent)]
    Math(#[from] crate::math::Error),
    /// A time-scale or Earth-orientation model could not produce the transform epoch.
    #[error(transparent)]
    Time(#[from] crate::time::Error),

    /// A transform was applied or composed at a different physical epoch.
    #[error(
        "frame transform epoch {transform_tai_nanoseconds} TAI ns does not match state or following transform epoch {value_tai_nanoseconds} TAI ns"
    )]
    EpochMismatch {
        /// Transform epoch as TAI nanoseconds since 1900-01-01 TAI.
        transform_tai_nanoseconds: i128,
        /// State or following-transform epoch in the same representation.
        value_tai_nanoseconds: i128,
    },
    /// EOP coverage provided a value but not the derivative required for a state transform.
    #[error("Earth-orientation rate {field} is unavailable at {epoch_tai_nanoseconds} TAI ns")]
    EarthOrientationRateUnavailable {
        /// Missing derivative.
        field: &'static str,
        /// Transform epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
    },
    /// Numerical rotation differentiation did not meet its explicit error bound.
    #[error(
        "rotation-rate differentiation residual {residual} rad/s exceeds tolerance {tolerance} rad/s"
    )]
    RotationRateDidNotConverge {
        /// Richardson error estimate in radians per second.
        residual: f64,
        /// Required maximum error in radians per second.
        tolerance: f64,
    },
}

impl Error {
    pub(crate) const fn epoch_mismatch(
        transform_tai_nanoseconds: i128,
        value_tai_nanoseconds: i128,
    ) -> Self {
        Self::EpochMismatch {
            transform_tai_nanoseconds,
            value_tai_nanoseconds,
        }
    }
}
