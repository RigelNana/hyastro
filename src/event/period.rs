use core::{f64::consts::TAU, fmt, marker::PhantomData};
use std::vec::Vec;

use libm::{asin, atan2, floor, sqrt};

use crate::{
    astro::MoonPhaseAngle,
    ephem::{CelestialBody, EphemerisProvenance, EphemerisProvider, EphemerisQuery, RelativeState},
    frame::{Bcrs, EclipticLatitude, EclipticLongitude},
    math::{Angle, Length, Speed},
    time::{Duration, Instant, JulianDate, JulianEpoch, TimeContext, TimeInterval, TimeScale, Tt},
};

use super::{
    AngularEventSearchOptions, Error, EventEvidence, Events, MoonPhaseAngleEvent, SolarTerm,
    search::BracketedRootSearch,
};

mod sealed {
    pub trait Sealed {}
}

/// Identifies one astronomical cycle without erasing its physical definition.
pub trait CycleKind: sealed::Sealed {
    /// Stable English name of the cycle.
    const NAME: &'static str;
}

macro_rules! cycle_kinds {
    ($(($name:ident, $label:literal)),+ $(,)?) => {
        $(
            #[doc = concat!("Type marker for a ", $label, ".")]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct $name;

            impl sealed::Sealed for $name {}

            impl CycleKind for $name {
                const NAME: &'static str = $label;
            }
        )+
    };
}

cycle_kinds!(
    (EquinoxYear, "equinox year"),
    (TropicalYear, "mean tropical year"),
    (SiderealYear, "sidereal year"),
    (AnomalisticYear, "anomalistic year"),
    (DraconicYear, "draconic year"),
    (SynodicMonth, "synodic month"),
    (SiderealMonth, "sidereal month"),
    (TropicalMonth, "tropical month"),
    (AnomalisticMonth, "anomalistic month"),
    (DraconicMonth, "draconic month"),
);

/// One of the two equinox crossings that can delimit an equinox year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquinoxKind {
    /// The northward apparent-Sun crossing at longitude 0°.
    March,
    /// The southward apparent-Sun crossing at longitude 180°.
    September,
}

impl EquinoxKind {
    const fn solar_term(self) -> SolarTerm {
        match self {
            Self::March => SolarTerm::SpringEquinox,
            Self::September => SolarTerm::AutumnEquinox,
        }
    }
}

/// One of the two directed crossings of the lunar orbit through the ecliptic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LunarNode {
    /// South-to-north ecliptic-latitude crossing.
    Ascending,
    /// North-to-south ecliptic-latitude crossing.
    Descending,
}

/// A typed residual retained by a cycle-boundary root search.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum CycleResidual {
    /// Signed angular residual from a longitude or latitude criterion.
    Angle(Angle),
    /// Signed radial-speed residual from a periapsis criterion.
    RadialSpeed(Speed),
}

/// Numerical convergence evidence for one physical cycle boundary.
pub struct CycleEvidence<S: TimeScale> {
    bracket_start: Instant<S>,
    bracket_end: Instant<S>,
    time_uncertainty: Duration,
    residual: CycleResidual,
    iterations: u32,
    evaluations: u32,
}

impl<S: TimeScale> CycleEvidence<S> {
    const fn new(
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        time_uncertainty: Duration,
        residual: CycleResidual,
        iterations: u32,
        evaluations: u32,
    ) -> Self {
        Self {
            bracket_start,
            bracket_end,
            time_uncertainty,
            residual,
            iterations,
            evaluations,
        }
    }

    fn from_angular(value: EventEvidence<S>) -> Self {
        Self::new(
            value.bracket_start(),
            value.bracket_end(),
            value.time_uncertainty(),
            CycleResidual::Angle(value.residual()),
            value.iterations(),
            value.evaluations(),
        )
    }

    /// Returns the final inclusive root bracket start.
    pub const fn bracket_start(self) -> Instant<S> {
        self.bracket_start
    }

    /// Returns the final inclusive root bracket end.
    pub const fn bracket_end(self) -> Instant<S> {
        self.bracket_end
    }

    /// Returns half the final bracket width.
    pub const fn time_uncertainty(self) -> Duration {
        self.time_uncertainty
    }

    /// Returns the final signed criterion residual.
    pub const fn residual(self) -> CycleResidual {
        self.residual
    }

    /// Returns the completed Brent iterations.
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns the cumulative ephemeris or astrometric evaluations at this boundary.
    pub const fn evaluations(self) -> u32 {
        self.evaluations
    }
}

impl<S: TimeScale> Copy for CycleEvidence<S> {}

impl<S: TimeScale> Clone for CycleEvidence<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for CycleEvidence<S> {
    fn eq(&self, other: &Self) -> bool {
        self.bracket_start == other.bracket_start
            && self.bracket_end == other.bracket_end
            && self.time_uncertainty == other.time_uncertainty
            && self.residual == other.residual
            && self.iterations == other.iterations
            && self.evaluations == other.evaluations
    }
}

impl<S: TimeScale> fmt::Debug for CycleEvidence<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CycleEvidence")
            .field("scale", &S::NAME)
            .field("bracket_start", &self.bracket_start)
            .field("bracket_end", &self.bracket_end)
            .field("time_uncertainty", &self.time_uncertainty)
            .field("residual", &self.residual)
            .field("iterations", &self.iterations)
            .field("evaluations", &self.evaluations)
            .finish()
    }
}

