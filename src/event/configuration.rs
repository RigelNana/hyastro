use core::{
    f64::consts::{FRAC_PI_2, PI},
    fmt,
};
use std::vec::Vec;

use crate::{
    earth::FixedSite,
    ephem::{CelestialBody, EphemerisProvider},
    math::{Angle, AngularSpeed, Separation},
    time::{Duration, EarthOrientationTable, Instant, TimeInterval, TimeScale},
};

use super::{
    AngularEventSearchOptions, AstrometricMode, Error, EventBodyPosition, EventEvidence, Events,
    ExtremumEvidence, ExtremumSearchOptions, ObservationOrigin, RelativeBodyQuery,
    relative::{
        FixedSiteRelativeSampler, GeocentricRelativeSampler, RelativeObservation, RelativeSampler,
    },
    search::{BracketedRootSearch, ExtremumSense, SampledExtremumSearch},
};

/// Coordinate difference used to define a conjunction, opposition, or quadrature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConfigurationCoordinate {
    /// Target-minus-reference longitude on the true ecliptic and equinox of date.
    EclipticLongitude,
    /// Target-minus-reference right ascension on the true equator and equinox of date.
    RightAscension,
}

/// One directed relative-coordinate configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConfigurationKind {
    /// Zero target-minus-reference coordinate difference.
    Conjunction,
    /// A 180-degree target-minus-reference coordinate difference.
    Opposition,
    /// A positive 90-degree target-minus-reference coordinate difference.
    EasternQuadrature,
    /// A negative 90-degree target-minus-reference coordinate difference.
    WesternQuadrature,
}

impl ConfigurationKind {
    /// Returns the defining directed target-minus-reference angle in `[0, 2π)`.
    pub const fn target_angle(self) -> Angle {
        match self {
            Self::Conjunction => Angle::from_finite(0.0),
            Self::Opposition => Angle::from_finite(PI),
            Self::EasternQuadrature => Angle::from_finite(FRAC_PI_2),
            Self::WesternQuadrature => Angle::from_finite(3.0 * FRAC_PI_2),
        }
    }
}

/// A complete specification for one family of relative-coordinate events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConfigurationQuery {
    bodies: RelativeBodyQuery,
    kind: ConfigurationKind,
    coordinate: ConfigurationCoordinate,
}

impl ConfigurationQuery {
    /// Constructs a configuration query from distinct bodies and explicit coordinate semantics.
    pub const fn new(
        bodies: RelativeBodyQuery,
        kind: ConfigurationKind,
        coordinate: ConfigurationCoordinate,
    ) -> Self {
        Self {
            bodies,
            kind,
            coordinate,
        }
    }

    /// Returns the ordered target and reference bodies and astrometric mode.
    pub const fn bodies(self) -> RelativeBodyQuery {
        self.bodies
    }

    /// Returns the requested relative configuration.
    pub const fn kind(self) -> ConfigurationKind {
        self.kind
    }

    /// Returns the coordinate difference that defines the event.
    pub const fn coordinate(self) -> ConfigurationCoordinate {
        self.coordinate
    }
}

/// Distance-order classification for a Sun-referenced conjunction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SolarConjunctionKind {
    /// The target is nearer to the observer than the Sun.
    Inferior,
    /// The target is farther from the observer than the Sun.
    Superior,
}

/// One refined conjunction, opposition, or quadrature event.
pub struct ConfigurationEvent<S: TimeScale> {
    query: ConfigurationQuery,
    origin: ObservationOrigin,
    target: EventBodyPosition<S>,
    reference: EventBodyPosition<S>,
    longitude_difference: Angle,
    right_ascension_difference: Angle,
    separation: Separation,
    evidence: EventEvidence<S>,
}

impl<S: TimeScale> ConfigurationEvent<S> {
    fn new(
        query: ConfigurationQuery,
        origin: ObservationOrigin,
        observation: RelativeObservation<S>,
        evidence: EventEvidence<S>,
    ) -> Self {
        Self {
            query,
            origin,
            target: observation.target(),
            reference: observation.reference(),
            longitude_difference: observation.longitude_difference(),
            right_ascension_difference: observation.right_ascension_difference(),
            separation: observation.separation(),
            evidence,
        }
    }

    /// Returns the complete defining query.
    pub const fn query(self) -> ConfigurationQuery {
        self.query
    }

    /// Returns the event instant.
    pub const fn instant(self) -> Instant<S> {
        self.target.epoch()
    }

