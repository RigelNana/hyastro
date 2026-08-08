use crate::time::{
    EarthAttitudeTable, EarthOrientationTable, Instant, JulianDate, TimeContext, TimeScale,
    TimeScaleModel, Tt, Ut1,
};

use super::{
    CelestialOrientationSolution, Cirs, CoordinateFrame, EarthAttitudeSolution,
    EarthOrientationSolution, Error, Gcrs, Itrs, SiderealTimeSolution, State, StateTransform, Tirs,
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

/// Typed astronomical frame and Earth-rotation algorithms backed by one time context.
#[derive(Debug, Clone, Copy)]
pub struct Frames<'context, 'leap, E> {
    time: &'context TimeContext<'leap, E>,
}

impl<'context, 'leap, E> Frames<'context, 'leap, E> {
    /// Constructs frame algorithms from a time context carrying model data.
    pub const fn new(time: &'context TimeContext<'leap, E>) -> Self {
        Self { time }
    }

    /// Returns the time context used by these algorithms.
    pub const fn time_context(self) -> &'context TimeContext<'leap, E> {
        self.time
    }

    /// Evaluates IAU 2006/2000A celestial orientation at one physical epoch.
    ///
    /// This calculation uses TT only and requires no Earth-rotation or
    /// Earth-orientation observations. The returned solution retains the
    /// physical epoch, its two-part TT date, and every derived rotation.
    pub fn celestial_orientation_at<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<CelestialOrientationSolution<S>, Error> {
        let terrestrial_time = JulianDate::<Tt>::from_instant(epoch, self.time)?;
        CelestialOrientationSolution::at(epoch, terrestrial_time)
    }

    /// Evaluates ERA and Greenwich or local sidereal time at one epoch.
    ///
    /// The context must provide UT1. An
    /// [`EarthRotationTable`](crate::time::EarthRotationTable) is sufficient;
    /// a complete [`EarthOrientationTable`] also satisfies the requirement.
    pub fn sidereal_time_at<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<SiderealTimeSolution<S>, Error>
    where
        TimeContext<'leap, E>: TimeScaleModel<Ut1>,
    {
        SiderealTimeSolution::at(epoch, self.time)
    }
}

impl<'context, 'leap, 'eop> Frames<'context, 'leap, EarthAttitudeTable<'eop>> {
    /// Evaluates one coherent observed Earth-attitude solution.
    ///
    /// The result can rotate directions through GCRS/CIRS/TIRS/ITRS without
    /// requiring length of day or frame-rate observations.
    pub fn earth_attitude_at<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<EarthAttitudeSolution<S>, Error> {
        EarthAttitudeSolution::at(epoch, self.time)
    }
}

impl<'context, 'leap, 'eop> Frames<'context, 'leap, EarthOrientationTable<'eop>> {
    /// Evaluates one coherent IAU 2006/2000A Earth-orientation solution.
    ///
    /// TT, UT1, and every EOP value are resolved from the same physical
    /// instant and retained in the returned snapshot.
    pub fn earth_orientation_at<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<EarthOrientationSolution<S>, Error> {
        EarthOrientationSolution::at(epoch, self.time)
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
        self.earth_orientation_at(epoch)?
            .gcrs_to_cirs_state_transform()
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
        self.earth_orientation_at(epoch)?
            .cirs_to_tirs_state_transform()
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
        self.earth_orientation_at(epoch)?
            .tirs_to_itrs_state_transform()
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
        let solution = self.earth_orientation_at(epoch)?;
        solution
            .gcrs_to_cirs_state_transform()?
            .then(solution.cirs_to_tirs_state_transform()?)
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
        let solution = self.earth_orientation_at(epoch)?;
        solution
            .cirs_to_tirs_state_transform()?
            .then(solution.tirs_to_itrs_state_transform()?)
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
        let solution = self.earth_orientation_at(epoch)?;
        solution
            .gcrs_to_cirs_state_transform()?
            .then(solution.cirs_to_tirs_state_transform()?)?
            .then(solution.tirs_to_itrs_state_transform()?)
    }

    fn itrs_to_gcrs<S: TimeScale>(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Gcrs, S>, Error> {
        self.gcrs_to_itrs(epoch)?.inverse()
    }
}

impl<S: TimeScale> sealed::Sealed<Gcrs, Cirs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Gcrs, Cirs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Gcrs, Cirs, S>, Error> {
        self.gcrs_to_cirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Cirs, Gcrs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Cirs, Gcrs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Cirs, Gcrs, S>, Error> {
        self.cirs_to_gcrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Cirs, Tirs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Cirs, Tirs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Cirs, Tirs, S>, Error> {
        self.cirs_to_tirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Tirs, Cirs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Tirs, Cirs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Tirs, Cirs, S>, Error> {
        self.tirs_to_cirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Tirs, Itrs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Tirs, Itrs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Tirs, Itrs, S>, Error> {
        self.tirs_to_itrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Itrs, Tirs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Itrs, Tirs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Tirs, S>, Error> {
        self.itrs_to_tirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Gcrs, Tirs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Gcrs, Tirs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Gcrs, Tirs, S>, Error> {
        self.gcrs_to_tirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Tirs, Gcrs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Tirs, Gcrs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Tirs, Gcrs, S>, Error> {
        self.tirs_to_gcrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Cirs, Itrs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Cirs, Itrs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Cirs, Itrs, S>, Error> {
        self.cirs_to_itrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Itrs, Cirs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Itrs, Cirs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Cirs, S>, Error> {
        self.itrs_to_cirs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Gcrs, Itrs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Gcrs, Itrs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Gcrs, Itrs, S>, Error> {
        self.gcrs_to_itrs(epoch)
    }
}

impl<S: TimeScale> sealed::Sealed<Itrs, Gcrs, S> for Frames<'_, '_, EarthOrientationTable<'_>> {}

impl<S: TimeScale> StateTransformModel<Itrs, Gcrs, S>
    for Frames<'_, '_, EarthOrientationTable<'_>>
{
    fn state_transform_at(
        &self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<Itrs, Gcrs, S>, Error> {
        self.itrs_to_gcrs(epoch)
    }
}
