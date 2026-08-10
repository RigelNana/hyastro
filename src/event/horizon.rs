use libm::asin;

use std::vec::Vec;

use crate::{
    astro::{
        Astrometry, AtmosphericConditions, ObservedPlace, ReceptionLightTimeOptions,
        VacuumObservedPlace,
    },
    earth::FixedSite,
    ephem::{CelestialBody, EphemerisProvider, SphericalBodyFigure},
    math::{Altitude, Angle},
    time::{Duration, EarthAttitudeTable, EarthOrientationTable, Instant, TimeInterval, TimeScale},
};

use super::{Error, EventEvidence, Events, search::BracketedRootSearch};

/// Coordinate stage used by an altitude-crossing criterion.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum HorizonReference {
    /// Topocentric vacuum altitude.
    Vacuum,
    /// Altitude after applying the retained atmospheric conditions.
    Refracted(AtmosphericConditions),
}

/// Point on a spherical apparent disk whose altitude defines a horizon contact.
///
/// The upper and lower limbs are the points farthest toward and away from the
/// zenith. Their angular offsets are recomputed from the converged topocentric
/// distance at every event-search evaluation. For refracted criteria, refraction
/// is evaluated at the disk centre and the vacuum spherical semidiameter is then
/// applied unchanged; atmospheric differential refraction across the disk is not
/// modelled.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum HorizonDiskPoint {
    /// The apparent disk centre.
    Center,
    /// The vertically upper limb of the selected spherical figure.
    UpperLimb(SphericalBodyFigure),
    /// The vertically lower limb of the selected spherical figure.
    LowerLimb(SphericalBodyFigure),
}

impl HorizonDiskPoint {
    /// Returns the spherical figure when the criterion selects a limb.
    pub const fn figure(self) -> Option<SphericalBodyFigure> {
        match self {
            Self::Center => None,
            Self::UpperLimb(figure) | Self::LowerLimb(figure) => Some(figure),
        }
    }
}

/// Explicit altitude and coordinate stage used to define rise and set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonCriterion {
    altitude: Altitude,
    reference: HorizonReference,
    disk_point: HorizonDiskPoint,
}

impl HorizonCriterion {
    /// Returns the astronomical horizon applied to the vacuum target centre.
    pub const fn geometric_center() -> Self {
        Self {
            altitude: Altitude::from_finite(0.0),
            reference: HorizonReference::Vacuum,
            disk_point: HorizonDiskPoint::Center,
        }
    }

    /// Returns the astronomical horizon applied after atmospheric refraction.
    pub const fn refracted_center(conditions: AtmosphericConditions) -> Self {
        Self {
            altitude: Altitude::from_finite(0.0),
            reference: HorizonReference::Refracted(conditions),
            disk_point: HorizonDiskPoint::Center,
        }
    }

    /// Returns the astronomical horizon applied to a vacuum spherical upper limb.
    pub const fn geometric_upper_limb(figure: SphericalBodyFigure) -> Self {
        Self {
            altitude: Altitude::from_finite(0.0),
            reference: HorizonReference::Vacuum,
            disk_point: HorizonDiskPoint::UpperLimb(figure),
        }
    }

    /// Returns the astronomical horizon applied to a vacuum spherical lower limb.
    pub const fn geometric_lower_limb(figure: SphericalBodyFigure) -> Self {
        Self {
            altitude: Altitude::from_finite(0.0),
            reference: HorizonReference::Vacuum,
            disk_point: HorizonDiskPoint::LowerLimb(figure),
        }
    }

    /// Constructs an arbitrary vacuum-altitude crossing for a selected disk point.
    ///
    /// A conventional fixed horizon-refraction allowance can be represented by
    /// selecting a limb and supplying the corresponding negative vacuum altitude.
    pub const fn vacuum_disk_altitude(altitude: Altitude, disk_point: HorizonDiskPoint) -> Self {
        Self {
            altitude,
            reference: HorizonReference::Vacuum,
            disk_point,
        }
    }

