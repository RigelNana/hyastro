use crate::{
    constants::earth::{
        ROTATION_DETERMINANT_TOLERANCE, ROTATION_ORTHOGONALITY_TOLERANCE,
        ROTATION_RATE_CONVERGENCE_TOLERANCE_RADIANS_PER_SECOND,
        ROTATION_RATE_DIFFERENCE_STEP_SECONDS,
    },
    math::{Angle, AngularSpeed, Matrix3, Rotation, RotationTolerance, Vector3},
    time::{
        Duration, EarthOrientation, EarthOrientationTable, Instant, JulianDate, TimeContext,
        TimeScale, Tt,
    },
};

use super::{Cirs, CoordinateFrame, Error, Gcrs, Itrs, Tirs};

pub(super) struct KinematicRotation<From, To>
where
    From: CoordinateFrame,
    To: CoordinateFrame,
{
    rotation: Rotation<From, To>,
    angular_velocity: Vector3<To, AngularSpeed>,
}

impl<From, To> KinematicRotation<From, To>
where
    From: CoordinateFrame,
    To: CoordinateFrame,
{
    pub(super) const fn rotation(&self) -> Rotation<From, To> {
        self.rotation
    }

    pub(super) const fn angular_velocity(&self) -> Vector3<To, AngularSpeed> {
        self.angular_velocity
    }
}

pub(super) struct Iau2006A;

impl Iau2006A {
    pub(super) fn gcrs_to_cirs<S: TimeScale>(
        epoch: Instant<S>,
        time: &TimeContext<'_, EarthOrientationTable<'_>>,
        orientation: EarthOrientation<S>,
    ) -> Result<KinematicRotation<Gcrs, Cirs>, Error> {
        let tt = JulianDate::<Tt>::from_instant(epoch, time)?;
        let offset_x = orientation.celestial_pole_offset_x().as_angle();
        let offset_y = orientation.celestial_pole_offset_y().as_angle();
        let rate_x = RotationRate::required_rate(
            orientation.celestial_pole_offset_rate_x(),
            "celestial-pole offset dX",
            epoch,
        )?;
        let rate_y = RotationRate::required_rate(
            orientation.celestial_pole_offset_rate_y(),
            "celestial-pole offset dY",
            epoch,
        )?;
        RotationRate::differentiate(epoch, |offset_seconds| {
            let shifted_tt =
                tt.checked_add_duration(Duration::from_seconds_f64(offset_seconds)?)?;
            let shifted_x = Angle::from_radians(
                offset_x.as_radians() + rate_x.as_radians_per_second() * offset_seconds,
            )?;
            let shifted_y = Angle::from_radians(
                offset_y.as_radians() + rate_y.as_radians_per_second() * offset_seconds,
            )?;
            Self::matrix(shifted_tt, shifted_x, shifted_y)
        })
    }

    fn matrix(tt: JulianDate<Tt>, offset_x: Angle, offset_y: Angle) -> Result<Matrix3, Error> {
        let (tt_first, tt_second) = tt.parts();
        let (x, y, s) = sofars::pnp::xys06a(tt_first, tt_second);
        Matrix3::try_from_rows(sofars::pnp::c2ixys(
            x + offset_x.as_radians(),
            y + offset_y.as_radians(),
            s,
        ))
        .map_err(Error::from)
    }
}

pub(super) struct IersPolarMotion;

impl IersPolarMotion {
    pub(super) fn tirs_to_itrs<S: TimeScale>(
        epoch: Instant<S>,
        time: &TimeContext<'_, EarthOrientationTable<'_>>,
        orientation: EarthOrientation<S>,
    ) -> Result<KinematicRotation<Tirs, Itrs>, Error> {
        let tt = JulianDate::<Tt>::from_instant(epoch, time)?;
        let polar_motion_x = orientation.polar_motion_x().as_angle();
        let polar_motion_y = orientation.polar_motion_y().as_angle();
        let rate_x = RotationRate::required_rate(
            orientation.polar_motion_rate_x(),
            "polar motion xp",
            epoch,
        )?;
        let rate_y = RotationRate::required_rate(
            orientation.polar_motion_rate_y(),
            "polar motion yp",
            epoch,
        )?;
        RotationRate::differentiate(epoch, |offset_seconds| {
            let shifted_tt =
                tt.checked_add_duration(Duration::from_seconds_f64(offset_seconds)?)?;
            let shifted_x = Angle::from_radians(
                polar_motion_x.as_radians() + rate_x.as_radians_per_second() * offset_seconds,
            )?;
            let shifted_y = Angle::from_radians(
                polar_motion_y.as_radians() + rate_y.as_radians_per_second() * offset_seconds,
            )?;
            Self::matrix(shifted_tt, shifted_x, shifted_y)
        })
    }

    fn matrix(
        tt: JulianDate<Tt>,
        polar_motion_x: Angle,
        polar_motion_y: Angle,
    ) -> Result<Matrix3, Error> {
        let (tt_first, tt_second) = tt.parts();
        let tio_locator = sofars::pnp::sp00(tt_first, tt_second);
        Matrix3::try_from_rows(sofars::pnp::pom00(
            polar_motion_x.as_radians(),
            polar_motion_y.as_radians(),
            tio_locator,
        ))
        .map_err(Error::from)
    }
}