/// Physical state retained at one cycle boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum CycleEvent {
    /// An apparent geocentric solar-longitude crossing.
    ApparentSolarLongitude {
        /// Requested longitude.
        target: EclipticLongitude,
        /// Longitude at the refined event.
        actual: EclipticLongitude,
    },
    /// An apparent directed Moon-minus-Sun longitude crossing.
    ApparentLunarElongation {
        /// Requested directed phase-cycle angle.
        target: MoonPhaseAngle,
        /// Directed phase-cycle angle at the refined event.
        actual: MoonPhaseAngle,
    },
    /// A geometric body longitude on fixed IAU 2006 mean J2000 ecliptic axes.
    FixedEclipticLongitude {
        /// Body whose relative state supplied the direction.
        body: CelestialBody,
        /// Centre of the relative state.
        center: CelestialBody,
        /// Requested fixed reference longitude.
        target: EclipticLongitude,
        /// Longitude at the refined event.
        actual: EclipticLongitude,
    },
    /// A geometric body longitude on IAU 2006 mean ecliptic and equinox of date axes.
    MeanOfDateEclipticLongitude {
        /// Body whose relative state supplied the direction.
        body: CelestialBody,
        /// Centre of the relative state.
        center: CelestialBody,
        /// Requested moving reference longitude.
        target: EclipticLongitude,
        /// Longitude at the refined event.
        actual: EclipticLongitude,
    },
    /// A geometric periapsis of one target relative to one centre.
    Periapsis {
        /// Body passing periapsis.
        body: CelestialBody,
        /// Centre about which the distance is minimized.
        center: CelestialBody,
        /// Geometric centre-to-centre distance at the event.
        distance: Length,
        /// Signed radial speed at the refined event.
        radial_speed: Speed,
    },
    /// A directed lunar ecliptic-node crossing.
    LunarEclipticNode {
        /// Selected crossing direction.
        node: LunarNode,
        /// Mean-of-date ecliptic longitude of the Moon.
        longitude: EclipticLongitude,
        /// Mean-of-date ecliptic latitude at the refined event.
        latitude: EclipticLatitude,
    },
    /// A geocentric Sun crossing of the instantaneous osculating lunar node.
    SunAtLunarNode {
        /// Selected lunar node direction.
        node: LunarNode,
        /// Mean-of-date longitude of the instantaneous lunar node.
        node_longitude: EclipticLongitude,
        /// Mean-of-date geometric geocentric solar longitude.
        sun_longitude: EclipticLongitude,
        /// Directed solar-minus-node longitude at the refined event.
        relative_longitude: Angle,
    },
}

/// One refined physical event that can delimit a measured cycle.
pub struct CycleBoundary<S: TimeScale> {
    instant: Instant<S>,
    event: CycleEvent,
    evidence: CycleEvidence<S>,
}

impl<S: TimeScale> CycleBoundary<S> {
    const fn new(instant: Instant<S>, event: CycleEvent, evidence: CycleEvidence<S>) -> Self {
        Self {
            instant,
            event,
            evidence,
        }
    }

    /// Returns the physical event instant.
    pub const fn instant(self) -> Instant<S> {
        self.instant
    }

    /// Returns the physical criterion state at the event.
    pub const fn event(self) -> CycleEvent {
        self.event
    }

    /// Returns the numerical root-search evidence.
    pub const fn evidence(self) -> CycleEvidence<S> {
        self.evidence
    }

    fn periapsis_distance(self) -> Option<Length> {
        match self.event {
            CycleEvent::Periapsis { distance, .. } => Some(distance),
            _ => None,
        }
    }
}

impl<S: TimeScale> Copy for CycleBoundary<S> {}

impl<S: TimeScale> Clone for CycleBoundary<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for CycleBoundary<S> {
    fn eq(&self, other: &Self) -> bool {
        self.instant == other.instant
            && self.event == other.event
            && self.evidence == other.evidence
    }
}

impl<S: TimeScale> fmt::Debug for CycleBoundary<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CycleBoundary")
            .field("scale", &S::NAME)
            .field("instant", &self.instant)
            .field("event", &self.event)
            .field("evidence", &self.evidence)
            .finish()
    }
}

/// Reproducibility metadata for an event-measured astronomical cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleModel {
    criterion: &'static str,
    reference_axes: &'static str,
    ephemeris: EphemerisProvenance,
}

impl CycleModel {
    fn new(
        criterion: &'static str,
        reference_axes: &'static str,
        ephemeris: EphemerisProvenance,
    ) -> Self {
        Self {
            criterion,
            reference_axes,
            ephemeris,
        }
    }

    /// Returns the stable physical event criterion description.
    pub const fn criterion(&self) -> &'static str {
        self.criterion
    }

    /// Returns the reference axes and equinox convention.
    pub const fn reference_axes(&self) -> &'static str {
        self.reference_axes
    }

    /// Returns the exact model and data provenance used by the measurement.
    pub const fn provenance(&self) -> &EphemerisProvenance {
        &self.ephemeris
    }
}

/// One astronomical cycle measured between adjacent same-kind physical events.
pub struct MeasuredCycle<K: CycleKind, S: TimeScale> {
    start: CycleBoundary<S>,
    end: CycleBoundary<S>,
    duration: Duration,
    model: CycleModel,
    kind: PhantomData<K>,
}

impl<K: CycleKind, S: TimeScale> MeasuredCycle<K, S> {
    fn new(
        start: CycleBoundary<S>,
        end: CycleBoundary<S>,
        model: CycleModel,
    ) -> Result<Self, Error> {
        let duration = end.instant().duration_since(start.instant())?;
        Ok(Self {
            start,
            end,
            duration,
            model,
            kind: PhantomData,
        })
    }