    /// Returns the astronomical horizon applied to a refracted spherical upper limb.
    pub const fn refracted_upper_limb(
        figure: SphericalBodyFigure,
        conditions: AtmosphericConditions,
    ) -> Self {
        Self {
            altitude: Altitude::from_finite(0.0),
            reference: HorizonReference::Refracted(conditions),
            disk_point: HorizonDiskPoint::UpperLimb(figure),
        }
    }

    /// Returns the astronomical horizon applied to a refracted spherical lower limb.
    pub const fn refracted_lower_limb(
        figure: SphericalBodyFigure,
        conditions: AtmosphericConditions,
    ) -> Self {
        Self {
            altitude: Altitude::from_finite(0.0),
            reference: HorizonReference::Refracted(conditions),
            disk_point: HorizonDiskPoint::LowerLimb(figure),
        }
    }

    /// Constructs an arbitrary refracted-altitude crossing for a selected disk point.
    pub const fn refracted_disk_altitude(
        altitude: Altitude,
        disk_point: HorizonDiskPoint,
        conditions: AtmosphericConditions,
    ) -> Self {
        Self {
            altitude,
            reference: HorizonReference::Refracted(conditions),
            disk_point,
        }
    }

    /// Constructs an arbitrary vacuum-altitude crossing criterion.
    pub const fn vacuum_altitude(altitude: Altitude) -> Self {
        Self::vacuum_disk_altitude(altitude, HorizonDiskPoint::Center)
    }

    /// Constructs an arbitrary refracted-altitude crossing criterion.
    pub const fn refracted_altitude(altitude: Altitude, conditions: AtmosphericConditions) -> Self {
        Self::refracted_disk_altitude(altitude, HorizonDiskPoint::Center, conditions)
    }

    /// Returns the target altitude whose crossings define rise and set.
    pub const fn altitude(self) -> Altitude {
        self.altitude
    }

    /// Returns whether the criterion uses vacuum or refracted altitude.
    pub const fn reference(self) -> HorizonReference {
        self.reference
    }

    /// Returns the selected centre or spherical limb.
    pub const fn disk_point(self) -> HorizonDiskPoint {
        self.disk_point
    }
}

/// Explicit scanning and refinement controls for horizon-event searches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonSearchOptions {
    scan_step: Duration,
    time_tolerance: Duration,
    angular_tolerance: Angle,
    max_refinement_iterations: u32,
    max_evaluations: u32,
    light_time: ReceptionLightTimeOptions,
}

impl HorizonSearchOptions {
    /// Largest supported coarse step for diurnal horizon and meridian crossings.
    pub const MAX_SCAN_STEP: Duration =
        Duration::from_nanoseconds(6 * 3_600 * Duration::NANOSECONDS_PER_SECOND);

    /// Constructs validated horizon-search controls.
    pub fn new(
        scan_step: Duration,
        time_tolerance: Duration,
        angular_tolerance: Angle,
        max_refinement_iterations: u32,
        max_evaluations: u32,
        light_time: ReceptionLightTimeOptions,
    ) -> Result<Self, Error> {
        if scan_step <= Duration::ZERO || scan_step > Self::MAX_SCAN_STEP {
            return Err(Error::InvalidSearchDuration {
                field: "horizon-event scan step",
                nanoseconds: scan_step.as_nanoseconds(),
                maximum_nanoseconds: Self::MAX_SCAN_STEP.as_nanoseconds(),
            });
        }
        if time_tolerance <= Duration::ZERO || time_tolerance > scan_step {
            return Err(Error::InvalidSearchDuration {
                field: "horizon-event time tolerance",
                nanoseconds: time_tolerance.as_nanoseconds(),
                maximum_nanoseconds: scan_step.as_nanoseconds(),
            });
        }
        let maximum_angular_tolerance = 1.0_f64.to_radians();
        if angular_tolerance.as_radians() <= 0.0
            || angular_tolerance.as_radians() > maximum_angular_tolerance
        {
            return Err(Error::InvalidAngularTolerance {
                field: "horizon-event angular tolerance",
                radians: angular_tolerance.as_radians(),
                maximum_radians: maximum_angular_tolerance,
            });
        }
        if max_refinement_iterations == 0 {
            return Err(Error::InvalidSearchLimit {
                field: "horizon-event refinement iterations",
                value: max_refinement_iterations,
            });
        }
        if max_evaluations == 0 {
            return Err(Error::InvalidSearchLimit {
                field: "horizon-event evaluations",
                value: max_evaluations,
            });
        }
        Ok(Self {
            scan_step,
            time_tolerance,
            angular_tolerance,
            max_refinement_iterations,
            max_evaluations,
            light_time,
        })
    }

