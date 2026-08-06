use super::Error;

/// A finite three-by-three matrix stored in row-major order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3 {
    rows: [[f64; 3]; 3],
}

impl Matrix3 {
    /// Constructs a finite matrix from row-major values.
    pub fn try_from_rows(rows: [[f64; 3]; 3]) -> Result<Self, Error> {
        for row in rows {
            for value in row {
                Error::ensure_finite("matrix element", value)?;
            }
        }
        Ok(Self { rows })
    }

    /// Returns the identity matrix.
    pub const fn identity() -> Self {
        Self {
            rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Returns the row-major values.
    pub const fn rows(self) -> [[f64; 3]; 3] {
        self.rows
    }

    /// Returns one matrix element, or `None` when either index is out of bounds.
    pub fn element(self, row: usize, column: usize) -> Option<f64> {
        self.rows
            .get(row)
            .and_then(|values| values.get(column))
            .copied()
    }

    /// Returns the transpose.
    pub fn transpose(self) -> Self {
        Self {
            rows: [
                [self.rows[0][0], self.rows[1][0], self.rows[2][0]],
                [self.rows[0][1], self.rows[1][1], self.rows[2][1]],
                [self.rows[0][2], self.rows[1][2], self.rows[2][2]],
            ],
        }
    }

    /// Returns the determinant.
    pub fn determinant(self) -> f64 {
        let [a, b, c] = self.rows[0];
        let [d, e, f] = self.rows[1];
        let [g, h, i] = self.rows[2];
        a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
    }

    /// Multiplies two matrices while preserving the finite invariant.
    pub fn checked_mul(self, rhs: Self) -> Result<Self, Error> {
        let mut rows = [[0.0; 3]; 3];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (column_index, value) in row.iter_mut().enumerate() {
                *value = self.rows[row_index][0] * rhs.rows[0][column_index]
                    + self.rows[row_index][1] * rhs.rows[1][column_index]
                    + self.rows[row_index][2] * rhs.rows[2][column_index];
            }
        }
        Self::try_from_rows(rows)
    }

    /// Returns the inverse when the matrix is non-singular and finite.
    pub fn inverse(self) -> Result<Self, Error> {
        let determinant = self.determinant();
        Error::ensure_finite("matrix determinant", determinant)?;
        if determinant == 0.0 {
            return Err(Error::SingularMatrix { determinant });
        }

        let [a, b, c] = self.rows[0];
        let [d, e, f] = self.rows[1];
        let [g, h, i] = self.rows[2];
        let inverse_determinant = 1.0 / determinant;
        Self::try_from_rows([
            [
                (e * i - f * h) * inverse_determinant,
                (c * h - b * i) * inverse_determinant,
                (b * f - c * e) * inverse_determinant,
            ],
            [
                (f * g - d * i) * inverse_determinant,
                (a * i - c * g) * inverse_determinant,
                (c * d - a * f) * inverse_determinant,
            ],
            [
                (d * h - e * g) * inverse_determinant,
                (b * g - a * h) * inverse_determinant,
                (a * e - b * d) * inverse_determinant,
            ],
        ])
    }

    /// Returns the maximum absolute element of RᵀR - I.
    pub fn orthogonality_residual(self) -> Result<f64, Error> {
        let product = self.transpose().checked_mul(self)?;
        let mut residual = 0.0_f64;
        for row in 0..3 {
            for column in 0..3 {
                let expected = if row == column { 1.0 } else { 0.0 };
                residual = residual.max((product.rows[row][column] - expected).abs());
            }
        }
        Ok(residual)
    }

    pub(crate) fn checked_mul_components(self, rhs: [f64; 3]) -> Result<[f64; 3], Error> {
        let result = [
            self.rows[0][0] * rhs[0] + self.rows[0][1] * rhs[1] + self.rows[0][2] * rhs[2],
            self.rows[1][0] * rhs[0] + self.rows[1][1] * rhs[1] + self.rows[1][2] * rhs[2],
            self.rows[2][0] * rhs[0] + self.rows[2][1] * rhs[1] + self.rows[2][2] * rhs[2],
        ];
        Error::ensure_finite("matrix-vector product x", result[0])?;
        Error::ensure_finite("matrix-vector product y", result[1])?;
        Error::ensure_finite("matrix-vector product z", result[2])?;
        Ok(result)
    }
}
