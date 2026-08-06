use core::{fmt, marker::PhantomData};

use libm::{acos, sin, sqrt};

use super::{Angle, Coordinate, Direction, Error, Matrix3, Vector3};

/// Explicit tolerances used to validate a rotation matrix.
#[cfg_attr(feature = "std", derive(garde::Validate))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotationTolerance {
    #[cfg_attr(
        feature = "std",
        garde(range(min = f64::MIN_POSITIVE, max = f64::MAX))
    )]
    orthogonality: f64,
    #[cfg_attr(
        feature = "std",
        garde(range(min = f64::MIN_POSITIVE, max = f64::MAX))
    )]
    determinant: f64,
}

impl RotationTolerance {
    /// Constructs finite, normal, positive orthogonality and determinant tolerances.
    pub fn new(orthogonality: f64, determinant: f64) -> Result<Self, Error> {
        let candidate = Self {
            orthogonality,
            determinant,
        };
        #[cfg(feature = "std")]
        {
            use garde::Validate as _;
            if candidate.validate().is_ok() {
                return Ok(candidate);
            }
        }
        Error::ensure_positive_tolerance("orthogonality", orthogonality)?;
        Error::ensure_positive_tolerance("determinant", determinant)?;
        Ok(candidate)
    }

    /// Returns the maximum allowed element residual in RᵀR - I.
    pub const fn orthogonality(self) -> f64 {
        self.orthogonality
    }

    /// Returns the maximum allowed absolute determinant error from one.
    pub const fn determinant(self) -> f64 {
        self.determinant
    }
}

/// A validated rotation from one coordinate frame to another.
pub struct Rotation<From, To> {
    matrix: Matrix3,
    frames: PhantomData<fn(From) -> To>,
}

impl<From, To> Rotation<From, To> {
    /// Validates and constructs a rotation matrix with explicit tolerances.
    pub fn try_from_matrix(matrix: Matrix3, tolerance: RotationTolerance) -> Result<Self, Error> {
        let orthogonality_residual = matrix.orthogonality_residual()?;
        let determinant = matrix.determinant();
        Error::ensure_finite("rotation determinant", determinant)?;
        if orthogonality_residual > tolerance.orthogonality
            || (determinant - 1.0).abs() > tolerance.determinant
        {
            return Err(Error::InvalidRotation {
                orthogonality_residual,
                determinant,
                orthogonality_tolerance: tolerance.orthogonality,
                determinant_tolerance: tolerance.determinant,
            });
        }
        Ok(Self::from_validated_matrix(matrix))
    }

    /// Returns the underlying validated matrix.
    pub const fn matrix(self) -> Matrix3 {
        self.matrix
    }

    /// Returns the inverse rotation.
    pub fn inverse(self) -> Rotation<To, From> {
        Rotation::from_validated_matrix(self.matrix.transpose())
    }

    /// Applies the rotation to a vector while changing its frame type.
    pub fn apply_vector<Q: Coordinate>(
        self,
        vector: Vector3<From, Q>,
    ) -> Result<Vector3<To, Q>, Error> {
        Vector3::from_canonical(
            self.matrix
                .checked_mul_components(vector.canonical_components())?,
        )
    }

    /// Applies the rotation to a unit direction.
    pub fn apply_direction(self, direction: Direction<From>) -> Result<Direction<To>, Error> {
        Direction::try_from_components(self.matrix.checked_mul_components(direction.components())?)
    }

    /// Composes this rotation with a following rotation.
    pub fn then<Next>(self, next: Rotation<To, Next>) -> Result<Rotation<From, Next>, Error> {
        Ok(Rotation::from_validated_matrix(
            next.matrix.checked_mul(self.matrix)?,
        ))
    }

    fn from_validated_matrix(matrix: Matrix3) -> Self {
        Self {
            matrix,
            frames: PhantomData,
        }
    }
}

impl<F> Rotation<F, F> {
    /// Returns the identity rotation.
    pub const fn identity() -> Self {
        Self {
            matrix: Matrix3::identity(),
            frames: PhantomData,
        }
    }

    /// Constructs an active right-handed rotation around the x axis.
    pub fn around_x(angle: Angle) -> Result<Self, Error> {
        let (sine, cosine) = angle.sin_cos();
        Ok(Self::from_validated_matrix(Matrix3::try_from_rows([
            [1.0, 0.0, 0.0],
            [0.0, cosine.value(), -sine.value()],
            [0.0, sine.value(), cosine.value()],
        ])?))
    }