    /// Returns the stable cycle-kind name.
    pub const fn kind_name(&self) -> &'static str {
        K::NAME
    }

    /// Returns the first physical boundary event.
    pub const fn start(&self) -> CycleBoundary<S> {
        self.start
    }

    /// Returns the adjacent same-kind physical boundary event.
    pub const fn end(&self) -> CycleBoundary<S> {
        self.end
    }

    /// Returns the actual physical interval between the boundaries.
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns the criterion, axes, and exact ephemeris provenance.
    pub const fn model(&self) -> &CycleModel {
        &self.model
    }
}

impl<K: CycleKind, S: TimeScale> Clone for MeasuredCycle<K, S> {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            end: self.end,
            duration: self.duration,
            model: self.model.clone(),
            kind: PhantomData,
        }
    }
}

impl<K: CycleKind, S: TimeScale> PartialEq for MeasuredCycle<K, S> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start
            && self.end == other.end
            && self.duration == other.duration
            && self.model == other.model
    }
}

impl<K: CycleKind, S: TimeScale> fmt::Debug for MeasuredCycle<K, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MeasuredCycle")
            .field("kind", &K::NAME)
            .field("scale", &S::NAME)
            .field("start", &self.start)
            .field("end", &self.end)
            .field("duration", &self.duration)
            .field("model", &self.model)
            .finish()
    }
}

/// Closed recommended epoch range for a numerical mean-cycle model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelValidity {
    start: JulianEpoch,
    end: JulianEpoch,
}

impl ModelValidity {
    const fn new(start: JulianEpoch, end: JulianEpoch) -> Self {
        Self { start, end }
    }

    /// Returns the inclusive first recommended Julian epoch.
    pub const fn start(self) -> JulianEpoch {
        self.start
    }

    /// Returns the inclusive last recommended Julian epoch.
    pub const fn end(self) -> JulianEpoch {
        self.end
    }

    /// Reports whether a Julian epoch is within the recommended range.
    pub fn contains(self, epoch: JulianEpoch) -> bool {
        self.start <= epoch && epoch <= self.end
    }
}

/// A local mean cycle obtained from an explicit numerical model rather than adjacent events.
pub struct ModeledCycle<K: CycleKind, S: TimeScale> {
    evaluation_epoch: Instant<S>,
    duration: Duration,
    model_identifier: &'static str,
    validity: ModelValidity,
    kind: PhantomData<K>,
}

impl<K: CycleKind, S: TimeScale> ModeledCycle<K, S> {
    /// Returns the stable cycle-kind name.
    pub const fn kind_name(&self) -> &'static str {
        K::NAME
    }

    /// Returns the physical epoch at which the local mean was evaluated.
    pub const fn evaluation_epoch(self) -> Instant<S> {
        self.evaluation_epoch
    }

    /// Returns the modeled local mean duration.
    pub const fn duration(self) -> Duration {
        self.duration
    }

    /// Returns the stable numerical-model identifier.
    pub const fn model_identifier(self) -> &'static str {
        self.model_identifier
    }

    /// Returns the model's closed recommended Julian-epoch range.
    pub const fn validity(self) -> ModelValidity {
        self.validity
    }
}

impl<K: CycleKind, S: TimeScale> Copy for ModeledCycle<K, S> {}

impl<K: CycleKind, S: TimeScale> Clone for ModeledCycle<K, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: CycleKind, S: TimeScale> PartialEq for ModeledCycle<K, S> {
    fn eq(&self, other: &Self) -> bool {
        self.evaluation_epoch == other.evaluation_epoch
            && self.duration == other.duration
            && self.model_identifier == other.model_identifier
            && self.validity == other.validity
    }
}

impl<K: CycleKind, S: TimeScale> fmt::Debug for ModeledCycle<K, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModeledCycle")
            .field("kind", &K::NAME)
            .field("scale", &S::NAME)
            .field("evaluation_epoch", &self.evaluation_epoch)
            .field("duration", &self.duration)
            .field("model_identifier", &self.model_identifier)
            .field("validity", &self.validity)
            .finish()
    }
}

impl<S: TimeScale> ModeledCycle<TropicalYear, S> {
    /// Evaluates the local mean tropical year from the derivative of the Meeus J2000 mean-Sun
    /// longitude polynomial.
    ///
    /// The model is referred to the mean ecliptic and equinox of date. Its conservative
    /// recommended range is J500.0 through J3500.0; epochs outside that interval are rejected
    /// rather than silently extrapolated. This is a mean model, not an interval between observed
    /// equinoxes.
    pub fn from_meeus_mean_solar_longitude<E>(
        evaluation_epoch: Instant<S>,
        time: &TimeContext<'_, E>,
    ) -> Result<Self, Error> {
        const MODEL: &str = "Meeus J2000 geometric mean solar longitude derivative";
        let validity = ModelValidity::new(JulianEpoch::new(500.0)?, JulianEpoch::new(3500.0)?);
        let terrestrial_time = JulianDate::<Tt>::from_instant(evaluation_epoch, time)?;
        let julian_epoch = JulianEpoch::from_tt(terrestrial_time)?;
        if !validity.contains(julian_epoch) {
            return Err(Error::ModelEpochOutsideValidity {
                model: MODEL,
                epoch: julian_epoch.value(),
                start: validity.start().value(),
                end: validity.end().value(),
            });
        }

        let centuries =
            (terrestrial_time.as_f64_lossy() - JulianDate::<Tt>::J2000_VALUE) / 36_525.0;
        let degrees_per_century = 36_000.769_83 + 0.000_606_4 * centuries;
        let days = 360.0 * 36_525.0 / degrees_per_century;
        let duration = Duration::from_seconds_f64(days * 86_400.0)?;
        Ok(Self {
            evaluation_epoch,
            duration,
            model_identifier: MODEL,
            validity,
            kind: PhantomData,
        })
    }
}