    /// Returns one-hour scanning, one-millisecond timing, and nanoradian angular tolerance.
    pub const fn standard() -> Self {
        Self {
            scan_step: Duration::from_nanoseconds(3_600 * Duration::NANOSECONDS_PER_SECOND),
            time_tolerance: Duration::from_nanoseconds(1_000_000),
            angular_tolerance: Angle::from_finite(1.0e-9),
            max_refinement_iterations: 64,
            max_evaluations: 8_192,
            light_time: ReceptionLightTimeOptions::standard(),
        }
    }

    /// Returns the maximum physical duration between coarse samples.
    pub const fn scan_step(self) -> Duration {
        self.scan_step
    }

    /// Returns the required final time-bracket width.
    pub const fn time_tolerance(self) -> Duration {
        self.time_tolerance
    }

    /// Returns the maximum accepted altitude or meridian residual.
    pub const fn angular_tolerance(self) -> Angle {
        self.angular_tolerance
    }

    /// Returns the maximum Brent iterations per crossing.
    pub const fn max_refinement_iterations(self) -> u32 {
        self.max_refinement_iterations
    }

    /// Returns the maximum astrometric evaluations for one search.
    pub const fn max_evaluations(self) -> u32 {
        self.max_evaluations
    }

    /// Returns the reception light-time controls used for each target place.
    pub const fn light_time(self) -> ReceptionLightTimeOptions {
        self.light_time
    }
}

/// Identity of one rise, set, or meridian-crossing event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HorizonEventKind {
    /// Target altitude crosses upward through the selected criterion.
    Rise,
    /// Target altitude crosses downward through the selected criterion.
    Set,
    /// Target crosses the upper branch of the local meridian.
    UpperTransit,
    /// Target crosses the lower branch of the local meridian.
    LowerTransit,
}

/// Rise/set reachability classification over the requested closed interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HorizonVisibility {
    /// At least one ordinary rise and set both occur in the interval.
    RisesAndSets,
    /// The target centre remains above the selected altitude throughout the interval.
    CircumpolarOverInterval,
    /// The target centre remains below the selected altitude throughout the interval.
    NeverRisesOverInterval,
    /// The interval cuts off an otherwise paired rise or set.
    TruncatedByInterval,
    /// More than one rise/set cycle occurs in the interval.
    MultipleCycles,
    /// The sampled path reaches the criterion without a resolved sign-changing crossing.
    GrazesCriterion,
}

/// One refined finite-target horizon or meridian event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonEvent<S: TimeScale> {
    kind: HorizonEventKind,
    vacuum: VacuumObservedPlace<S>,
    observed: Option<ObservedPlace<S>>,
    evidence: EventEvidence<S>,
}

impl<S: TimeScale> HorizonEvent<S> {
    /// Returns the event identity.
    pub const fn kind(self) -> HorizonEventKind {
        self.kind
    }

    /// Returns the refined physical event instant.
    pub const fn instant(self) -> Instant<S> {
        self.vacuum.reception_epoch()
    }

