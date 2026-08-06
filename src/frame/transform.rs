use crate::{
    math::{AngularSpeed, Coordinate, Direction, Length, Point3, Rotation, Speed, Vector3},
    time::{Instant, TimeScale},
};

use super::{CoordinateFrame, Error, State};

/// A source-to-target coordinate transform valid at one physical epoch.
///
/// Position components follow `r_to = R r_from + t`, where `t` is the source
/// origin relative to the target origin, expressed in the target frame.
/// Velocity components follow
/// `v_to = R v_from + omega × (R r_from) + t_dot`. `omega` is expressed in
/// the target frame and is the axial vector satisfying `R_dot R_transpose =
/// [omega]_cross`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateTransform<From, To, S>
where
    From: CoordinateFrame,
    To: CoordinateFrame,
    S: TimeScale,
{
    epoch: Instant<S>,
    rotation: Rotation<From, To>,
    angular_velocity: Vector3<To, AngularSpeed>,
    translation: Vector3<To, Length>,
    translation_rate: Vector3<To, Speed>,
}

impl<From, To, S> StateTransform<From, To, S>
where
    From: CoordinateFrame,
    To: CoordinateFrame,
    S: TimeScale,
{
    /// Constructs a transform from already validated typed components.
    pub const fn new(
        epoch: Instant<S>,
        rotation: Rotation<From, To>,
        angular_velocity: Vector3<To, AngularSpeed>,
        translation: Vector3<To, Length>,
        translation_rate: Vector3<To, Speed>,
    ) -> Self {
        Self {
            epoch,
            rotation,
            angular_velocity,
            translation,
            translation_rate,
        }
    }

    /// Returns the physical epoch at which this transform is valid.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the source-to-target component rotation.
    pub const fn rotation(self) -> Rotation<From, To> {
        self.rotation
    }

    /// Returns the target-expressed angular velocity of the component rotation.
    pub const fn angular_velocity(self) -> Vector3<To, AngularSpeed> {
        self.angular_velocity
    }

    /// Returns the source origin relative to the target origin in target coordinates.
    pub const fn translation(self) -> Vector3<To, Length> {
        self.translation
    }

    /// Returns the physical-second derivative of the target-expressed translation.
    pub const fn translation_rate(self) -> Vector3<To, Speed> {
        self.translation_rate
    }

    /// Applies only the rotational part to a free vector.
    pub fn apply_vector<Q: Coordinate>(
        self,
        vector: Vector3<From, Q>,
    ) -> Result<Vector3<To, Q>, Error> {
        self.rotation.apply_vector(vector).map_err(Error::from)
    }

    /// Applies only the rotational part to a unit direction.
    pub fn apply_direction(self, direction: Direction<From>) -> Result<Direction<To>, Error> {
        self.rotation
            .apply_direction(direction)
            .map_err(Error::from)
    }

    /// Applies rotation and origin translation to a point.
    pub fn apply_position(self, position: Point3<From>) -> Result<Point3<To>, Error> {
        self.rotation
            .apply_vector(position.position())?
            .checked_add(self.translation)
            .map(Point3::from_position)
            .map_err(Error::from)
    }

    /// Applies the full six-dimensional transform to a state at the same epoch.
    pub fn apply_state(self, state: State<From, S>) -> Result<State<To, S>, Error> {
        self.ensure_epoch(state.epoch())?;

        let rotated_position = self.rotation.apply_vector(state.position().position())?;
        let position = Point3::from_position(rotated_position.checked_add(self.translation)?);
        let rotational_velocity =
            Self::angular_cross_position(self.angular_velocity, rotated_position)?;
        let velocity = self
            .rotation
            .apply_vector(state.velocity())?
            .checked_add(rotational_velocity)?
            .checked_add(self.translation_rate)?;

        Ok(State::new(position, velocity, state.epoch()))
    }

    /// Returns the target-to-source transform at the same physical epoch.
    pub fn inverse(self) -> Result<StateTransform<To, From, S>, Error> {
        let rotation = self.rotation.inverse();
        let translation = rotation
            .apply_vector(self.translation)?
            .checked_scale(-1.0)?;
        let angular_velocity = rotation
            .apply_vector(self.angular_velocity)?
            .checked_scale(-1.0)?;
        let rotation_translation_velocity =
            Self::angular_cross_position(self.angular_velocity, self.translation)?;
        let translation_rate = rotation
            .apply_vector(rotation_translation_velocity.checked_sub(self.translation_rate)?)?;

        Ok(StateTransform::new(
            self.epoch,
            rotation,
            angular_velocity,
            translation,
            translation_rate,
        ))
    }

    /// Composes this transform with a following transform at the same epoch.
    pub fn then<Next>(
        self,
        next: StateTransform<To, Next, S>,
    ) -> Result<StateTransform<From, Next, S>, Error>
    where
        Next: CoordinateFrame,
    {
        self.ensure_epoch(next.epoch)?;

        let translated_source_origin = next.rotation.apply_vector(self.translation)?;
        let translation = translated_source_origin.checked_add(next.translation)?;
        let angular_velocity = next
            .rotation
            .apply_vector(self.angular_velocity)?
            .checked_add(next.angular_velocity)?;
        let rotation_translation_velocity =
            Self::angular_cross_position(next.angular_velocity, translated_source_origin)?;
        let translation_rate = next
            .rotation
            .apply_vector(self.translation_rate)?
            .checked_add(rotation_translation_velocity)?
            .checked_add(next.translation_rate)?;
        let rotation = self.rotation.then(next.rotation)?;

        Ok(StateTransform::new(
            self.epoch,
            rotation,
            angular_velocity,
            translation,
            translation_rate,
        ))
    }

    fn ensure_epoch(self, value: Instant<S>) -> Result<(), Error> {
        let transform_tai_nanoseconds = self.epoch.tai_nanoseconds_since_1900();
        let value_tai_nanoseconds = value.tai_nanoseconds_since_1900();
        if transform_tai_nanoseconds == value_tai_nanoseconds {
            Ok(())
        } else {
            Err(Error::epoch_mismatch(
                transform_tai_nanoseconds,
                value_tai_nanoseconds,
            ))
        }
    }

    fn angular_cross_position<F>(
        angular_velocity: Vector3<F, AngularSpeed>,
        position: Vector3<F, Length>,
    ) -> Result<Vector3<F, Speed>, crate::math::Error> {
        let [omega_x, omega_y, omega_z] = angular_velocity.components();
        let [x, y, z] = position.components();
        Ok(Vector3::from_array([
            Speed::from_metres_per_second(
                omega_y.as_radians_per_second() * z.as_metres()
                    - omega_z.as_radians_per_second() * y.as_metres(),
            )?,
            Speed::from_metres_per_second(
                omega_z.as_radians_per_second() * x.as_metres()
                    - omega_x.as_radians_per_second() * z.as_metres(),
            )?,
            Speed::from_metres_per_second(
                omega_x.as_radians_per_second() * y.as_metres()
                    - omega_y.as_radians_per_second() * x.as_metres(),
            )?,
        ]))
    }
}

impl<F, S> StateTransform<F, F, S>
where
    F: CoordinateFrame,
    S: TimeScale,
{
    /// Constructs the identity state transform at a physical epoch.
    pub fn identity(epoch: Instant<S>) -> Result<Self, Error> {
        let zero_angular_speed = AngularSpeed::from_radians_per_second(0.0)?;
        let zero_length = Length::from_metres(0.0)?;
        let zero_speed = Speed::from_metres_per_second(0.0)?;
        Ok(Self::new(
            epoch,
            Rotation::identity(),
            Vector3::new(zero_angular_speed, zero_angular_speed, zero_angular_speed),
            Vector3::new(zero_length, zero_length, zero_length),
            Vector3::new(zero_speed, zero_speed, zero_speed),
        ))
    }
}