    /// Returns whether the event was evaluated at the geocentre or a fixed site.
    pub const fn origin(self) -> ObservationOrigin {
        self.origin
    }

    /// Returns the evaluated target position.
    pub const fn target(self) -> EventBodyPosition<S> {
        self.target
    }

    /// Returns the evaluated reference position.
    pub const fn reference(self) -> EventBodyPosition<S> {
        self.reference
    }

    /// Returns target-minus-reference true-ecliptic longitude in `[0, 2π)`.
    pub const fn longitude_difference(self) -> Angle {
        self.longitude_difference
    }

    /// Returns target-minus-reference true-equatorial right ascension in `[0, 2π)`.
    pub const fn right_ascension_difference(self) -> Angle {
        self.right_ascension_difference
    }

    /// Returns the stable great-circle separation of the two directions.
    pub const fn separation(self) -> Separation {
        self.separation
    }

    /// Classifies an inferior or superior conjunction when the reference is the Sun.
    pub fn solar_conjunction_kind(self) -> Option<SolarConjunctionKind> {
        if self.query.kind() != ConfigurationKind::Conjunction
            || self.query.bodies().reference() != CelestialBody::Sun
        {
            return None;
        }
        if self.target.distance() < self.reference.distance() {
            Some(SolarConjunctionKind::Inferior)
        } else {
            Some(SolarConjunctionKind::Superior)
        }
    }

    /// Returns numerical root-search evidence.
    pub const fn evidence(self) -> EventEvidence<S> {
        self.evidence
    }
}

impl<S: TimeScale> Copy for ConfigurationEvent<S> {}

impl<S: TimeScale> Clone for ConfigurationEvent<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for ConfigurationEvent<S> {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query
            && self.origin == other.origin
            && self.target == other.target
            && self.reference == other.reference
            && self.longitude_difference == other.longitude_difference
            && self.right_ascension_difference == other.right_ascension_difference
            && self.separation == other.separation
            && self.evidence == other.evidence
    }
}

impl<S: TimeScale> fmt::Debug for ConfigurationEvent<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfigurationEvent")
            .field("query", &self.query)
            .field("origin", &self.origin)
            .field("target", &self.target)
            .field("reference", &self.reference)
            .field("longitude_difference", &self.longitude_difference)
            .field(
                "right_ascension_difference",
                &self.right_ascension_difference,
            )
            .field("separation", &self.separation)
            .field("evidence", &self.evidence)
            .finish()
    }
}

/// Side of the reference body on which a greatest elongation occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ElongationSide {
    /// The target has a positive signed ecliptic-longitude difference from the reference.
    Eastern,
    /// The target has a negative signed ecliptic-longitude difference from the reference.
    Western,
}

/// One local maximum of the angular separation between two bodies.
pub struct GreatestElongationEvent<S: TimeScale> {
    query: RelativeBodyQuery,
    origin: ObservationOrigin,
    side: ElongationSide,
    target: EventBodyPosition<S>,
    reference: EventBodyPosition<S>,
    longitude_difference: Angle,
    separation: Separation,
    evidence: ExtremumEvidence<S>,
}

impl<S: TimeScale> GreatestElongationEvent<S> {
    fn new(
        query: RelativeBodyQuery,
        origin: ObservationOrigin,
        observation: RelativeObservation<S>,
        evidence: ExtremumEvidence<S>,
    ) -> Result<Self, Error> {
        let signed = Angle::wrap_signed(
            observation.longitude_difference().as_radians(),
            "greatest-elongation side",
        )?;
        let side = if signed >= 0.0 {
            ElongationSide::Eastern
        } else {
            ElongationSide::Western
        };
        Ok(Self {
            query,
            origin,
            side,
            target: observation.target(),
            reference: observation.reference(),
            longitude_difference: observation.longitude_difference(),
            separation: observation.separation(),
            evidence,
        })
    }

    /// Returns the ordered body pair and astrometric semantics.
    pub const fn query(self) -> RelativeBodyQuery {
        self.query
    }

    /// Returns the extremum instant.
    pub const fn instant(self) -> Instant<S> {
        self.target.epoch()
    }

    /// Returns whether the event was evaluated at the geocentre or a fixed site.
    pub const fn origin(self) -> ObservationOrigin {
        self.origin
    }

    /// Returns the eastern or western elongation classification.
    pub const fn side(self) -> ElongationSide {
        self.side
    }

    /// Returns the evaluated target position.
    pub const fn target(self) -> EventBodyPosition<S> {
        self.target
    }