/// Descriptive statistics over complete measured cycles of one kind.
pub struct CycleStatistics<K: CycleKind> {
    count: usize,
    minimum: Duration,
    maximum: Duration,
    mean: Duration,
    standard_deviation: Duration,
    kind: PhantomData<K>,
}

impl<K: CycleKind> CycleStatistics<K> {
    /// Computes population statistics from one or more complete measured cycles.
    pub fn from_cycles<S: TimeScale>(cycles: &[MeasuredCycle<K, S>]) -> Result<Self, Error> {
        let first = cycles.first().ok_or(Error::EmptyCycleSample)?.duration();
        let mut minimum = first;
        let mut maximum = first;
        let mut sum_seconds = 0.0;
        for cycle in cycles {
            let duration = cycle.duration();
            minimum = minimum.min(duration);
            maximum = maximum.max(duration);
            sum_seconds += duration.as_seconds_f64();
        }
        let count = cycles.len();
        let mean_seconds = sum_seconds / count as f64;
        let mut squared_deviation_sum = 0.0;
        for cycle in cycles {
            let deviation = cycle.duration().as_seconds_f64() - mean_seconds;
            squared_deviation_sum += deviation * deviation;
        }
        Ok(Self {
            count,
            minimum,
            maximum,
            mean: Duration::from_seconds_f64(mean_seconds)?,
            standard_deviation: Duration::from_seconds_f64(sqrt(
                squared_deviation_sum / count as f64,
            ))?,
            kind: PhantomData,
        })
    }

    /// Returns the number of complete cycles in the sample.
    pub const fn count(&self) -> usize {
        self.count
    }

    /// Returns the shortest measured cycle.
    pub const fn minimum(&self) -> Duration {
        self.minimum
    }

    /// Returns the longest measured cycle.
    pub const fn maximum(&self) -> Duration {
        self.maximum
    }

    /// Returns the arithmetic mean duration.
    pub const fn mean(&self) -> Duration {
        self.mean
    }

    /// Returns the population standard deviation.
    pub const fn standard_deviation(&self) -> Duration {
        self.standard_deviation
    }
}

impl<K: CycleKind> Clone for CycleStatistics<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: CycleKind> Copy for CycleStatistics<K> {}

impl<K: CycleKind> PartialEq for CycleStatistics<K> {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count
            && self.minimum == other.minimum
            && self.maximum == other.maximum
            && self.mean == other.mean
            && self.standard_deviation == other.standard_deviation
    }
}

impl<K: CycleKind> fmt::Debug for CycleStatistics<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CycleStatistics")
            .field("kind", &K::NAME)
            .field("count", &self.count)
            .field("minimum", &self.minimum)
            .field("maximum", &self.maximum)
            .field("mean", &self.mean)
            .field("standard_deviation", &self.standard_deviation)
            .finish()
    }
}

#[derive(Clone, Copy)]
struct AngularSample {
    wrapped: f64,
    event: CycleEvent,
}

#[derive(Clone, Copy)]
struct ScalarSample {
    residual: f64,
    event: CycleEvent,
    residual_kind: ScalarResidualKind,
}

