//! Strongly typed uncertainties and correlation structures.

use core::fmt::Debug;

use thiserror::Error;

use crate::{
    math::{Acceleration, Angle, AngularSpeed, Dimensionless, Length, Speed},
    time::Duration,
};

mod sealed {
    pub trait Sealed {}
}

/// A quantity that can carry a non-negative standard uncertainty.
///
/// The trait is sealed so every supported quantity has one crate-defined
/// canonical unit. Callers normally use it only through
/// [`StandardUncertainty`].
pub trait UncertaintyQuantity: sealed::Sealed + Copy + Debug + PartialEq {
    /// Human-readable quantity name used by construction errors.
    #[doc(hidden)]
    const QUANTITY_NAME: &'static str;

    /// Canonical unit used by construction errors.
    #[doc(hidden)]
    const CANONICAL_UNIT: &'static str;

    /// Returns the signed value in the quantity's canonical unit.
    #[doc(hidden)]
    fn canonical_value(self) -> f64;
}

/// Errors produced while constructing uncertainty values.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[non_exhaustive]
pub enum Error {
    /// A standard uncertainty was negative.
    #[error("{quantity} standard uncertainty must be non-negative, got {value} {unit}")]
    NegativeStandardUncertainty {
        /// Quantity whose uncertainty was rejected.
        quantity: &'static str,
        /// Rejected value in the canonical unit.
        value: f64,
        /// Canonical unit of `value`.
        unit: &'static str,
    },

    /// A correlation coefficient was not finite or outside `[-1, 1]`.
    #[error(
        "correlation coefficient [{row}, {column}] must be finite and within [-1, 1], got {value}"
    )]
    InvalidCorrelationCoefficient {
        /// Matrix row.
        row: usize,
        /// Matrix column.
        column: usize,
        /// Rejected coefficient.
        value: f64,
    },

    /// A correlation-matrix diagonal was not one.
    #[error("correlation diagonal [{index}, {index}] must be one, got {value}")]
    InvalidCorrelationDiagonal {
        /// Diagonal index.
        index: usize,
        /// Rejected diagonal value.
        value: f64,
    },

    /// Mirrored correlation coefficients disagreed.
    #[error(
        "correlation coefficients [{row}, {column}]={forward} and [{column}, {row}]={reverse} are not symmetric"
    )]
    AsymmetricCorrelation {
        /// First matrix index.
        row: usize,
        /// Second matrix index.
        column: usize,
        /// Coefficient in row-column order.
        forward: f64,
        /// Coefficient in column-row order.
        reverse: f64,
    },

    /// A correlation matrix was not positive semidefinite.
    #[error(
        "correlation matrix is not positive semidefinite at pivot {pivot}, residual {residual}"
    )]
    CorrelationMatrixNotPositiveSemidefinite {
        /// Failing factorization pivot.
        pivot: usize,
        /// Negative or inconsistent factorization residual.
        residual: f64,
    },
}
/// Provenance of a standard uncertainty attached to a resolved result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UncertaintyOrigin {
    /// The upstream data source reported the uncertainty for this exact sample.
    SourceReported,
    /// Adjacent source standard uncertainties were propagated through a linear
    /// interpolation without assuming their correlation.
    ///
    /// The resulting upper bound is
    /// `(1 − f) σ_left + f σ_right`. It does not include interpolation-model
    /// discrepancy.
    CorrelationAgnosticLinearInterpolation,
}

/// A finite, non-negative one-standard-deviation uncertainty.
///
/// `Q` preserves the physical dimension and canonical unit. This type does not
/// imply that the represented error is independent, Gaussian, or complete;
/// covariance, systematic effects, and model discrepancy remain separate
/// evidence owned by the result that exposes this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct StandardUncertainty<Q: UncertaintyQuantity>(Q);

impl<Q: UncertaintyQuantity> StandardUncertainty<Q> {
    /// Constructs a standard uncertainty from a quantity value.
    pub fn new(value: Q) -> Result<Self, Error> {
        let canonical = value.canonical_value();
        if canonical >= 0.0 {
            Ok(Self(value))
        } else {
            Err(Error::NegativeStandardUncertainty {
                quantity: Q::QUANTITY_NAME,
                value: canonical,
                unit: Q::CANONICAL_UNIT,
            })
        }
    }
    /// Constructs an uncertainty after a crate-internal non-negative proof.
    pub(crate) const fn from_validated(value: Q) -> Self {
        Self(value)
    }

    /// Returns the non-negative uncertainty in its original strong type.
    pub const fn value(self) -> Q {
        self.0
    }

    /// Returns whether this uncertainty is exactly zero.
    pub fn is_zero(self) -> bool {
        self.0.canonical_value() == 0.0
    }
}

/// A finite, symmetric, positive-semidefinite correlation matrix.
///
/// Coefficients are dimensionless. Parameter order and units belong to the
/// result type that owns this matrix; the matrix cannot define them by itself.
/// Construction canonicalizes only floating-point differences within
/// [`Self::VALIDATION_TOLERANCE`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorrelationMatrix<const N: usize> {
    coefficients: [[f64; N]; N],
}

impl<const N: usize> CorrelationMatrix<N> {
    /// Absolute tolerance used for unit diagonal, symmetry, bounds, and
    /// positive-semidefinite factorization residuals.
    pub const VALIDATION_TOLERANCE: f64 = 1.0e-12;