    /// Returns the evaluated reference position.
    pub const fn reference(self) -> EventBodyPosition<S> {
        self.reference
    }

    /// Returns target-minus-reference true-ecliptic longitude in `[0, 2π)`.
    pub const fn longitude_difference(self) -> Angle {
        self.longitude_difference
    }

    /// Returns the locally maximal great-circle separation.
    pub const fn separation(self) -> Separation {
        self.separation
    }

    /// Returns numerical bounded-extremum evidence.
    pub const fn evidence(self) -> ExtremumEvidence<S> {
        self.evidence
    }
}

impl<S: TimeScale> Copy for GreatestElongationEvent<S> {}

impl<S: TimeScale> Clone for GreatestElongationEvent<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for GreatestElongationEvent<S> {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query
            && self.origin == other.origin
            && self.side == other.side
            && self.target == other.target
            && self.reference == other.reference
            && self.longitude_difference == other.longitude_difference
            && self.separation == other.separation
            && self.evidence == other.evidence
    }
}

impl<S: TimeScale> fmt::Debug for GreatestElongationEvent<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GreatestElongationEvent")
            .field("query", &self.query)
            .field("origin", &self.origin)
            .field("side", &self.side)
            .field("target", &self.target)
            .field("reference", &self.reference)
            .field("longitude_difference", &self.longitude_difference)
            .field("separation", &self.separation)
            .field("evidence", &self.evidence)
            .finish()
    }
}

/// Direction of apparent or geometric longitudinal motion after a stationary point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StationKind {
    /// Prograde motion changes to retrograde motion.
    Retrograde,
    /// Retrograde motion changes to prograde motion.
    Direct,
}

/// A validated stationary-point query for one body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StationQuery {
    body: CelestialBody,
    mode: AstrometricMode,
}

impl StationQuery {
    /// Constructs a stationary-longitude query.
    pub const fn new(body: CelestialBody, mode: AstrometricMode) -> Self {
        Self { body, mode }
    }

    /// Returns the body whose true-ecliptic longitude is differentiated.
    pub const fn body(self) -> CelestialBody {
        self.body
    }

    /// Returns the selected geometric or apparent-place semantics.
    pub const fn mode(self) -> AstrometricMode {
        self.mode
    }
}

/// Numerical evidence for a refined stationary point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StationEvidence<S: TimeScale> {
    bracket_start: Instant<S>,
    bracket_end: Instant<S>,
    time_uncertainty: Duration,
    residual_rate: AngularSpeed,
    iterations: u32,
    evaluations: u32,
}

impl<S: TimeScale> StationEvidence<S> {
    fn new(
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        time_uncertainty: Duration,
        residual_rate: AngularSpeed,
        iterations: u32,
        evaluations: u32,
    ) -> Self {
        Self {
            bracket_start,
            bracket_end,
            time_uncertainty,
            residual_rate,
            iterations,
            evaluations,
        }
    }

    /// Returns the final inclusive bracket start.
    pub const fn bracket_start(self) -> Instant<S> {
        self.bracket_start
    }

    /// Returns the final inclusive bracket end.
    pub const fn bracket_end(self) -> Instant<S> {
        self.bracket_end
    }

    /// Returns half the final bracket width.
    pub const fn time_uncertainty(self) -> Duration {
        self.time_uncertainty
    }

    /// Returns the final central-difference longitude rate.
    pub const fn residual_rate(self) -> AngularSpeed {
        self.residual_rate
    }

    /// Returns the completed Brent iterations.
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns the cumulative astrometric evaluations consumed by the search.
    pub const fn evaluations(self) -> u32 {
        self.evaluations
    }
}

/// One transition between prograde and retrograde longitudinal motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StationEvent<S: TimeScale> {
    query: StationQuery,
    origin: ObservationOrigin,
    kind: StationKind,
    position: EventBodyPosition<S>,
    evidence: StationEvidence<S>,
}

impl<S: TimeScale> StationEvent<S> {
    /// Returns the defining body and astrometric semantics.
    pub const fn query(self) -> StationQuery {
        self.query
    }

    /// Returns the stationary instant.
    pub const fn instant(self) -> Instant<S> {
        self.position.epoch()
    }

    /// Returns whether the event was evaluated at the geocentre or a fixed site.
    pub const fn origin(self) -> ObservationOrigin {
        self.origin
    }

    /// Returns the post-station direction of longitudinal motion.
    pub const fn kind(self) -> StationKind {
        self.kind
    }

