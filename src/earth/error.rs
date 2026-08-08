use thiserror::Error;

/// Errors produced by Earth-shape, geodetic-position, and fixed-site algorithms.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A mathematical value or operation was invalid.
    #[error(transparent)]
    Math(#[from] crate::math::Error),

    /// A coordinate-frame transformation failed.
    #[error(transparent)]
    Frame(#[from] crate::frame::Error),

    /// A reference-ellipsoid parameter violated its invariant.
    #[error("invalid reference ellipsoid {field}: {value}; expected {requirement}")]
    InvalidEllipsoid {
        /// Invalid parameter name.
        field: &'static str,
        /// Invalid numeric value.
        value: f64,
        /// Required invariant.
        requirement: &'static str,
    },

    /// A reference ellipsoid had no model identifier.
    #[error("reference ellipsoid identifier must not be empty")]
    EmptyEllipsoidIdentifier,

    /// A fixed site had no identifier.
    #[error("fixed-site identifier must not be empty")]
    EmptySiteIdentifier,

    /// The geocentric origin has no unique longitude or latitude.
    #[error("geodetic coordinates are undefined at the geocentric origin")]
    UndefinedGeodeticPosition,

    /// SOFA rejected a geodetic-coordinate transformation.
    #[error("SOFA failed while {operation} with status {status}")]
    GeodeticConversionFailed {
        /// Transformation being evaluated.
        operation: &'static str,
        /// SOFA integer status.
        status: i32,
    },
}