#[derive(Clone, Copy)]
enum ScalarResidualKind {
    Angle,
    RadialSpeed,
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Events<'context, 'data, E, P> {
    /// Measures complete intervals between adjacent occurrences of one selected equinox.
    pub fn equinox_years_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        equinox: EquinoxKind,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MeasuredCycle<EquinoxYear, S>>, Error> {
        let target = equinox.solar_term().target_longitude();
        let boundaries = CycleBoundarySearch::new(interval, options).increasing_angle(
            target.as_radians(),
            |epoch, evaluations| {
                Self::consume_evaluation(options, evaluations)?;
                let apparent_sun = self
                    .astrometry
                    .solar_apparent_place(epoch, options.light_time())?;
                let actual = apparent_sun.longitude();
                Ok(AngularSample {
                    wrapped: actual.as_radians(),
                    event: CycleEvent::ApparentSolarLongitude { target, actual },
                })
            },
        )?;
        self.cycles_from_boundaries(
            &boundaries,
            "adjacent same apparent geocentric equinox crossings",
            "IAU 2006/2000A true ecliptic and equinox of date",
        )
    }

    /// Measures complete synodic months between adjacent occurrences of one directed phase angle.
    pub fn synodic_months_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        phase: MoonPhaseAngle,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MeasuredCycle<SynodicMonth, S>>, Error> {
        let boundaries = self
            .moon_phase_angle_in(interval, phase, options)?
            .into_iter()
            .map(Self::moon_phase_boundary)
            .collect::<Vec<_>>();
        self.cycles_from_boundaries(
            &boundaries,
            "adjacent same directed apparent Moon-minus-Sun longitude crossings",
            "IAU 2006/2000A true ecliptic and equinox of date",
        )
    }

    /// Measures complete sidereal years from heliocentric Earth returns to a fixed longitude.
    pub fn sidereal_years_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        reference: EclipticLongitude,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MeasuredCycle<SiderealYear, S>>, Error> {
        let boundaries = self.increasing_longitude_boundaries(
            interval,
            reference,
            options,
            CelestialBody::Earth,
            CelestialBody::Sun,
            EclipticConvention::FixedJ2000,
        )?;
        self.cycles_from_boundaries(
            &boundaries,
            "heliocentric geometric Earth longitude returns to one fixed inertial direction",
            "IAU 2006 mean ecliptic and equinox J2000.0",
        )
    }

    /// Measures complete anomalistic years between adjacent geometric Earth perihelia.
    pub fn anomalistic_years_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MeasuredCycle<AnomalisticYear, S>>, Error> {
        let boundaries = self.periapsis_boundaries(
            interval,
            options,
            CelestialBody::Earth,
            CelestialBody::Sun,
            Some(Duration::from_days(90)?),
        )?;
        self.cycles_from_boundaries(
            &boundaries,
            "adjacent heliocentric Earth distance minima after resolving lunar-scale substructure",
            "geometric BCRS-aligned J2000 axes",
        )
    }

    /// Measures complete draconic years from successive Sun crossings of one osculating lunar node.
    pub fn draconic_years_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        node: LunarNode,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MeasuredCycle<DraconicYear, S>>, Error> {
        let boundaries = self.draconic_year_boundaries(interval, node, options)?;
        self.cycles_from_boundaries(
            &boundaries,
            "geocentric geometric Sun returns to the selected instantaneous lunar osculating node",
            "IAU 2006 mean ecliptic and equinox of date",
        )
    }

    /// Measures complete sidereal months from geocentric Moon returns to a fixed longitude.
    pub fn sidereal_months_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        reference: EclipticLongitude,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MeasuredCycle<SiderealMonth, S>>, Error> {
        let boundaries = self.increasing_longitude_boundaries(
            interval,
            reference,
            options,
            CelestialBody::Moon,
            CelestialBody::Earth,
            EclipticConvention::FixedJ2000,
        )?;
        self.cycles_from_boundaries(
            &boundaries,
            "geocentric geometric Moon longitude returns to one fixed inertial direction",
            "IAU 2006 mean ecliptic and equinox J2000.0",
        )
    }

    /// Measures complete tropical months from geocentric Moon returns to a mean-of-date longitude.
    pub fn tropical_months_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        reference: EclipticLongitude,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MeasuredCycle<TropicalMonth, S>>, Error> {
        let boundaries = self.increasing_longitude_boundaries(
            interval,
            reference,
            options,
            CelestialBody::Moon,
            CelestialBody::Earth,
            EclipticConvention::MeanOfDate,
        )?;
        self.cycles_from_boundaries(
            &boundaries,
            "geocentric geometric Moon longitude returns to one moving mean equinox direction",
            "IAU 2006 mean ecliptic and equinox of date",
        )
    }

    /// Measures complete anomalistic months between adjacent geometric lunar perigees.
    pub fn anomalistic_months_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MeasuredCycle<AnomalisticMonth, S>>, Error> {
        let boundaries = self.periapsis_boundaries(
            interval,
            options,
            CelestialBody::Moon,
            CelestialBody::Earth,
            None,
        )?;
        self.cycles_from_boundaries(
            &boundaries,
            "adjacent negative-to-positive geocentric lunar radial-speed roots",
            "geometric BCRS-aligned J2000 axes",
        )
    }

    /// Measures complete draconic months between adjacent same-direction lunar node crossings.
    pub fn draconic_months_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        node: LunarNode,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MeasuredCycle<DraconicMonth, S>>, Error> {
        let boundaries = self.lunar_node_boundaries(interval, node, options)?;
        self.cycles_from_boundaries(
            &boundaries,
            "adjacent same-direction geometric lunar ecliptic-latitude roots",
            "IAU 2006 mean ecliptic and equinox of date",
        )
    }

    fn cycles_from_boundaries<K: CycleKind, S: TimeScale>(
        &self,
        boundaries: &[CycleBoundary<S>],
        criterion: &'static str,
        reference_axes: &'static str,
    ) -> Result<Vec<MeasuredCycle<K, S>>, Error> {
        let model = CycleModel::new(
            criterion,
            reference_axes,
            self.astrometry
                .ephemeris()
                .provenance()
                .map_err(crate::astro::Error::from)?,
        );
        let mut cycles = Vec::with_capacity(boundaries.len().saturating_sub(1));
        for pair in boundaries.windows(2) {
            cycles.push(MeasuredCycle::new(pair[0], pair[1], model.clone())?);
        }
        Ok(cycles)
    }

    fn moon_phase_boundary<S: TimeScale>(event: MoonPhaseAngleEvent<S>) -> CycleBoundary<S> {
        CycleBoundary::new(
            event.instant(),
            CycleEvent::ApparentLunarElongation {
                target: event.target(),
                actual: event.longitude_difference(),
            },
            CycleEvidence::from_angular(event.evidence()),
        )
    }

    fn increasing_longitude_boundaries<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        reference: EclipticLongitude,
        options: AngularEventSearchOptions,
        body: CelestialBody,
        center: CelestialBody,
        convention: EclipticConvention,
    ) -> Result<Vec<CycleBoundary<S>>, Error> {
        CycleBoundarySearch::new(interval, options).increasing_angle(
            reference.as_radians(),
            |epoch, evaluations| {
                let state = self.geometric_state(body, center, epoch, options, evaluations)?;
                let coordinates = self.ecliptic_state(state, convention)?;
                let actual = EclipticLongitude::wrap_radians(coordinates.longitude)?;
                let event = match convention {
                    EclipticConvention::FixedJ2000 => CycleEvent::FixedEclipticLongitude {
                        body,
                        center,
                        target: reference,
                        actual,
                    },
                    EclipticConvention::MeanOfDate => CycleEvent::MeanOfDateEclipticLongitude {
                        body,
                        center,
                        target: reference,
                        actual,
                    },
                };
                Ok(AngularSample {
                    wrapped: coordinates.longitude,
                    event,
                })
            },
        )
    }