    /// Returns the evaluated body position at the stationary instant.
    pub const fn position(self) -> EventBodyPosition<S> {
        self.position
    }

    /// Returns numerical root-search evidence including the residual angular rate.
    pub const fn evidence(self) -> StationEvidence<S> {
        self.evidence
    }
}

struct ConfigurationSearch<S: TimeScale, R> {
    interval: TimeInterval<S>,
    query: ConfigurationQuery,
    options: AngularEventSearchOptions,
    sampler: R,
    evaluations: u32,
}

impl<S: TimeScale, R: RelativeSampler<S>> ConfigurationSearch<S, R> {
    const fn new(
        interval: TimeInterval<S>,
        query: ConfigurationQuery,
        options: AngularEventSearchOptions,
        sampler: R,
    ) -> Self {
        Self {
            interval,
            query,
            options,
            sampler,
            evaluations: 0,
        }
    }

    fn events(mut self) -> Result<Vec<ConfigurationEvent<S>>, Error> {
        let mut previous_epoch = self.interval.start();
        let previous = self.evaluate(previous_epoch)?;
        let mut previous_residual = self.residual(previous)?;
        let mut events = Vec::new();
        if previous_residual.abs() <= self.options.angular_tolerance().as_radians() {
            let evidence = EventEvidence::new(
                previous_epoch,
                previous_epoch,
                Duration::ZERO,
                Angle::from_radians(previous_residual)?,
                0,
                self.evaluations,
            );
            events.push(ConfigurationEvent::new(
                self.query,
                self.sampler.origin(),
                previous,
                evidence,
            ));
        }

        while previous_epoch < self.interval.end() {
            let remaining = self.interval.end().duration_since(previous_epoch)?;
            let step = remaining.min(self.options.scan_step());
            let current_epoch = previous_epoch.checked_add(step)?;
            let current = self.evaluate(current_epoch)?;
            let current_residual = self.residual(current)?;
            let crosses_target = (previous_residual * current_residual < 0.0
                && (current_residual - previous_residual).abs() < PI)
                || (current_residual == 0.0 && previous_residual != 0.0);
            if crosses_target {
                let root = BracketedRootSearch::refine(
                    previous_epoch,
                    current_epoch,
                    self.options.time_tolerance(),
                    self.options.max_refinement_iterations(),
                    |epoch| {
                        let observation = self.evaluate(epoch)?;
                        self.residual(observation)
                    },
                )?;
                let observation = self.evaluate(root.instant())?;
                let residual = self.residual(observation)?;
                if residual.abs() > self.options.angular_tolerance().as_radians() {
                    return Err(Error::AngularResidualExceeded {
                        event: "relative-coordinate configuration",
                        residual_radians: residual.abs(),
                        tolerance_radians: self.options.angular_tolerance().as_radians(),
                    });
                }
                let event = ConfigurationEvent::new(
                    self.query,
                    self.sampler.origin(),
                    observation,
                    EventEvidence::new(
                        root.bracket_start(),
                        root.bracket_end(),
                        root.time_uncertainty(),
                        Angle::from_radians(residual)?,
                        root.iterations(),
                        self.evaluations,
                    ),
                );
                Self::push_unique(&mut events, event, self.options.time_tolerance())?;
            } else if current_epoch == self.interval.end()
                && current_residual.abs() <= self.options.angular_tolerance().as_radians()
            {
                let event = ConfigurationEvent::new(
                    self.query,
                    self.sampler.origin(),
                    current,
                    EventEvidence::new(
                        current_epoch,
                        current_epoch,
                        Duration::ZERO,
                        Angle::from_radians(current_residual)?,
                        0,
                        self.evaluations,
                    ),
                );
                Self::push_unique(&mut events, event, self.options.time_tolerance())?;
            }
            previous_epoch = current_epoch;
            previous_residual = current_residual;
        }
        Ok(events)
    }

    fn evaluate(&mut self, epoch: Instant<S>) -> Result<RelativeObservation<S>, Error> {
        self.sampler.sample(
            self.query.bodies().target(),
            self.query.bodies().reference(),
            epoch,
            &mut self.evaluations,
            self.options.max_evaluations(),
        )
    }

    fn residual(&self, observation: RelativeObservation<S>) -> Result<f64, Error> {
        let actual = match self.query.coordinate() {
            ConfigurationCoordinate::EclipticLongitude => observation.longitude_difference(),
            ConfigurationCoordinate::RightAscension => observation.right_ascension_difference(),
        };
        Angle::wrap_signed(
            actual.as_radians() - self.query.kind().target_angle().as_radians(),
            "relative-configuration residual",
        )
        .map_err(Error::from)
    }