    /// Constructs an active right-handed rotation around the y axis.
    pub fn around_y(angle: Angle) -> Result<Self, Error> {
        let (sine, cosine) = angle.sin_cos();
        Ok(Self::from_validated_matrix(Matrix3::try_from_rows([
            [cosine.value(), 0.0, sine.value()],
            [0.0, 1.0, 0.0],
            [-sine.value(), 0.0, cosine.value()],
        ])?))
    }

    /// Constructs an active right-handed rotation around the z axis.
    pub fn around_z(angle: Angle) -> Result<Self, Error> {
        let (sine, cosine) = angle.sin_cos();
        Ok(Self::from_validated_matrix(Matrix3::try_from_rows([
            [cosine.value(), -sine.value(), 0.0],
            [sine.value(), cosine.value(), 0.0],
            [0.0, 0.0, 1.0],
        ])?))
    }

    /// Constructs an active right-handed rotation around a typed axis.
    pub fn around_axis(axis: Direction<F>, angle: Angle) -> Result<Self, Error> {
        Quaternion::from_axis_angle(axis, angle)?.to_rotation()
    }
}

impl<From, To> Copy for Rotation<From, To> {}

impl<From, To> Clone for Rotation<From, To> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<From, To> PartialEq for Rotation<From, To> {
    fn eq(&self, other: &Self) -> bool {
        self.matrix == other.matrix
    }
}

impl<From, To> fmt::Debug for Rotation<From, To> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Rotation")
            .field(&self.matrix)
            .finish()
    }
}

/// A unit quaternion representing a typed rotation.
pub struct Quaternion<From, To> {
    scalar: f64,
    vector: [f64; 3],
    frames: PhantomData<fn(From) -> To>,
}

impl<From, To> Quaternion<From, To> {
    /// Constructs and normalizes a quaternion from scalar-first components.
    pub fn try_from_components(scalar: f64, x: f64, y: f64, z: f64) -> Result<Self, Error> {
        Error::ensure_finite("quaternion scalar", scalar)?;
        Error::ensure_finite("quaternion x", x)?;
        Error::ensure_finite("quaternion y", y)?;
        Error::ensure_finite("quaternion z", z)?;
        let norm = sqrt(scalar * scalar + x * x + y * y + z * z);
        if norm == 0.0 || !norm.is_finite() {
            return Err(Error::InvalidQuaternion { norm });
        }
        Ok(Self {
            scalar: scalar / norm,
            vector: [x / norm, y / norm, z / norm],
            frames: PhantomData,
        })
    }

    /// Converts a validated rotation matrix to a unit quaternion.
    pub fn from_rotation(rotation: Rotation<From, To>) -> Result<Self, Error> {
        let matrix = rotation.matrix.rows();
        let trace = matrix[0][0] + matrix[1][1] + matrix[2][2];
        let (scalar, x, y, z) = if trace > 0.0 {
            let scale = sqrt(trace + 1.0) * 2.0;
            (
                0.25 * scale,
                (matrix[2][1] - matrix[1][2]) / scale,
                (matrix[0][2] - matrix[2][0]) / scale,
                (matrix[1][0] - matrix[0][1]) / scale,
            )
        } else if matrix[0][0] > matrix[1][1] && matrix[0][0] > matrix[2][2] {
            let scale = sqrt(1.0 + matrix[0][0] - matrix[1][1] - matrix[2][2]) * 2.0;
            (
                (matrix[2][1] - matrix[1][2]) / scale,
                0.25 * scale,
                (matrix[0][1] + matrix[1][0]) / scale,
                (matrix[0][2] + matrix[2][0]) / scale,
            )
        } else if matrix[1][1] > matrix[2][2] {
            let scale = sqrt(1.0 + matrix[1][1] - matrix[0][0] - matrix[2][2]) * 2.0;
            (
                (matrix[0][2] - matrix[2][0]) / scale,
                (matrix[0][1] + matrix[1][0]) / scale,
                0.25 * scale,
                (matrix[1][2] + matrix[2][1]) / scale,
            )
        } else {
            let scale = sqrt(1.0 + matrix[2][2] - matrix[0][0] - matrix[1][1]) * 2.0;
            (
                (matrix[1][0] - matrix[0][1]) / scale,
                (matrix[0][2] + matrix[2][0]) / scale,
                (matrix[1][2] + matrix[2][1]) / scale,
                0.25 * scale,
            )
        };
        Self::try_from_components(scalar, x, y, z)
    }

