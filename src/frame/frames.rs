use crate::{
    math::{AngularSpeed, Length, Matrix3, Rotation, RotationTolerance, Speed, Vector3},
    time::{EarthOrientationTable, Instant, TimeContext, TimeScale},
};

use super::{
    Cirs, CoordinateFrame, Error, FrameRotation, Gcrs, Itrs, State, StateTransform, Tirs,
    earth_orientation::{Iau2006A, IersPolarMotion, KinematicRotation},
};

mod sealed {
    pub trait Sealed<From, To, S> {}
}

/// A supported source-to-target state-transform model at time scale `S`.
///
/// This trait is sealed. The available implementations are the static paths
/// that [`Frames`] can compute from its concrete model data.
pub trait StateTransformModel<From, To, S>: sealed::Sealed<From, To, S>
where
    From: CoordinateFrame,
    To: CoordinateFrame,
    S: TimeScale,
{
    /// Computes the transform at one physical epoch.
    #[doc(hidden)]
    fn state_transform_at(&self, epoch: Instant<S>) -> Result<StateTransform<From, To, S>, Error>;
}

/// Typed astronomical frame transformations backed by one time context.
#[derive(Debug, Clone, Copy)]
pub struct Frames<'context, 'leap, 'eop> {
    time: &'context TimeContext<'leap, EarthOrientationTable<'eop>>,
}

impl<'context, 'leap, 'eop> Frames<'context, 'leap, 'eop> {
    /// Constructs frame algorithms from a time context carrying EOP data.
    pub const fn new(time: &'context TimeContext<'leap, EarthOrientationTable<'eop>>) -> Self {
        Self { time }
    }

    /// Returns the time and Earth-orientation context used by these algorithms.
    pub const fn time_context(self) -> &'context TimeContext<'leap, EarthOrientationTable<'eop>> {
        self.time
    }

    /// Computes one statically supported frame transform at an epoch.
    pub fn at<From, To, S>(&self, epoch: Instant<S>) -> Result<StateTransform<From, To, S>, Error>
    where
        From: CoordinateFrame,
        To: CoordinateFrame,
        S: TimeScale,
        Self: StateTransformModel<From, To, S>,
    {
        <Self as StateTransformModel<From, To, S>>::state_transform_at(self, epoch)
    }

    /// Converts a state through one statically supported frame path.
    pub fn transform<From, To, S>(&self, state: State<From, S>) -> Result<State<To, S>, Error>
    where
        From: CoordinateFrame,
        To: CoordinateFrame,
        S: TimeScale,
        Self: StateTransformModel<From, To, S>,
    {
        self.at::<From, To, S>(state.epoch())?.apply_state(state)
    }

    fn gcrs_to_cirs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Gcrs, Cirs, S>, Error> {
        let orientation = self.time.earth_orientation_at(epoch)?;
        let rotation = Iau2006A::gcrs_to_cirs(epoch, self.time, orientation)?;
        Self::earth_centered_transform(epoch, &rotation)
    }

    fn cirs_to_gcrs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Cirs, Gcrs, S>, Error> {
        self.gcrs_to_cirs(epoch)?.inverse()
    }

    fn cirs_to_tirs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Cirs, Tirs, S>, Error> {
        const NOMINAL_DAY_SECONDS: f64 = 86_400.0;
        const NOMINAL_EARTH_ANGULAR_SPEED: f64 = 7.292_115_0e-5;

        let orientation = self.time.earth_orientation_at(epoch)?;
        let ut1 = self.time.julian_date_from_orientation(epoch, orientation)?;
        let (ut1_first, ut1_second) = ut1.parts();
        let earth_rotation_angle = sofars::erst::era00(ut1_first, ut1_second);

        let mut rows = [[0.0; 3]; 3];
        sofars::vm::ir(&mut rows);
        sofars::vm::rz(earth_rotation_angle, &mut rows);
        let matrix = Matrix3::try_from_rows(rows)?;
        let tolerance = RotationTolerance::new(1.0e-12, 1.0e-12)?;
        let rotation = Rotation::<Cirs, Tirs>::try_from_matrix(matrix, tolerance)?;
        let frame_rotation = FrameRotation::new(epoch, rotation);

        let excess_day_seconds = orientation
            .excess_length_of_day()
            .as_duration()
            .as_seconds_f64();
        let angular_speed = NOMINAL_EARTH_ANGULAR_SPEED * NOMINAL_DAY_SECONDS
            / (NOMINAL_DAY_SECONDS + excess_day_seconds);
        let zero_angular_speed = AngularSpeed::from_radians_per_second(0.0)?;
        let negative_angular_speed = AngularSpeed::from_radians_per_second(-angular_speed)?;
        let zero_length = Length::from_metres(0.0)?;
        let zero_speed = Speed::from_metres_per_second(0.0)?;

        Ok(StateTransform::new(
            frame_rotation.epoch(),
            frame_rotation.rotation(),
            Vector3::new(
                zero_angular_speed,
                zero_angular_speed,
                negative_angular_speed,
            ),
            Vector3::new(zero_length, zero_length, zero_length),
            Vector3::new(zero_speed, zero_speed, zero_speed),
        ))
    }

    fn tirs_to_cirs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Tirs, Cirs, S>, Error> {
        self.cirs_to_tirs(epoch)?.inverse()
    }

    fn tirs_to_itrs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Tirs, Itrs, S>, Error> {
        let orientation = self.time.earth_orientation_at(epoch)?;
        let rotation = IersPolarMotion::tirs_to_itrs(epoch, self.time, orientation)?;
        Self::earth_centered_transform(epoch, &rotation)
    }

    fn itrs_to_tirs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Tirs, S>, Error> {
        self.tirs_to_itrs(epoch)?.inverse()
    }

    fn gcrs_to_tirs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Gcrs, Tirs, S>, Error> {
        self.gcrs_to_cirs(epoch)?.then(self.cirs_to_tirs(epoch)?)
    }

    fn tirs_to_gcrs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Tirs, Gcrs, S>, Error> {
        self.gcrs_to_tirs(epoch)?.inverse()
    }

    fn cirs_to_itrs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Cirs, Itrs, S>, Error> {
        self.cirs_to_tirs(epoch)?.then(self.tirs_to_itrs(epoch)?)
    }

    fn itrs_to_cirs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Cirs, S>, Error> {
        self.cirs_to_itrs(epoch)?.inverse()
    }

    fn gcrs_to_itrs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Gcrs, Itrs, S>, Error> {
        self.gcrs_to_cirs(epoch)?
            .then(self.cirs_to_tirs(epoch)?)?
            .then(self.tirs_to_itrs(epoch)?)
    }

    fn itrs_to_gcrs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Gcrs, S>, Error> {
        self.gcrs_to_itrs(epoch)?.inverse()
    }

    fn earth_centered_transform<From, To, S>(
        epoch: Instant<S>,
        rotation: &KinematicRotation<From, To>,
    ) -> Result<StateTransform<From, To, S>, Error>
    where
        From: CoordinateFrame,
        To: CoordinateFrame,
        S: TimeScale,
    {
        let zero_length = Length::from_metres(0.0)?;
        let zero_speed = Speed::from_metres_per_second(0.0)?;
        Ok(StateTransform::new(
            epoch,
            rotation.rotation(),
            rotation.angular_velocity(),
            Vector3::new(zero_length, zero_length, zero_length),
            Vector3::new(zero_speed, zero_speed, zero_speed),
        ))
    }
}

