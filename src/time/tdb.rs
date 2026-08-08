use super::{
    Duration, Error, Instant, JulianDate, Tdb, TimeContext, TimeScale, TimeScaleModel, Tt,
    context::sealed,
};

/// The SOFA Fairhead-Bretagnon analytical model for geocentric `TDB−TT`.
///
/// The model evaluates the full Fairhead & Bretagnon (1990) periodic series
/// with the topocentric term disabled. SOFA documents absolute accuracy better
/// than ±3 ns from 1950 through 2050 relative to numerical time ephemerides.
/// Outside that interval the model remains evaluable, but that accuracy bound
/// is not claimed. A later observer-state model can add the separate
/// topocentric contribution without changing this geocentric interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GeocentricTdb;

impl GeocentricTdb {
    /// Identifies the analytical model and upstream numerical implementation.
    pub const MODEL: &'static str = "Fairhead-Bretagnon 1990 via SOFA 2023-10-11";

    /// Constructs the stateless geocentric TDB model.
    pub const fn new() -> Self {
        Self
    }

    /// Evaluates geocentric TDB and `TDB−TT` at one physical epoch.
    pub fn at<S: TimeScale>(&self, epoch: Instant<S>) -> Result<TdbSolution<S>, Error> {
        let terrestrial_time = JulianDate::<Tt>::from_instant(epoch, &TimeContext::builtin())?;
        let (tt_first, tt_second) = terrestrial_time.parts();
        let offset_seconds = sofars::ts::dtdb(tt_first, tt_second, 0.0, 0.0, 0.0, 0.0);
        Error::ensure_finite("geocentric TDB−TT", offset_seconds)?;
        let tdb_minus_tt = Duration::from_seconds_f64(offset_seconds)?;
        let shifted = terrestrial_time.checked_add_duration(tdb_minus_tt)?;
        let (tdb_first, tdb_second) = shifted.parts();
        let barycentric_dynamical_time = JulianDate::<Tdb>::from_parts(tdb_first, tdb_second)?;

        Ok(TdbSolution {
            epoch,
            terrestrial_time,
            barycentric_dynamical_time,
            tdb_minus_tt,
        })
    }
}

impl sealed::Sealed for GeocentricTdb {}

impl TimeScaleModel<Tdb> for GeocentricTdb {
    fn validate_instant<From: TimeScale>(&self, instant: Instant<From>) -> Result<(), Error> {
        self.at(instant).map(|_| ())
    }

    fn julian_date_at<From: TimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<Tdb>, Error> {
        self.at(instant)
            .map(TdbSolution::barycentric_dynamical_time)
    }
}

/// One coherent geocentric TDB solution at a physical epoch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TdbSolution<S: TimeScale> {
    epoch: Instant<S>,
    terrestrial_time: JulianDate<Tt>,
    barycentric_dynamical_time: JulianDate<Tdb>,
    tdb_minus_tt: Duration,
}

impl<S: TimeScale> TdbSolution<S> {
    /// Returns the physical epoch used by the model.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the two-part TT Julian Date used as the analytical argument.
    pub const fn terrestrial_time(self) -> JulianDate<Tt> {
        self.terrestrial_time
    }

    /// Returns the resulting two-part TDB Julian Date.
    pub const fn barycentric_dynamical_time(self) -> JulianDate<Tdb> {
        self.barycentric_dynamical_time
    }

    /// Returns the nanosecond-rounded periodic difference `TDB−TT`.
    pub const fn tdb_minus_tt(self) -> Duration {
        self.tdb_minus_tt
    }

    /// Returns `TDB−TT` in SI seconds.
    pub fn tdb_minus_tt_seconds(self) -> f64 {
        self.tdb_minus_tt.as_seconds_f64()
    }
}