    fn periapsis_boundaries<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        options: AngularEventSearchOptions,
        body: CelestialBody,
        center: CelestialBody,
        cluster_gap: Option<Duration>,
    ) -> Result<Vec<CycleBoundary<S>>, Error> {
        let candidates = CycleBoundarySearch::new(interval, options).scalar(
            ScalarCrossing::NegativeToPositive,
            |epoch, evaluations| {
                let state = self.geometric_state(body, center, epoch, options, evaluations)?;
                let position = state.position();
                let velocity = state.velocity();
                let distance = position.magnitude()?;
                let [x, y, z] = position.components().map(Length::as_metres);
                let [vx, vy, vz] = velocity.components().map(Speed::as_metres_per_second);
                let radial_speed = (x * vx + y * vy + z * vz) / distance.as_metres();
                let radial_speed = Speed::from_metres_per_second(radial_speed)?;
                Ok(ScalarSample {
                    residual: radial_speed.as_metres_per_second(),
                    event: CycleEvent::Periapsis {
                        body,
                        center,
                        distance,
                        radial_speed,
                    },
                    residual_kind: ScalarResidualKind::RadialSpeed,
                })
            },
        )?;
        let Some(maximum_cluster_gap) = cluster_gap else {
            return Ok(candidates);
        };
        let mut selected: Vec<CycleBoundary<S>> = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if let Some(previous) = selected.last_mut()
                && candidate.instant().duration_since(previous.instant())? <= maximum_cluster_gap
            {
                if let (Some(candidate_distance), Some(previous_distance)) = (
                    candidate.periapsis_distance(),
                    previous.periapsis_distance(),
                ) && candidate_distance < previous_distance
                {
                    *previous = candidate;
                }
                continue;
            }
            selected.push(candidate);
        }
        Ok(selected)
    }

    fn lunar_node_boundaries<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        node: LunarNode,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<CycleBoundary<S>>, Error> {
        let crossing = match node {
            LunarNode::Ascending => ScalarCrossing::NegativeToPositive,
            LunarNode::Descending => ScalarCrossing::PositiveToNegative,
        };
        CycleBoundarySearch::new(interval, options).scalar(crossing, |epoch, evaluations| {
            let state = self.geometric_state(
                CelestialBody::Moon,
                CelestialBody::Earth,
                epoch,
                options,
                evaluations,
            )?;
            let coordinates = self.ecliptic_state(state, EclipticConvention::MeanOfDate)?;
            let longitude = EclipticLongitude::wrap_radians(coordinates.longitude)?;
            let latitude = EclipticLatitude::try_from_radians(coordinates.latitude)?;
            Ok(ScalarSample {
                residual: coordinates.latitude,
                event: CycleEvent::LunarEclipticNode {
                    node,
                    longitude,
                    latitude,
                },
                residual_kind: ScalarResidualKind::Angle,
            })
        })
    }

    fn draconic_year_boundaries<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        node: LunarNode,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<CycleBoundary<S>>, Error> {
        CycleBoundarySearch::new(interval, options).increasing_angle(0.0, |epoch, evaluations| {
            let moon = self.geometric_state(
                CelestialBody::Moon,
                CelestialBody::Earth,
                epoch,
                options,
                evaluations,
            )?;
            let sun = self.geometric_state(
                CelestialBody::Sun,
                CelestialBody::Earth,
                epoch,
                options,
                evaluations,
            )?;
            let moon = self.ecliptic_state(moon, EclipticConvention::MeanOfDate)?;
            let sun = self.ecliptic_state(sun, EclipticConvention::MeanOfDate)?;
            let node_longitude_value = moon.lunar_node_longitude(node)?;
            let relative = Angle::wrap_zero_tau(
                sun.longitude - node_longitude_value,
                "solar longitude relative to lunar node",
            )?;
            let node_longitude = EclipticLongitude::wrap_radians(node_longitude_value)?;
            let sun_longitude = EclipticLongitude::wrap_radians(sun.longitude)?;
            Ok(AngularSample {
                wrapped: relative,
                event: CycleEvent::SunAtLunarNode {
                    node,
                    node_longitude,
                    sun_longitude,
                    relative_longitude: Angle::from_radians(relative)?,
                },
            })
        })
    }

    fn geometric_state<S: TimeScale>(
        &self,
        body: CelestialBody,
        center: CelestialBody,
        epoch: Instant<S>,
        options: AngularEventSearchOptions,
        evaluations: &mut u32,
    ) -> Result<RelativeState<Bcrs, S>, Error> {
        Self::consume_evaluation(options, evaluations)?;
        self.astrometry
            .ephemeris()
            .state(EphemerisQuery::new(body, center, epoch))
            .map_err(crate::astro::Error::from)
            .map_err(Error::from)
    }

    fn consume_evaluation(
        options: AngularEventSearchOptions,
        evaluations: &mut u32,
    ) -> Result<(), Error> {
        if *evaluations >= options.max_evaluations() {
            return Err(Error::EvaluationLimitExceeded {
                maximum: options.max_evaluations(),
            });
        }
        *evaluations += 1;
        Ok(())
    }

    fn ecliptic_state<S: TimeScale>(
        &self,
        state: RelativeState<Bcrs, S>,
        convention: EclipticConvention,
    ) -> Result<EclipticState, Error> {
        let (date1, date2) = match convention {
            EclipticConvention::FixedJ2000 => (JulianDate::<Tt>::J2000_VALUE, 0.0),
            EclipticConvention::MeanOfDate => {
                JulianDate::<Tt>::from_instant(state.epoch(), self.astrometry.time_context())?
                    .parts()
            }
        };
        let matrix = sofars::coords::ecm06(date1, date2);
        let position = state.position().components().map(Length::as_metres);
        let velocity = state
            .velocity()
            .components()
            .map(Speed::as_metres_per_second);
        let mut ecliptic_position = [0.0; 3];
        let mut ecliptic_velocity = [0.0; 3];
        sofars::vm::rxp(&matrix, &position, &mut ecliptic_position);
        sofars::vm::rxp(&matrix, &velocity, &mut ecliptic_velocity);
        EclipticState::new(ecliptic_position, ecliptic_velocity)
    }
}