    /// Returns the finite target's vacuum place at the event.
    pub const fn vacuum_place(self) -> VacuumObservedPlace<S> {
        self.vacuum
    }

    /// Returns the refracted place when the search criterion requested one.
    pub const fn observed_place(self) -> Option<ObservedPlace<S>> {
        self.observed
    }

    /// Returns the numerical evidence retained for the event.
    pub const fn evidence(self) -> EventEvidence<S> {
        self.evidence
    }
}

/// Complete rise, set, and transit result over one closed interval.
#[derive(Debug, Clone, PartialEq)]
pub struct HorizonEventSearch<S: TimeScale> {
    target: CelestialBody,
    interval: TimeInterval<S>,
    criterion: HorizonCriterion,
    visibility: HorizonVisibility,
    events: Vec<HorizonEvent<S>>,
}

impl<S: TimeScale> HorizonEventSearch<S> {
    /// Returns the searched finite target.
    pub const fn target(&self) -> CelestialBody {
        self.target
    }

    /// Returns the searched closed physical-time interval.
    pub const fn interval(&self) -> TimeInterval<S> {
        self.interval
    }

    /// Returns the altitude and refraction criterion.
    pub const fn criterion(&self) -> HorizonCriterion {
        self.criterion
    }

    /// Returns rise/set reachability over the interval.
    pub const fn visibility(&self) -> HorizonVisibility {
        self.visibility
    }

    /// Returns all rise, set, upper-transit, and lower-transit events in time order.
    pub fn events(&self) -> &[HorizonEvent<S>] {
        &self.events
    }
}

#[derive(Debug, Clone, Copy)]
struct HorizonSample<S: TimeScale> {
    vacuum: VacuumObservedPlace<S>,
    observed: Option<ObservedPlace<S>>,
    altitude_residual: f64,
    meridian_residual: f64,
}

#[derive(Debug, Clone, Copy)]
enum HorizonResidual {
    Altitude,
    Meridian,
}

trait HorizonObserver {
    fn vacuum_observed_place<S: TimeScale>(
        &self,
        site: &FixedSite,
        target: CelestialBody,
        epoch: Instant<S>,
        light_time: ReceptionLightTimeOptions,
    ) -> Result<VacuumObservedPlace<S>, crate::astro::Error>;
}

impl<'context, 'data, 'eop, P: EphemerisProvider + ?Sized> HorizonObserver
    for Astrometry<'context, 'data, EarthOrientationTable<'eop>, P>
{
    fn vacuum_observed_place<S: TimeScale>(
        &self,
        site: &FixedSite,
        target: CelestialBody,
        epoch: Instant<S>,
        light_time: ReceptionLightTimeOptions,
    ) -> Result<VacuumObservedPlace<S>, crate::astro::Error> {
        self.fixed_observer_at(site, epoch)?
            .vacuum_observed_place(target, light_time)
    }
}

impl<'context, 'data, 'eop, P: EphemerisProvider + ?Sized> HorizonObserver
    for Astrometry<'context, 'data, EarthAttitudeTable<'eop>, P>
{
    fn vacuum_observed_place<S: TimeScale>(
        &self,
        site: &FixedSite,
        target: CelestialBody,
        epoch: Instant<S>,
        light_time: ReceptionLightTimeOptions,
    ) -> Result<VacuumObservedPlace<S>, crate::astro::Error> {
        self.fixed_observer_with_nominal_rotation_at(site, epoch)?
            .vacuum_observed_place(target, light_time)
    }
}

impl<'context, 'data, 'eop, P: EphemerisProvider + ?Sized>
    Events<'context, 'data, EarthOrientationTable<'eop>, P>
{
    /// Finds finite-target rise, set, and transit events using observed Earth rotation.
    pub fn horizon_events_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        target: CelestialBody,
        interval: TimeInterval<S>,
        criterion: HorizonCriterion,
        options: HorizonSearchOptions,
    ) -> Result<HorizonEventSearch<S>, Error> {
        self.search_horizon_events_in(site, target, interval, criterion, options)
    }
}

