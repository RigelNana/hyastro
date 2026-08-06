use hifitime::{
    Duration as HifitimeDuration, Epoch as HifitimeEpoch, TimeScale as HifitimeTimeScale,
};

use super::{
    Calendar, Date, DateTime, Duration, Error, Gps, Gregorian, Instant, JulianDate, Tai, Tcb, Tcg,
    Tdb, TimeOfDay, TimeScale, TimeScaleModel, Tt, UnixTimestamp, Utc, context::sealed,
};

/// A hyastro uniform time scale supported by the Hifitime adapter.
///
/// UTC is deliberately excluded: hyastro resolves UTC labels through
/// [`TimeContext`](super::TimeContext) and its explicit leap-second data.
pub trait HifitimeScale: TimeScale {
    /// Returns the corresponding Hifitime scale.
    #[doc(hidden)]
    fn hifitime_scale() -> HifitimeTimeScale;
}

impl HifitimeScale for Tai {
    fn hifitime_scale() -> HifitimeTimeScale {
        HifitimeTimeScale::TAI
    }
}

impl HifitimeScale for Tt {
    fn hifitime_scale() -> HifitimeTimeScale {
        HifitimeTimeScale::TT
    }
}

impl HifitimeScale for Tdb {
    fn hifitime_scale() -> HifitimeTimeScale {
        HifitimeTimeScale::TDB
    }
}

impl HifitimeScale for Tcg {
    fn hifitime_scale() -> HifitimeTimeScale {
        HifitimeTimeScale::TCG
    }
}

impl HifitimeScale for Tcb {
    fn hifitime_scale() -> HifitimeTimeScale {
        HifitimeTimeScale::TCB
    }
}

impl HifitimeScale for Gps {
    fn hifitime_scale() -> HifitimeTimeScale {
        HifitimeTimeScale::GPST
    }
}

/// Stateless adapter for Hifitime epochs and model-dependent time scales.
#[derive(Debug, Clone, Copy, Default)]
pub struct Hifitime;

impl Hifitime {
    /// Constructs the stateless adapter.
    pub const fn new() -> Self {
        Self
    }

    /// Imports a Hifitime epoch as a strongly typed physical instant.
    pub fn import<S: TimeScale>(self, epoch: HifitimeEpoch) -> Instant<S> {
        let tai = epoch.to_time_scale(HifitimeTimeScale::TAI);
        Instant::from_tai_nanoseconds(tai.duration.total_nanoseconds())
    }

    /// Exports a typed instant to a Hifitime epoch in the selected scale.
    pub fn export<S: HifitimeScale>(self, instant: Instant<S>) -> HifitimeEpoch {
        HifitimeEpoch::from_tai_duration(HifitimeDuration::from_total_nanoseconds(
            instant.tai_nanoseconds_since_1900(),
        ))
        .to_time_scale(S::hifitime_scale())
    }

    /// Resolves a uniform-scale calendar label using Hifitime's model.
    pub fn resolve<C: Calendar, S: HifitimeScale>(
        self,
        value: DateTime<C, S>,
    ) -> Result<Instant<S>, Error> {
        let scale = S::hifitime_scale();
        if matches!(scale, HifitimeTimeScale::TCG | HifitimeTimeScale::TCB) {
            let julian = JulianDate::<S>::from_datetime(value)?;
            let (first, second) = julian.parts();
            let epoch = HifitimeEpoch::from_jde_in_time_scale(first, scale)
                + HifitimeDuration::from_days(second);
            return Ok(self.import(epoch));
        }

        let date: Date<Gregorian> = value.date().convert()?;
        let time = value.time();
        let epoch = HifitimeEpoch::maybe_from_gregorian(
            date.year(),
            date.month(),
            date.day(),
            time.hour(),
            time.minute(),
            time.second(),
            time.nanosecond(),
            scale,
        )
        .map_err(|reason| Error::Hifitime {
            operation: "resolving calendar label",
            reason,
        })?;
        Ok(self.import(epoch))
    }

    /// Represents an instant as a uniform-scale calendar label using Hifitime.
    pub fn represent<C: Calendar, S: HifitimeScale>(
        self,
        instant: Instant<S>,
    ) -> Result<DateTime<C, S>, Error> {
        let scale = S::hifitime_scale();
        let epoch = self.export(instant);
        if let Some(julian) = Self::coordinate_julian_date(epoch, scale)? {
            return julian.to_datetime();
        }

        let (year, month, day, hour, minute, second, nanosecond) = epoch.to_gregorian(scale);
        let date = Date::<Gregorian>::new(year, month, day)?.convert()?;
        let time = TimeOfDay::from_backend_components(hour, minute, second, nanosecond)?;
        DateTime::new(date, time)
    }

    fn modeled_julian_date<From: TimeScale, S: HifitimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<S>, Error> {
        let target = instant.retag::<S>();
        let scale = S::hifitime_scale();
        let epoch = (*self).export(target);
        if let Some(julian) = Self::coordinate_julian_date(epoch, scale)? {
            return Ok(julian);
        }
        JulianDate::from_datetime((*self).represent::<Gregorian, S>(target)?)
    }

    fn coordinate_julian_date<S: TimeScale>(
        epoch: HifitimeEpoch,
        scale: HifitimeTimeScale,
    ) -> Result<Option<JulianDate<S>>, Error> {
        let reference_seconds = match scale {
            HifitimeTimeScale::TCG => 32.184,
            HifitimeTimeScale::TCB => 32.184_065_5,
            _ => return Ok(None),
        };
        const SECONDS_PER_DAY: f64 = 86_400.0;
        let scaled = epoch.to_time_scale(scale);
        let reference = JulianDate::from_parts(2_443_144.5, reference_seconds / SECONDS_PER_DAY)?;
        reference
            .checked_add_duration(Duration::from_nanoseconds(
                scaled.duration.total_nanoseconds(),
            ))
            .map(Some)
    }

    /// Resolves an exact POSIX timestamp through Hifitime's Unix mapping.
    pub fn resolve_unix(self, value: UnixTimestamp) -> Instant<Utc> {
        self.import(HifitimeEpoch::from_unix_duration(
            HifitimeDuration::from_total_nanoseconds(value.as_nanoseconds()),
        ))
    }

    /// Converts a physical instant to Hifitime's exact nominal Unix timestamp.
    pub fn unix_timestamp(self, instant: Instant<Utc>) -> UnixTimestamp {
        let epoch = HifitimeEpoch::from_tai_duration(HifitimeDuration::from_total_nanoseconds(
            instant.tai_nanoseconds_since_1900(),
        ))
        .to_time_scale(HifitimeTimeScale::UTC);
        let unix_epoch =
            HifitimeEpoch::from_unix_duration(HifitimeDuration::from_total_nanoseconds(0));
        UnixTimestamp::from_nanoseconds((epoch - unix_epoch).total_nanoseconds())
    }
}

impl sealed::Sealed for Hifitime {}

impl<S: HifitimeScale> TimeScaleModel<S> for Hifitime {
    fn validate_instant<From: TimeScale>(&self, instant: Instant<From>) -> Result<(), Error> {
        let _ = (*self).export(instant.retag::<S>());
        Ok(())
    }

    fn julian_date_at<From: TimeScale>(
        &self,
        instant: Instant<From>,
    ) -> Result<JulianDate<S>, Error> {
        self.modeled_julian_date(instant)
    }
}