#[derive(Clone, Copy)]
enum EclipticConvention {
    FixedJ2000,
    MeanOfDate,
}

struct EclipticState {
    position: [f64; 3],
    velocity: [f64; 3],
    longitude: f64,
    latitude: f64,
}

impl EclipticState {
    fn new(position: [f64; 3], velocity: [f64; 3]) -> Result<Self, Error> {
        let radius = Self::magnitude(position);
        let longitude = Self::longitude(position)?;
        let latitude = asin((position[2] / radius).clamp(-1.0, 1.0));
        Ok(Self {
            position,
            velocity,
            longitude,
            latitude,
        })
    }

    fn lunar_node_longitude(&self, node: LunarNode) -> Result<f64, Error> {
        let angular_momentum = Self::cross(self.position, self.velocity);
        let mut node_vector = [-angular_momentum[1], angular_momentum[0], 0.0];
        if node == LunarNode::Descending {
            node_vector = node_vector.map(|value| -value);
        }
        Self::longitude(node_vector)
    }

    fn magnitude(vector: [f64; 3]) -> f64 {
        sqrt(vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2])
    }

    fn longitude(vector: [f64; 3]) -> Result<f64, Error> {
        let horizontal = sqrt(vector[0] * vector[0] + vector[1] * vector[1]);
        if horizontal == 0.0 {
            return Err(crate::math::Error::UndefinedLongitude.into());
        }
        Angle::wrap_zero_tau(atan2(vector[1], vector[0]), "ecliptic longitude").map_err(Error::from)
    }

    fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
        [
            left[1] * right[2] - left[2] * right[1],
            left[2] * right[0] - left[0] * right[2],
            left[0] * right[1] - left[1] * right[0],
        ]
    }
}

impl ScalarSample {
    fn is_root(self, angular_tolerance: Angle) -> bool {
        match self.residual_kind {
            ScalarResidualKind::Angle => self.residual.abs() <= angular_tolerance.as_radians(),
            ScalarResidualKind::RadialSpeed => self.residual == 0.0,
        }
    }

    fn cycle_residual(self) -> Result<CycleResidual, Error> {
        Ok(match self.residual_kind {
            ScalarResidualKind::Angle => CycleResidual::Angle(Angle::from_radians(self.residual)?),
            ScalarResidualKind::RadialSpeed => {
                CycleResidual::RadialSpeed(Speed::from_metres_per_second(self.residual)?)
            }
        })
    }
}

#[derive(Clone, Copy)]
enum ScalarCrossing {
    NegativeToPositive,
    PositiveToNegative,
}

impl ScalarCrossing {
    fn crossed(self, previous: f64, current: f64) -> bool {
        match self {
            Self::NegativeToPositive => previous < 0.0 && current >= 0.0,
            Self::PositiveToNegative => previous > 0.0 && current <= 0.0,
        }
    }
}

struct CycleBoundarySearch<S: TimeScale> {
    interval: TimeInterval<S>,
    options: AngularEventSearchOptions,
    evaluations: u32,
}

impl<S: TimeScale> CycleBoundarySearch<S> {
    const fn new(interval: TimeInterval<S>, options: AngularEventSearchOptions) -> Self {
        Self {
            interval,
            options,
            evaluations: 0,
        }
    }