    /// Returns scalar-first quaternion components.
    pub const fn components(self) -> [f64; 4] {
        [self.scalar, self.vector[0], self.vector[1], self.vector[2]]
    }

    /// Converts the unit quaternion to a typed rotation.
    pub fn to_rotation(self) -> Result<Rotation<From, To>, Error> {
        let [x, y, z] = self.vector;
        let w = self.scalar;
        Ok(Rotation::from_validated_matrix(Matrix3::try_from_rows([
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
            ],
            [
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
            ],
            [
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ])?))
    }

    /// Returns the inverse unit quaternion.
    pub fn inverse(self) -> Quaternion<To, From> {
        Quaternion {
            scalar: self.scalar,
            vector: [-self.vector[0], -self.vector[1], -self.vector[2]],
            frames: PhantomData,
        }
    }

    /// Composes this quaternion with a following quaternion.
    pub fn then<Next>(self, next: Quaternion<To, Next>) -> Result<Quaternion<From, Next>, Error> {
        let aw = next.scalar;
        let [ax, ay, az] = next.vector;
        let bw = self.scalar;
        let [bx, by, bz] = self.vector;
        Quaternion::try_from_components(
            aw * bw - ax * bx - ay * by - az * bz,
            aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
        )
    }

    /// Applies the quaternion rotation to a vector.
    pub fn apply_vector<Q: Coordinate>(
        self,
        vector: Vector3<From, Q>,
    ) -> Result<Vector3<To, Q>, Error> {
        self.to_rotation()?.apply_vector(vector)
    }

    /// Interpolates along the shortest quaternion arc for t in [0, 1].
    pub fn slerp(self, mut rhs: Self, t: f64) -> Result<Self, Error> {
        Error::ensure_finite("quaternion interpolation fraction", t)?;
        if !(0.0..=1.0).contains(&t) {
            return Err(Error::OutOfRange {
                field: "quaternion interpolation fraction",
                value: t,
                interval: "[0, 1]",
                unit: "",
            });
        }

        let mut dot = self.scalar * rhs.scalar
            + self.vector[0] * rhs.vector[0]
            + self.vector[1] * rhs.vector[1]
            + self.vector[2] * rhs.vector[2];
        if dot < 0.0 {
            dot = -dot;
            rhs.scalar = -rhs.scalar;
            rhs.vector = [-rhs.vector[0], -rhs.vector[1], -rhs.vector[2]];
        }
        dot = dot.clamp(-1.0, 1.0);

        if dot > 0.9995 {
            return Self::try_from_components(
                self.scalar + t * (rhs.scalar - self.scalar),
                self.vector[0] + t * (rhs.vector[0] - self.vector[0]),
                self.vector[1] + t * (rhs.vector[1] - self.vector[1]),
                self.vector[2] + t * (rhs.vector[2] - self.vector[2]),
            );
        }

        let theta = acos(dot);
        let denominator = sin(theta);
        let left_weight = sin((1.0 - t) * theta) / denominator;
        let right_weight = sin(t * theta) / denominator;
        Self::try_from_components(
            left_weight * self.scalar + right_weight * rhs.scalar,
            left_weight * self.vector[0] + right_weight * rhs.vector[0],
            left_weight * self.vector[1] + right_weight * rhs.vector[1],
            left_weight * self.vector[2] + right_weight * rhs.vector[2],
        )
    }
}

impl<F> Quaternion<F, F> {
    /// Returns the identity quaternion.
    pub const fn identity() -> Self {
        Self {
            scalar: 1.0,
            vector: [0.0, 0.0, 0.0],
            frames: PhantomData,
        }
    }

    /// Constructs a quaternion from a typed axis and angle.
    pub fn from_axis_angle(axis: Direction<F>, angle: Angle) -> Result<Self, Error> {
        let half = angle.checked_scale(0.5)?;
        let (sine, cosine) = half.sin_cos();
        let [x, y, z] = axis.components();
        Self::try_from_components(
            cosine.value(),
            x * sine.value(),
            y * sine.value(),
            z * sine.value(),
        )
    }
}

impl<From, To> Copy for Quaternion<From, To> {}

impl<From, To> Clone for Quaternion<From, To> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<From, To> PartialEq for Quaternion<From, To> {
    fn eq(&self, other: &Self) -> bool {
        self.scalar == other.scalar && self.vector == other.vector
    }
}

impl<From, To> fmt::Debug for Quaternion<From, To> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Quaternion")
            .field(&self.components())
            .finish()
    }
}