struct RotationRate;

impl RotationRate {
    fn differentiate<From, To, Evaluate>(
        epoch: Instant<impl TimeScale>,
        evaluate: Evaluate,
    ) -> Result<KinematicRotation<From, To>, Error>
    where
        From: CoordinateFrame,
        To: CoordinateFrame,
        Evaluate: Fn(f64) -> Result<Matrix3, Error>,
    {
        let current = evaluate(0.0)?;
        let coarse = Self::central_derivative(
            evaluate(-ROTATION_RATE_DIFFERENCE_STEP_SECONDS)?,
            evaluate(ROTATION_RATE_DIFFERENCE_STEP_SECONDS)?,
            ROTATION_RATE_DIFFERENCE_STEP_SECONDS,
        )?;
        let fine_step = ROTATION_RATE_DIFFERENCE_STEP_SECONDS / 2.0;
        let fine =
            Self::central_derivative(evaluate(-fine_step)?, evaluate(fine_step)?, fine_step)?;
        let coarse_omega = Self::axial(current, coarse)?;
        let fine_omega = Self::axial(current, fine)?;
        let mut extrapolated = [0.0; 3];
        let mut residual = 0.0_f64;
        for index in 0..3 {
            let correction = (fine_omega[index] - coarse_omega[index]) / 3.0;
            extrapolated[index] = fine_omega[index] + correction;
            residual = residual.max(correction.abs());
        }

        let extrapolated_derivative = Self::richardson_derivative(coarse, fine)?;
        residual = residual.max(Self::skew_residual(current, extrapolated_derivative)?);
        if residual > ROTATION_RATE_CONVERGENCE_TOLERANCE_RADIANS_PER_SECOND {
            return Err(Error::RotationRateDidNotConverge {
                residual,
                tolerance: ROTATION_RATE_CONVERGENCE_TOLERANCE_RADIANS_PER_SECOND,
            });
        }

        let tolerance = RotationTolerance::new(
            ROTATION_ORTHOGONALITY_TOLERANCE,
            ROTATION_DETERMINANT_TOLERANCE,
        )?;
        let rotation = Rotation::<From, To>::try_from_matrix(current, tolerance)?;
        let angular_velocity = Vector3::from_array([
            AngularSpeed::from_radians_per_second(extrapolated[0])?,
            AngularSpeed::from_radians_per_second(extrapolated[1])?,
            AngularSpeed::from_radians_per_second(extrapolated[2])?,
        ]);
        let _ = epoch;
        Ok(KinematicRotation {
            rotation,
            angular_velocity,
        })
    }

    fn required_rate<S: TimeScale>(
        rate: Option<AngularSpeed>,
        field: &'static str,
        epoch: Instant<S>,
    ) -> Result<AngularSpeed, Error> {
        rate.ok_or(Error::EarthOrientationRateUnavailable {
            field,
            epoch_tai_nanoseconds: epoch.tai_nanoseconds_since_1900(),
        })
    }

    fn central_derivative(
        minus: Matrix3,
        plus: Matrix3,
        half_span_seconds: f64,
    ) -> Result<Matrix3, Error> {
        let minus = minus.rows();
        let plus = plus.rows();
        let denominator = 2.0 * half_span_seconds;
        let mut rows = [[0.0; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                rows[row][column] = (plus[row][column] - minus[row][column]) / denominator;
            }
        }
        Matrix3::try_from_rows(rows).map_err(Error::from)
    }

    fn richardson_derivative(coarse: Matrix3, fine: Matrix3) -> Result<Matrix3, Error> {
        let coarse = coarse.rows();
        let fine = fine.rows();
        let mut rows = [[0.0; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                rows[row][column] =
                    fine[row][column] + (fine[row][column] - coarse[row][column]) / 3.0;
            }
        }
        Matrix3::try_from_rows(rows).map_err(Error::from)
    }

    fn axial(rotation: Matrix3, derivative: Matrix3) -> Result<[f64; 3], Error> {
        let product = derivative.checked_mul(rotation.transpose())?.rows();
        Ok([
            (product[2][1] - product[1][2]) / 2.0,
            (product[0][2] - product[2][0]) / 2.0,
            (product[1][0] - product[0][1]) / 2.0,
        ])
    }

    fn skew_residual(rotation: Matrix3, derivative: Matrix3) -> Result<f64, Error> {
        let product = derivative.checked_mul(rotation.transpose())?.rows();
        let residual = product[0][0]
            .abs()
            .max(product[1][1].abs())
            .max(product[2][2].abs())
            .max(((product[0][1] + product[1][0]) / 2.0).abs())
            .max(((product[0][2] + product[2][0]) / 2.0).abs())
            .max(((product[1][2] + product[2][1]) / 2.0).abs());
        Ok(residual)
    }
}