impl<S: TimeScale> sealed::Sealed<Gcrs, Cirs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Gcrs, Cirs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Gcrs, Cirs, S>, Error> {
        self.gcrs_to_cirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Cirs, Gcrs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Cirs, Gcrs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Cirs, Gcrs, S>, Error> {
        self.cirs_to_gcrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Cirs, Tirs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Cirs, Tirs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Cirs, Tirs, S>, Error> {
        self.cirs_to_tirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Tirs, Cirs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Tirs, Cirs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Tirs, Cirs, S>, Error> {
        self.tirs_to_cirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Tirs, Itrs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Tirs, Itrs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Tirs, Itrs, S>, Error> {
        self.tirs_to_itrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Itrs, Tirs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Itrs, Tirs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Tirs, S>, Error> {
        self.itrs_to_tirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Gcrs, Tirs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Gcrs, Tirs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Gcrs, Tirs, S>, Error> {
        self.gcrs_to_tirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Tirs, Gcrs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Tirs, Gcrs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Tirs, Gcrs, S>, Error> {
        self.tirs_to_gcrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Cirs, Itrs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Cirs, Itrs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Cirs, Itrs, S>, Error> {
        self.cirs_to_itrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Itrs, Cirs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Itrs, Cirs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Cirs, S>, Error> {
        self.itrs_to_cirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Gcrs, Itrs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Gcrs, Itrs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Gcrs, Itrs, S>, Error> {
        self.gcrs_to_itrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Itrs, Gcrs, S> for Frames<'_, '_, '_> {}

impl<S: TimeScale> StateTransformModel<Itrs, Gcrs, S> for Frames<'_, '_, '_> {
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Gcrs, S>, Error> {
        self.itrs_to_gcrs(epoch)
    }
}