    fn push_unique(
        events: &mut Vec<ConfigurationEvent<S>>,
        candidate: ConfigurationEvent<S>,
        tolerance: Duration,
    ) -> Result<(), Error> {
        if let Some(previous) = events.last_mut()
            && candidate
                .instant()
                .duration_since(previous.instant())?
                .checked_abs()?
                <= tolerance
        {
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

struct ElongationSearch<S: TimeScale, R> {
    interval: TimeInterval<S>,
    query: RelativeBodyQuery,
    options: ExtremumSearchOptions,
    sampler: R,
}

impl<S: TimeScale, R: RelativeSampler<S>> ElongationSearch<S, R> {
    const fn new(
        interval: TimeInterval<S>,
        query: RelativeBodyQuery,
        options: ExtremumSearchOptions,
        sampler: R,
    ) -> Self {
        Self {
            interval,
            query,
            options,
            sampler,
        }
    }

    fn events(self) -> Result<Vec<GreatestElongationEvent<S>>, Error> {
        let origin = self.sampler.origin();
        SampledExtremumSearch::new(self.interval.start(), self.interval.end(), self.options)
            .search(ExtremumSense::Maximum, |epoch, evaluations| {
                let observation = self.sampler.sample(
                    self.query.target(),
                    self.query.reference(),
                    epoch,
                    evaluations,
                    self.options.max_evaluations(),
                )?;
                Ok((observation.separation().as_radians(), observation))
            })?
            .into_iter()
            .map(|located| {
                let (observation, evidence) = located.into_parts();
                GreatestElongationEvent::new(self.query, origin, observation, evidence)
            })
            .collect()
    }
}

struct StationSearch<S: TimeScale, R> {
    interval: TimeInterval<S>,
    query: StationQuery,
    options: AngularEventSearchOptions,
    sampler: R,
    evaluations: u32,
}

impl<S: TimeScale, R: RelativeSampler<S>> StationSearch<S, R> {
    const RATE_STENCIL: Duration = Duration::from_nanoseconds(30 * 60 * 1_000_000_000);

    const fn new(
        interval: TimeInterval<S>,
        query: StationQuery,
        options: AngularEventSearchOptions,
        sampler: R,
    ) -> Self {
        Self {
            interval,
            query,
            options,
            sampler,
            evaluations: 0,
        }
    }

    fn events(mut self) -> Result<Vec<StationEvent<S>>, Error> {
        let mut previous_epoch = self.interval.start();
        let mut previous_rate = self.longitude_rate(previous_epoch)?;
        let mut events = Vec::new();
        while previous_epoch < self.interval.end() {
            let remaining = self.interval.end().duration_since(previous_epoch)?;
            let step = remaining.min(self.options.scan_step());
            let current_epoch = previous_epoch.checked_add(step)?;
            let current_rate = self.longitude_rate(current_epoch)?;
            if previous_rate * current_rate < 0.0 || (current_rate == 0.0 && previous_rate != 0.0) {
                let kind = if previous_rate > 0.0 {
                    StationKind::Retrograde
                } else {
                    StationKind::Direct
                };
                let root = BracketedRootSearch::refine(
                    previous_epoch,
                    current_epoch,
                    self.options.time_tolerance(),
                    self.options.max_refinement_iterations(),
                    |epoch| self.longitude_rate(epoch),
                )?;
                let residual_rate =
                    AngularSpeed::from_radians_per_second(self.longitude_rate(root.instant())?)?;
                let position = self.sampler.position(
                    self.query.body(),
                    root.instant(),
                    &mut self.evaluations,
                    self.options.max_evaluations(),
                )?;
                let event = StationEvent {
                    query: self.query,
                    origin: self.sampler.origin(),
                    kind,
                    position,
                    evidence: StationEvidence::new(
                        root.bracket_start(),
                        root.bracket_end(),
                        root.time_uncertainty(),
                        residual_rate,
                        root.iterations(),
                        self.evaluations,
                    ),
                };
                Self::push_unique(&mut events, event, self.options.time_tolerance())?;
            }
            previous_epoch = current_epoch;
            previous_rate = current_rate;
        }
        Ok(events)
    }

    fn longitude_rate(&mut self, epoch: Instant<S>) -> Result<f64, Error> {
        let lower = match epoch.checked_sub(Self::RATE_STENCIL) {
            Ok(candidate) if candidate >= self.interval.start() => candidate,
            _ => self.interval.start(),
        };
        let upper = match epoch.checked_add(Self::RATE_STENCIL) {
            Ok(candidate) if candidate <= self.interval.end() => candidate,
            _ => self.interval.end(),
        };
        let lower_position = self.sampler.position(
            self.query.body(),
            lower,
            &mut self.evaluations,
            self.options.max_evaluations(),
        )?;
        let upper_position = self.sampler.position(
            self.query.body(),
            upper,
            &mut self.evaluations,
            self.options.max_evaluations(),
        )?;
        let lower_longitude = lower_position
            .true_ecliptic()
            .coordinates()
            .longitude()
            .as_radians();
        let upper_longitude = upper_position
            .true_ecliptic()
            .coordinates()
            .longitude()
            .as_radians();
        let change = Angle::wrap_signed(
            upper_longitude - lower_longitude,
            "station longitude derivative",
        )?;
        let elapsed = upper.duration_since(lower)?.as_seconds_f64();
        Ok(change / elapsed)
    }

    fn push_unique(
        events: &mut Vec<StationEvent<S>>,
        candidate: StationEvent<S>,
        tolerance: Duration,
    ) -> Result<(), Error> {
        if let Some(previous) = events.last_mut()
            && candidate
                .instant()
                .duration_since(previous.instant())?
                .checked_abs()?
                <= tolerance
        {
            if candidate
                .evidence()
                .residual_rate()
                .as_radians_per_second()
                .abs()
                < previous
                    .evidence()
                    .residual_rate()
                    .as_radians_per_second()
                    .abs()
            {
                *previous = candidate;
            }
            return Ok(());
        }
        events.push(candidate);
        Ok(())
    }
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Events<'context, 'data, E, P> {
    /// Finds all geocentric conjunction, opposition, or quadrature events in a closed interval.
    pub fn configurations_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        query: ConfigurationQuery,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<ConfigurationEvent<S>>, Error> {
        let sampler = GeocentricRelativeSampler::new(
            self.astrometry,
            query.bodies().mode(),
            options.light_time(),
        );
        ConfigurationSearch::new(interval, query, options, sampler).events()
    }

    /// Finds all geocentric local maxima of angular separation in a closed interval.
    pub fn greatest_elongations_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        query: RelativeBodyQuery,
        options: ExtremumSearchOptions,
    ) -> Result<Vec<GreatestElongationEvent<S>>, Error> {
        let sampler =
            GeocentricRelativeSampler::new(self.astrometry, query.mode(), options.light_time());
        ElongationSearch::new(interval, query, options, sampler).events()
    }

    /// Finds all geocentric transitions between prograde and retrograde longitude motion.
    pub fn stations_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        query: StationQuery,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<StationEvent<S>>, Error> {
        let sampler =
            GeocentricRelativeSampler::new(self.astrometry, query.mode(), options.light_time());
        StationSearch::new(interval, query, options, sampler).events()
    }
}

impl<'context, 'data, 'eop, P: EphemerisProvider + ?Sized>
    Events<'context, 'data, EarthOrientationTable<'eop>, P>
{
    /// Finds all fixed-site conjunction, opposition, or quadrature events in a closed interval.
    pub fn fixed_site_configurations_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        interval: TimeInterval<S>,
        query: ConfigurationQuery,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<ConfigurationEvent<S>>, Error> {
        let sampler = FixedSiteRelativeSampler::new(
            self.astrometry,
            site,
            query.bodies().mode(),
            options.light_time(),
        );
        ConfigurationSearch::new(interval, query, options, sampler).events()
    }

    /// Finds all fixed-site local maxima of angular separation in a closed interval.
    pub fn fixed_site_greatest_elongations_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        interval: TimeInterval<S>,
        query: RelativeBodyQuery,
        options: ExtremumSearchOptions,
    ) -> Result<Vec<GreatestElongationEvent<S>>, Error> {
        let sampler = FixedSiteRelativeSampler::new(
            self.astrometry,
            site,
            query.mode(),
            options.light_time(),
        );
        ElongationSearch::new(interval, query, options, sampler).events()
    }

    /// Finds all fixed-site transitions between prograde and retrograde longitude motion.
    pub fn fixed_site_stations_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        interval: TimeInterval<S>,
        query: StationQuery,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<StationEvent<S>>, Error> {
        let sampler = FixedSiteRelativeSampler::new(
            self.astrometry,
            site,
            query.mode(),
            options.light_time(),
        );
        StationSearch::new(interval, query, options, sampler).events()
    }
}