impl<'context, 'data, 'eop, P: EphemerisProvider + ?Sized>
    Events<'context, 'data, EarthAttitudeTable<'eop>, P>
{
    /// Finds finite-target rise, set, and transit events using nominal Earth rotation.
    ///
    /// This path retains observed UT1, polar motion, and celestial-pole offsets,
    /// but uses the IERS nominal angular speed because the context has no
    /// measured length-of-day value.
    pub fn horizon_events_with_nominal_rotation_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        target: CelestialBody,
        interval: TimeInterval<S>,
        criterion: HorizonCriterion,
        options: HorizonSearchOptions,
    ) -> Result<HorizonEventSearch<S>, Error> {
        self.search_horizon_events_in(site, target, interval, criterion, options)
    }
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Events<'context, 'data, E, P> {
    fn search_horizon_events_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        target: CelestialBody,
        interval: TimeInterval<S>,
        criterion: HorizonCriterion,
        options: HorizonSearchOptions,
    ) -> Result<HorizonEventSearch<S>, Error>
    where
        Astrometry<'context, 'data, E, P>: HorizonObserver,
    {
        let mut evaluations = 0_u32;
        let mut events = Vec::new();
        let mut previous_epoch = interval.start();
        let mut previous = self.evaluate_horizon(
            site,
            target,
            previous_epoch,
            criterion,
            options,
            &mut evaluations,
        )?;
        let mut minimum_altitude = previous.altitude_residual;
        let mut maximum_altitude = previous.altitude_residual;
        let tolerance = options.angular_tolerance().as_radians();
        let mut is_first_segment = true;

        while previous_epoch < interval.end() {
            let remaining = interval.end().duration_since(previous_epoch)?;
            let step = remaining.min(options.scan_step());
            let current_epoch = previous_epoch.checked_add(step)?;
            let current = self.evaluate_horizon(
                site,
                target,
                current_epoch,
                criterion,
                options,
                &mut evaluations,
            )?;
            minimum_altitude = minimum_altitude.min(current.altitude_residual);
            maximum_altitude = maximum_altitude.max(current.altitude_residual);

            if is_first_segment && previous.altitude_residual.abs() <= tolerance {
                let kind = if current.altitude_residual >= previous.altitude_residual {
                    HorizonEventKind::Rise
                } else {
                    HorizonEventKind::Set
                };
                Self::push_unique_horizon_event(
                    &mut events,
                    Self::endpoint_event(kind, previous, previous_epoch),
                    options.time_tolerance(),
                )?;
            }
            if previous.altitude_residual * current.altitude_residual < 0.0 {
                let kind = if current.altitude_residual > previous.altitude_residual {
                    HorizonEventKind::Rise
                } else {
                    HorizonEventKind::Set
                };
                let event = self.refine_horizon_event(
                    site,
                    target,
                    previous_epoch,
                    current_epoch,
                    criterion,
                    HorizonResidual::Altitude,
                    kind,
                    options,
                    &mut evaluations,
                )?;
                Self::push_unique_horizon_event(&mut events, event, options.time_tolerance())?;
            } else if current.altitude_residual.abs() <= tolerance {
                let kind = if current.altitude_residual >= previous.altitude_residual {
                    HorizonEventKind::Rise
                } else {
                    HorizonEventKind::Set
                };
                Self::push_unique_horizon_event(
                    &mut events,
                    Self::endpoint_event(kind, current, current_epoch),
                    options.time_tolerance(),
                )?;
            }

            if is_first_segment && previous.meridian_residual.abs() <= tolerance {
                let kind = if current.meridian_residual < previous.meridian_residual {
                    HorizonEventKind::UpperTransit
                } else {
                    HorizonEventKind::LowerTransit
                };
                Self::push_unique_horizon_event(
                    &mut events,
                    Self::endpoint_event(kind, previous, previous_epoch),
                    options.time_tolerance(),
                )?;
            }
            if previous.meridian_residual * current.meridian_residual < 0.0 {
                let kind = if current.meridian_residual < previous.meridian_residual {
                    HorizonEventKind::UpperTransit
                } else {
                    HorizonEventKind::LowerTransit
                };
                let event = self.refine_horizon_event(
                    site,
                    target,
                    previous_epoch,
                    current_epoch,
                    criterion,
                    HorizonResidual::Meridian,
                    kind,
                    options,
                    &mut evaluations,
                )?;
                Self::push_unique_horizon_event(&mut events, event, options.time_tolerance())?;
            } else if current.meridian_residual.abs() <= tolerance {
                let kind = if current.meridian_residual < previous.meridian_residual {
                    HorizonEventKind::UpperTransit
                } else {
                    HorizonEventKind::LowerTransit
                };
                Self::push_unique_horizon_event(
                    &mut events,
                    Self::endpoint_event(kind, current, current_epoch),
                    options.time_tolerance(),
                )?;
            }

            previous_epoch = current_epoch;
            previous = current;
            is_first_segment = false;
        }

        events.sort_by_key(|event| event.instant());
        let visibility = Self::classify_horizon_visibility(
            &events,
            minimum_altitude,
            maximum_altitude,
            tolerance,
        );
        Ok(HorizonEventSearch {
            target,
            interval,
            criterion,
            visibility,
            events,
        })
    }

    fn evaluate_horizon<S: TimeScale>(
        &self,
        site: &FixedSite,
        target: CelestialBody,
        epoch: Instant<S>,
        criterion: HorizonCriterion,
        options: HorizonSearchOptions,
        evaluations: &mut u32,
    ) -> Result<HorizonSample<S>, Error>
    where
        Astrometry<'context, 'data, E, P>: HorizonObserver,
    {
        if *evaluations >= options.max_evaluations() {
            return Err(Error::EvaluationLimitExceeded {
                maximum: options.max_evaluations(),
            });
        }
        *evaluations += 1;
        let vacuum =
            self.astrometry
                .vacuum_observed_place(site, target, epoch, options.light_time())?;
        let observed = match criterion.reference() {
            HorizonReference::Vacuum => None,
            HorizonReference::Refracted(conditions) => Some(vacuum.apply_refraction(conditions)?),
        };
        let horizontal = observed
            .map(ObservedPlace::horizontal)
            .unwrap_or_else(|| vacuum.horizontal());
        let east_component = horizontal.enu_components()[0].clamp(-1.0, 1.0);
        let limb_offset = match criterion.disk_point() {
            HorizonDiskPoint::Center => 0.0,
            HorizonDiskPoint::UpperLimb(figure) => {
                vacuum.apparent_disk(figure)?.semidiameter().as_radians()
            }
            HorizonDiskPoint::LowerLimb(figure) => {
                -vacuum.apparent_disk(figure)?.semidiameter().as_radians()
            }
        };
        Ok(HorizonSample {
            vacuum,
            observed,
            altitude_residual: horizontal.altitude().as_radians() + limb_offset
                - criterion.altitude().as_radians(),
            meridian_residual: asin(east_component),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn refine_horizon_event<S: TimeScale>(
        &self,
        site: &FixedSite,
        target: CelestialBody,
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        criterion: HorizonCriterion,
        residual_kind: HorizonResidual,
        event_kind: HorizonEventKind,
        options: HorizonSearchOptions,
        evaluations: &mut u32,
    ) -> Result<HorizonEvent<S>, Error>
    where
        Astrometry<'context, 'data, E, P>: HorizonObserver,
    {
        let evaluations_before = *evaluations;
        let root = BracketedRootSearch::refine(
            bracket_start,
            bracket_end,
            options.time_tolerance(),
            options.max_refinement_iterations(),
            |epoch| {
                let sample =
                    self.evaluate_horizon(site, target, epoch, criterion, options, evaluations)?;
                Ok(match residual_kind {
                    HorizonResidual::Altitude => sample.altitude_residual,
                    HorizonResidual::Meridian => sample.meridian_residual,
                })
            },
        )?;
        let sample = self.evaluate_horizon(
            site,
            target,
            root.instant(),
            criterion,
            options,
            evaluations,
        )?;
        let residual = match residual_kind {
            HorizonResidual::Altitude => sample.altitude_residual,
            HorizonResidual::Meridian => sample.meridian_residual,
        };
        if residual.abs() > options.angular_tolerance().as_radians() {
            return Err(Error::AngularResidualExceeded {
                event: match event_kind {
                    HorizonEventKind::Rise => "rise",
                    HorizonEventKind::Set => "set",
                    HorizonEventKind::UpperTransit => "upper transit",
                    HorizonEventKind::LowerTransit => "lower transit",
                },
                residual_radians: residual.abs(),
                tolerance_radians: options.angular_tolerance().as_radians(),
            });
        }
        Ok(HorizonEvent {
            kind: event_kind,
            vacuum: sample.vacuum,
            observed: sample.observed,
            evidence: EventEvidence::new(
                root.bracket_start(),
                root.bracket_end(),
                root.time_uncertainty(),
                Angle::from_radians(residual)?,
                root.iterations(),
                *evaluations - evaluations_before,
            ),
        })
    }

    fn endpoint_event<S: TimeScale>(
        kind: HorizonEventKind,
        sample: HorizonSample<S>,
        epoch: Instant<S>,
    ) -> HorizonEvent<S> {
        let residual = match kind {
            HorizonEventKind::Rise | HorizonEventKind::Set => sample.altitude_residual,
            HorizonEventKind::UpperTransit | HorizonEventKind::LowerTransit => {
                sample.meridian_residual
            }
        };
        HorizonEvent {
            kind,
            vacuum: sample.vacuum,
            observed: sample.observed,
            evidence: EventEvidence::new(
                epoch,
                epoch,
                Duration::ZERO,
                Angle::from_finite(residual),
                0,
                1,
            ),
        }
    }

    fn classify_horizon_visibility<S: TimeScale>(
        events: &[HorizonEvent<S>],
        minimum_altitude: f64,
        maximum_altitude: f64,
        tolerance: f64,
    ) -> HorizonVisibility {
        let rises = events
            .iter()
            .filter(|event| event.kind() == HorizonEventKind::Rise)
            .count();
        let sets = events
            .iter()
            .filter(|event| event.kind() == HorizonEventKind::Set)
            .count();
        match rises + sets {
            0 if minimum_altitude > tolerance => HorizonVisibility::CircumpolarOverInterval,
            0 if maximum_altitude < -tolerance => HorizonVisibility::NeverRisesOverInterval,
            0 => HorizonVisibility::GrazesCriterion,
            1 => HorizonVisibility::TruncatedByInterval,
            2 if rises == 1 && sets == 1 => HorizonVisibility::RisesAndSets,
            _ => HorizonVisibility::MultipleCycles,
        }
    }

    fn push_unique_horizon_event<S: TimeScale>(
        events: &mut Vec<HorizonEvent<S>>,
        candidate: HorizonEvent<S>,
        tolerance: Duration,
    ) -> Result<(), Error> {
        if let Some(previous) = events.iter_mut().rev().find(|event| {
            event.kind() == candidate.kind()
                && candidate
                    .instant()
                    .duration_since(event.instant())
                    .and_then(Duration::checked_abs)
                    .is_ok_and(|difference| difference <= tolerance)
        }) {
            if candidate.evidence().residual().as_radians().abs()
                < previous.evidence().residual().as_radians().abs()
            {
                *previous = candidate;
            }
            return Ok(());
        }
        events.push(candidate);
        Ok(())
    }
}