    fn increasing_angle<F>(
        mut self,
        target: f64,
        mut evaluate: F,
    ) -> Result<Vec<CycleBoundary<S>>, Error>
    where
        F: FnMut(Instant<S>, &mut u32) -> Result<AngularSample, Error>,
    {
        let mut previous_epoch = self.interval.start();
        let initial = evaluate(previous_epoch, &mut self.evaluations)?;
        let mut previous_wrapped = initial.wrapped;
        let mut previous_unwrapped = initial.wrapped;
        let mut events = Vec::new();
        let initial_residual =
            Angle::wrap_signed(initial.wrapped - target, "cycle angle residual")?;
        if initial_residual.abs() <= self.options.angular_tolerance().as_radians() {
            events.push(CycleBoundary::new(
                previous_epoch,
                initial.event,
                CycleEvidence::new(
                    previous_epoch,
                    previous_epoch,
                    Duration::ZERO,
                    CycleResidual::Angle(Angle::from_radians(initial_residual)?),
                    0,
                    self.evaluations,
                ),
            ));
        }

        while previous_epoch < self.interval.end() {
            let remaining = self.interval.end().duration_since(previous_epoch)?;
            let step = remaining.min(self.options.scan_step());
            let current_epoch = previous_epoch.checked_add(step)?;
            let current = evaluate(current_epoch, &mut self.evaluations)?;
            let advance =
                Angle::wrap_signed(current.wrapped - previous_wrapped, "cycle angular advance")?;
            if advance <= 0.0 {
                return Err(Error::CycleAngleNotIncreasing {
                    previous_radians: previous_wrapped,
                    current_radians: current.wrapped,
                    previous_tai_nanoseconds: previous_epoch.tai_nanoseconds_since_1900(),
                    current_tai_nanoseconds: current_epoch.tai_nanoseconds_since_1900(),
                });
            }
            let current_unwrapped = previous_unwrapped + advance;
            let cycle_index = floor((previous_unwrapped - target) / TAU) as i64 + 1;
            let boundary = target + cycle_index as f64 * TAU;
            if boundary <= current_unwrapped + self.options.angular_tolerance().as_radians()
                && boundary > previous_unwrapped + self.options.angular_tolerance().as_radians()
            {
                let root = BracketedRootSearch::refine(
                    previous_epoch,
                    current_epoch,
                    self.options.time_tolerance(),
                    self.options.max_refinement_iterations(),
                    |epoch| {
                        evaluate(epoch, &mut self.evaluations).and_then(|sample| {
                            Angle::wrap_signed(sample.wrapped - target, "cycle angle residual")
                                .map_err(Error::from)
                        })
                    },
                )?;
                let sample = evaluate(root.instant(), &mut self.evaluations)?;
                let residual = Angle::wrap_signed(sample.wrapped - target, "cycle angle residual")?;
                if residual.abs() > self.options.angular_tolerance().as_radians() {
                    return Err(Error::AngularResidualExceeded {
                        event: "astronomical cycle boundary",
                        residual_radians: residual.abs(),
                        tolerance_radians: self.options.angular_tolerance().as_radians(),
                    });
                }
                Self::push_unique(
                    &mut events,
                    CycleBoundary::new(
                        root.instant(),
                        sample.event,
                        CycleEvidence::new(
                            root.bracket_start(),
                            root.bracket_end(),
                            root.time_uncertainty(),
                            CycleResidual::Angle(Angle::from_radians(residual)?),
                            root.iterations(),
                            self.evaluations,
                        ),
                    ),
                    self.options.time_tolerance(),
                )?;
            }
            previous_epoch = current_epoch;
            previous_wrapped = current.wrapped;
            previous_unwrapped = current_unwrapped;
        }
        Ok(events)
    }

    fn scalar<F>(
        mut self,
        crossing: ScalarCrossing,
        mut evaluate: F,
    ) -> Result<Vec<CycleBoundary<S>>, Error>
    where
        F: FnMut(Instant<S>, &mut u32) -> Result<ScalarSample, Error>,
    {
        let mut previous_epoch = self.interval.start();
        let mut previous = evaluate(previous_epoch, &mut self.evaluations)?;
        let mut events = Vec::new();
        if previous.is_root(self.options.angular_tolerance()) {
            events.push(CycleBoundary::new(
                previous_epoch,
                previous.event,
                CycleEvidence::new(
                    previous_epoch,
                    previous_epoch,
                    Duration::ZERO,
                    previous.cycle_residual()?,
                    0,
                    self.evaluations,
                ),
            ));
        }

        while previous_epoch < self.interval.end() {
            let remaining = self.interval.end().duration_since(previous_epoch)?;
            let step = remaining.min(self.options.scan_step());
            let current_epoch = previous_epoch.checked_add(step)?;
            let current = evaluate(current_epoch, &mut self.evaluations)?;
            if crossing.crossed(previous.residual, current.residual) {
                let root = BracketedRootSearch::refine(
                    previous_epoch,
                    current_epoch,
                    self.options.time_tolerance(),
                    self.options.max_refinement_iterations(),
                    |epoch| evaluate(epoch, &mut self.evaluations).map(|sample| sample.residual),
                )?;
                let sample = evaluate(root.instant(), &mut self.evaluations)?;
                if matches!(sample.residual_kind, ScalarResidualKind::Angle)
                    && sample.residual.abs() > self.options.angular_tolerance().as_radians()
                {
                    return Err(Error::AngularResidualExceeded {
                        event: "ecliptic-node crossing",
                        residual_radians: sample.residual.abs(),
                        tolerance_radians: self.options.angular_tolerance().as_radians(),
                    });
                }
                Self::push_unique(
                    &mut events,
                    CycleBoundary::new(
                        root.instant(),
                        sample.event,
                        CycleEvidence::new(
                            root.bracket_start(),
                            root.bracket_end(),
                            root.time_uncertainty(),
                            sample.cycle_residual()?,
                            root.iterations(),
                            self.evaluations,
                        ),
                    ),
                    self.options.time_tolerance(),
                )?;
            }
            previous_epoch = current_epoch;
            previous = current;
        }
        Ok(events)
    }

    fn push_unique(
        events: &mut Vec<CycleBoundary<S>>,
        candidate: CycleBoundary<S>,
        tolerance: Duration,
    ) -> Result<(), Error> {
        if let Some(previous) = events.last_mut()
            && candidate
                .instant()
                .duration_since(previous.instant())?
                .checked_abs()?
                <= tolerance
        {
            if candidate.evidence().residual_magnitude() < previous.evidence().residual_magnitude()
            {
                *previous = candidate;
            }
            return Ok(());
        }
        events.push(candidate);
        Ok(())
    }
}

impl<S: TimeScale> CycleEvidence<S> {
    fn residual_magnitude(self) -> f64 {
        match self.residual {
            CycleResidual::Angle(value) => value.as_radians().abs(),
            CycleResidual::RadialSpeed(value) => value.as_metres_per_second().abs(),
        }
    }
}