    /// Constructs an identity correlation matrix.
    pub const fn identity() -> Self {
        let mut coefficients = [[0.0; N]; N];
        let mut index = 0;
        while index < N {
            coefficients[index][index] = 1.0;
            index += 1;
        }
        Self { coefficients }
    }

    /// Validates and canonicalizes a complete matrix.
    pub fn try_from_coefficients(mut coefficients: [[f64; N]; N]) -> Result<Self, Error> {
        let tolerance = Self::VALIDATION_TOLERANCE;
        let mut row = 0;
        while row < N {
            let diagonal = coefficients[row][row];
            if !diagonal.is_finite() || (diagonal - 1.0).abs() > tolerance {
                return Err(Error::InvalidCorrelationDiagonal {
                    index: row,
                    value: diagonal,
                });
            }
            coefficients[row][row] = 1.0;

            let mut column = row + 1;
            while column < N {
                let forward = coefficients[row][column];
                let reverse = coefficients[column][row];
                if !forward.is_finite() || forward.abs() > 1.0 + tolerance {
                    return Err(Error::InvalidCorrelationCoefficient {
                        row,
                        column,
                        value: forward,
                    });
                }
                if !reverse.is_finite() || reverse.abs() > 1.0 + tolerance {
                    return Err(Error::InvalidCorrelationCoefficient {
                        row: column,
                        column: row,
                        value: reverse,
                    });
                }
                if (forward - reverse).abs() > tolerance {
                    return Err(Error::AsymmetricCorrelation {
                        row,
                        column,
                        forward,
                        reverse,
                    });
                }
                let canonical = ((forward + reverse) * 0.5).clamp(-1.0, 1.0);
                coefficients[row][column] = canonical;
                coefficients[column][row] = canonical;
                column += 1;
            }
            row += 1;
        }

        Self::ensure_positive_semidefinite(&coefficients)?;
        Ok(Self { coefficients })
    }

    /// Returns one coefficient, or `None` when either index is out of bounds.
    pub fn coefficient(self, row: usize, column: usize) -> Option<f64> {
        self.coefficients
            .get(row)
            .and_then(|values| values.get(column))
            .copied()
    }

    /// Returns the canonicalized coefficient array.
    pub const fn coefficients(self) -> [[f64; N]; N] {
        self.coefficients
    }

    fn ensure_positive_semidefinite(coefficients: &[[f64; N]; N]) -> Result<(), Error> {
        let tolerance = Self::VALIDATION_TOLERANCE * (N.max(1) as f64);
        let mut factor = [[0.0; N]; N];
        let mut row = 0;
        while row < N {
            let mut column = 0;
            while column <= row {
                let mut residual = coefficients[row][column];
                let mut index = 0;
                while index < column {
                    residual -= factor[row][index] * factor[column][index];
                    index += 1;
                }

                if row == column {
                    if residual < -tolerance {
                        return Err(Error::CorrelationMatrixNotPositiveSemidefinite {
                            pivot: row,
                            residual,
                        });
                    }
                    factor[row][column] = if residual <= tolerance {
                        0.0
                    } else {
                        libm::sqrt(residual)
                    };
                } else if factor[column][column] > tolerance {
                    factor[row][column] = residual / factor[column][column];
                } else if residual.abs() > tolerance {
                    return Err(Error::CorrelationMatrixNotPositiveSemidefinite {
                        pivot: column,
                        residual,
                    });
                }
                column += 1;
            }
            row += 1;
        }
        Ok(())
    }
}

impl sealed::Sealed for Angle {}

impl UncertaintyQuantity for Angle {
    const QUANTITY_NAME: &'static str = "angle";
    const CANONICAL_UNIT: &'static str = "rad";

    fn canonical_value(self) -> f64 {
        self.as_radians()
    }
}

impl sealed::Sealed for AngularSpeed {}

impl UncertaintyQuantity for AngularSpeed {
    const QUANTITY_NAME: &'static str = "angular speed";
    const CANONICAL_UNIT: &'static str = "rad/s";

    fn canonical_value(self) -> f64 {
        self.as_radians_per_second()
    }
}

impl sealed::Sealed for Dimensionless {}

impl UncertaintyQuantity for Dimensionless {
    const QUANTITY_NAME: &'static str = "dimensionless value";
    const CANONICAL_UNIT: &'static str = "1";

    fn canonical_value(self) -> f64 {
        self.value()
    }
}

impl sealed::Sealed for Length {}

impl UncertaintyQuantity for Length {
    const QUANTITY_NAME: &'static str = "length";
    const CANONICAL_UNIT: &'static str = "m";

    fn canonical_value(self) -> f64 {
        self.as_metres()
    }
}

impl sealed::Sealed for Speed {}

impl UncertaintyQuantity for Speed {
    const QUANTITY_NAME: &'static str = "speed";
    const CANONICAL_UNIT: &'static str = "m/s";

    fn canonical_value(self) -> f64 {
        self.as_metres_per_second()
    }
}

impl sealed::Sealed for Acceleration {}

impl UncertaintyQuantity for Acceleration {
    const QUANTITY_NAME: &'static str = "acceleration";
    const CANONICAL_UNIT: &'static str = "m/s²";

    fn canonical_value(self) -> f64 {
        self.as_metres_per_second_squared()
    }
}

impl sealed::Sealed for Duration {}

impl UncertaintyQuantity for Duration {
    const QUANTITY_NAME: &'static str = "duration";
    const CANONICAL_UNIT: &'static str = "s";

    fn canonical_value(self) -> f64 {
        self.as_seconds_f64()
    }
}
