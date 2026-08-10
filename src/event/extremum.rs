use core::fmt;
use std::vec::Vec;

use crate::{
    earth::FixedSite,
    ephem::{CelestialBody, EphemerisProvider, EphemerisQuery, RelativeState},
    frame::{Bcrs, EclipticLatitude},
    math::{Angle, Declination, Length, Separation},
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

/// Selects a local minimum or local maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExtremumKind {
    /// A local minimum.
    Minimum,
    /// A local maximum.
    Maximum,
}

impl ExtremumKind {
    fn sense(self) -> ExtremumSense {
        match self {
            Self::Minimum => ExtremumSense::Minimum,
            Self::Maximum => ExtremumSense::Maximum,
        }
    }
}

/// A query for local extrema of the true angular separation between two bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AngularSeparationExtremumQuery {
    bodies: RelativeBodyQuery,
    kind: ExtremumKind,
}

impl AngularSeparationExtremumQuery {
    /// Constructs an angular-separation extremum query.
    pub const fn new(bodies: RelativeBodyQuery, kind: ExtremumKind) -> Self {
        Self { bodies, kind }
    }

    /// Returns the ordered body pair and astrometric semantics.
    pub const fn bodies(self) -> RelativeBodyQuery {
        self.bodies
    }

    /// Returns whether local minima or maxima are requested.
    pub const fn kind(self) -> ExtremumKind {
        self.kind
    }
}

/// One local angular-separation extremum.
pub struct AngularSeparationExtremumEvent<S: TimeScale> {
    query: AngularSeparationExtremumQuery,
    origin: ObservationOrigin,
    target: EventBodyPosition<S>,
    reference: EventBodyPosition<S>,
    separation: Separation,
    evidence: ExtremumEvidence<S>,
}

impl<S: TimeScale> AngularSeparationExtremumEvent<S> {
    fn new(
        query: AngularSeparationExtremumQuery,
        origin: ObservationOrigin,
        observation: RelativeObservation<S>,
        evidence: ExtremumEvidence<S>,
    ) -> Self {
        Self {
            query,
            origin,
            target: observation.target(),
            reference: observation.reference(),
            separation: observation.separation(),
            evidence,
        }
    }

    /// Returns the complete defining query.
    pub const fn query(self) -> AngularSeparationExtremumQuery {
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

    /// Returns the evaluated target position.
    pub const fn target(self) -> EventBodyPosition<S> {
        self.target
    }

    /// Returns the evaluated reference position.
    pub const fn reference(self) -> EventBodyPosition<S> {
        self.reference
    }

    /// Returns the locally extremal great-circle separation.
    pub const fn separation(self) -> Separation {
        self.separation
    }

    /// Returns numerical bounded-extremum evidence.
    pub const fn evidence(self) -> ExtremumEvidence<S> {
        self.evidence
    }
}

impl<S: TimeScale> Copy for AngularSeparationExtremumEvent<S> {}

impl<S: TimeScale> Clone for AngularSeparationExtremumEvent<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for AngularSeparationExtremumEvent<S> {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query
            && self.origin == other.origin
            && self.target == other.target
            && self.reference == other.reference
            && self.separation == other.separation
            && self.evidence == other.evidence
    }
}

impl<S: TimeScale> fmt::Debug for AngularSeparationExtremumEvent<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AngularSeparationExtremumEvent")
            .field("query", &self.query)
            .field("origin", &self.origin)
            .field("target", &self.target)
            .field("reference", &self.reference)
            .field("separation", &self.separation)
            .field("evidence", &self.evidence)
            .finish()
    }
}

/// A geometric target-centre distance extremum query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DistanceExtremumQuery {
    target: CelestialBody,
    center: CelestialBody,
    kind: ExtremumKind,
}

impl DistanceExtremumQuery {
    /// Constructs a geometric distance query for two distinct bodies.
    pub fn new(
        target: CelestialBody,
        center: CelestialBody,
        kind: ExtremumKind,
    ) -> Result<Self, Error> {
        if target == center {
            return Err(Error::IdenticalEventBodies { body: target });
        }
        Ok(Self {
            target,
            center,
            kind,
        })
    }

    /// Returns the body whose centre-to-centre state is evaluated.
    pub const fn target(self) -> CelestialBody {
        self.target
    }

    /// Returns the centre from which geometric distance is measured.
    pub const fn center(self) -> CelestialBody {
        self.center
    }

    /// Returns whether local minima or maxima are requested.
    pub const fn kind(self) -> ExtremumKind {
        self.kind
    }
}

/// One local geometric target-centre distance extremum.
pub struct DistanceExtremumEvent<S: TimeScale> {
    query: DistanceExtremumQuery,
    state: RelativeState<Bcrs, S>,
    distance: Length,
    evidence: ExtremumEvidence<S>,
}

impl<S: TimeScale> DistanceExtremumEvent<S> {
    fn new(
        query: DistanceExtremumQuery,
        state: RelativeState<Bcrs, S>,
        evidence: ExtremumEvidence<S>,
    ) -> Result<Self, Error> {
        let distance = state.position().magnitude()?;
        Ok(Self {
            query,
            state,
            distance,
            evidence,
        })
    }

    /// Returns the complete defining query.
    pub const fn query(self) -> DistanceExtremumQuery {
        self.query
    }

    /// Returns the geometric extremum instant.
    pub const fn instant(self) -> Instant<S> {
        self.state.epoch()
    }

    /// Returns the full geometric target-centre state at the extremum.
    pub const fn state(self) -> RelativeState<Bcrs, S> {
        self.state
    }

    /// Returns the locally extremal centre-to-centre distance.
    pub const fn distance(self) -> Length {
        self.distance
    }

    /// Returns numerical bounded-extremum evidence.
    pub const fn evidence(self) -> ExtremumEvidence<S> {
        self.evidence
    }
}

impl<S: TimeScale> Copy for DistanceExtremumEvent<S> {}

impl<S: TimeScale> Clone for DistanceExtremumEvent<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for DistanceExtremumEvent<S> {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query
            && self.state == other.state
            && self.distance == other.distance
            && self.evidence == other.evidence
    }
}

impl<S: TimeScale> fmt::Debug for DistanceExtremumEvent<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DistanceExtremumEvent")
            .field("query", &self.query)
            .field("state", &self.state)
            .field("distance", &self.distance)
            .field("evidence", &self.evidence)
            .finish()
    }
}

/// Signed angular coordinate used for extrema and zero crossings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EventCoordinate {
    /// Latitude on the true ecliptic and equinox of date.
    EclipticLatitude,
    /// Declination on the true equator and equinox of date.
    Declination,
}

/// A typed signed coordinate value retained at an event.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum EventCoordinateValue {
    /// True-ecliptic latitude.
    EclipticLatitude(EclipticLatitude),
    /// True-equatorial declination.
    Declination(Declination),
}

impl EventCoordinateValue {
    /// Returns the signed coordinate in radians.
    pub const fn as_radians(self) -> f64 {
        match self {
            Self::EclipticLatitude(value) => value.as_radians(),
            Self::Declination(value) => value.as_radians(),
        }
    }
}

/// A query for local extrema of one body's signed angular coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoordinateExtremumQuery {
    body: CelestialBody,
    mode: AstrometricMode,
    coordinate: EventCoordinate,
    kind: ExtremumKind,
}

impl CoordinateExtremumQuery {
    /// Constructs a coordinate-extremum query.
    pub const fn new(
        body: CelestialBody,
        mode: AstrometricMode,
        coordinate: EventCoordinate,
        kind: ExtremumKind,
    ) -> Self {
        Self {
            body,
            mode,
            coordinate,
            kind,
        }
    }

    /// Returns the evaluated body.
    pub const fn body(self) -> CelestialBody {
        self.body
    }

    /// Returns the selected geometric or apparent-place semantics.
    pub const fn mode(self) -> AstrometricMode {
        self.mode
    }

    /// Returns the signed coordinate being extremized.
    pub const fn coordinate(self) -> EventCoordinate {
        self.coordinate
    }

    /// Returns whether local minima or maxima are requested.
    pub const fn kind(self) -> ExtremumKind {
        self.kind
    }
}

/// One local ecliptic-latitude or declination extremum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinateExtremumEvent<S: TimeScale> {
    query: CoordinateExtremumQuery,
    origin: ObservationOrigin,
    position: EventBodyPosition<S>,
    value: EventCoordinateValue,
    evidence: ExtremumEvidence<S>,
}

impl<S: TimeScale> CoordinateExtremumEvent<S> {
    /// Returns the complete defining query.
    pub const fn query(self) -> CoordinateExtremumQuery {
        self.query
    }

    /// Returns the extremum instant.
    pub const fn instant(self) -> Instant<S> {
        self.position.epoch()
    }

    /// Returns whether the event was evaluated at the geocentre or a fixed site.
    pub const fn origin(self) -> ObservationOrigin {
        self.origin
    }

    /// Returns the evaluated body position.
    pub const fn position(self) -> EventBodyPosition<S> {
        self.position
    }

    /// Returns the typed locally extremal coordinate.
    pub const fn value(self) -> EventCoordinateValue {
        self.value
    }

    /// Returns numerical bounded-extremum evidence.
    pub const fn evidence(self) -> ExtremumEvidence<S> {
        self.evidence
    }
}

/// Direction in which a signed coordinate crosses zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CoordinateCrossingKind {
    /// The coordinate changes from negative to positive.
    Ascending,
    /// The coordinate changes from positive to negative.
    Descending,
}

/// A query for zero crossings of one body's signed angular coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoordinateCrossingQuery {
    body: CelestialBody,
    mode: AstrometricMode,
    coordinate: EventCoordinate,
}

impl CoordinateCrossingQuery {
    /// Constructs a coordinate-crossing query.
    pub const fn new(
        body: CelestialBody,
        mode: AstrometricMode,
        coordinate: EventCoordinate,
    ) -> Self {
        Self {
            body,
            mode,
            coordinate,
        }
    }

    /// Returns the evaluated body.
    pub const fn body(self) -> CelestialBody {
        self.body
    }

    /// Returns the selected geometric or apparent-place semantics.
    pub const fn mode(self) -> AstrometricMode {
        self.mode
    }

    /// Returns the signed coordinate whose zero crossing is requested.
    pub const fn coordinate(self) -> EventCoordinate {
        self.coordinate
    }
}

/// One ascending or descending ecliptic/equatorial crossing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinateCrossingEvent<S: TimeScale> {
    query: CoordinateCrossingQuery,
    origin: ObservationOrigin,
    kind: CoordinateCrossingKind,
    position: EventBodyPosition<S>,
    value: EventCoordinateValue,
    evidence: EventEvidence<S>,
}

impl<S: TimeScale> CoordinateCrossingEvent<S> {
    /// Returns the complete defining query.
    pub const fn query(self) -> CoordinateCrossingQuery {
        self.query
    }

    /// Returns the crossing instant.
    pub const fn instant(self) -> Instant<S> {
        self.position.epoch()
    }

    /// Returns whether the event was evaluated at the geocentre or a fixed site.
    pub const fn origin(self) -> ObservationOrigin {
        self.origin
    }

    /// Returns whether the coordinate crossed northward or southward.
    pub const fn kind(self) -> CoordinateCrossingKind {
        self.kind
    }

    /// Returns the evaluated body position.
    pub const fn position(self) -> EventBodyPosition<S> {
        self.position
    }

    /// Returns the refined signed coordinate value.
    pub const fn value(self) -> EventCoordinateValue {
        self.value
    }

    /// Returns numerical root-search evidence.
    pub const fn evidence(self) -> EventEvidence<S> {
        self.evidence
    }
}

struct SeparationExtremumSearch<S: TimeScale, R> {
    interval: TimeInterval<S>,
    query: AngularSeparationExtremumQuery,
    options: ExtremumSearchOptions,
    sampler: R,
}

impl<S: TimeScale, R: RelativeSampler<S>> SeparationExtremumSearch<S, R> {
    const fn new(
        interval: TimeInterval<S>,
        query: AngularSeparationExtremumQuery,
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

    fn events(self) -> Result<Vec<AngularSeparationExtremumEvent<S>>, Error> {
        let origin = self.sampler.origin();
        Ok(
            SampledExtremumSearch::new(self.interval.start(), self.interval.end(), self.options)
                .search(self.query.kind().sense(), |epoch, evaluations| {
                    let observation = self.sampler.sample(
                        self.query.bodies().target(),
                        self.query.bodies().reference(),
                        epoch,
                        evaluations,
                        self.options.max_evaluations(),
                    )?;
                    Ok((observation.separation().as_radians(), observation))
                })?
                .into_iter()
                .map(|located| {
                    let (observation, evidence) = located.into_parts();
                    AngularSeparationExtremumEvent::new(self.query, origin, observation, evidence)
                })
                .collect(),
        )
    }
}

struct DistanceExtremumSearch<'ephemeris, S: TimeScale, P: EphemerisProvider + ?Sized> {
    ephemeris: &'ephemeris P,
    interval: TimeInterval<S>,
    query: DistanceExtremumQuery,
    options: ExtremumSearchOptions,
}

impl<S: TimeScale, P: EphemerisProvider + ?Sized> DistanceExtremumSearch<'_, S, P> {
    fn events(self) -> Result<Vec<DistanceExtremumEvent<S>>, Error> {
        let maximum = self.options.max_evaluations();
        SampledExtremumSearch::new(self.interval.start(), self.interval.end(), self.options)
            .search(self.query.kind().sense(), |epoch, evaluations| {
                if *evaluations >= maximum {
                    return Err(Error::EvaluationLimitExceeded { maximum });
                }
                *evaluations += 1;
                let state = self
                    .ephemeris
                    .state(EphemerisQuery::new(
                        self.query.target(),
                        self.query.center(),
                        epoch,
                    ))
                    .map_err(crate::astro::Error::from)?;
                Ok((state.position().magnitude()?.as_metres(), state))
            })?
            .into_iter()
            .map(|located| {
                let (state, evidence) = located.into_parts();
                DistanceExtremumEvent::new(self.query, state, evidence)
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

struct CoordinateExtremumSearch<S: TimeScale, R> {
    interval: TimeInterval<S>,
    query: CoordinateExtremumQuery,
    options: ExtremumSearchOptions,
    sampler: R,
}

impl<S: TimeScale, R: RelativeSampler<S>> CoordinateExtremumSearch<S, R> {
    fn events(self) -> Result<Vec<CoordinateExtremumEvent<S>>, Error> {
        let origin = self.sampler.origin();
        SampledExtremumSearch::new(self.interval.start(), self.interval.end(), self.options)
            .search(self.query.kind().sense(), |epoch, evaluations| {
                let position = self.sampler.position(
                    self.query.body(),
                    epoch,
                    evaluations,
                    self.options.max_evaluations(),
                )?;
                Ok((
                    Self::coordinate_radians(self.query.coordinate(), position),
                    position,
                ))
            })?
            .into_iter()
            .map(|located| {
                let (position, evidence) = located.into_parts();
                Ok(CoordinateExtremumEvent {
                    query: self.query,
                    origin,
                    value: Self::coordinate_value(self.query.coordinate(), position)?,
                    position,
                    evidence,
                })
            })
            .collect::<Result<Vec<_>, Error>>()
    }

    fn coordinate_radians(coordinate: EventCoordinate, position: EventBodyPosition<S>) -> f64 {
        match coordinate {
            EventCoordinate::EclipticLatitude => position
                .true_ecliptic()
                .coordinates()
                .latitude()
                .as_radians(),
            EventCoordinate::Declination => position
                .true_equatorial()
                .coordinates()
                .declination()
                .as_radians(),
        }
    }

    fn coordinate_value(
        coordinate: EventCoordinate,
        position: EventBodyPosition<S>,
    ) -> Result<EventCoordinateValue, Error> {
        Ok(match coordinate {
            EventCoordinate::EclipticLatitude => EventCoordinateValue::EclipticLatitude(
                position.true_ecliptic().coordinates().latitude(),
            ),
            EventCoordinate::Declination => EventCoordinateValue::Declination(
                position.true_equatorial().coordinates().declination(),
            ),
        })
    }
}

struct CoordinateCrossingSearch<S: TimeScale, R> {
    interval: TimeInterval<S>,
    query: CoordinateCrossingQuery,
    options: AngularEventSearchOptions,
    sampler: R,
    evaluations: u32,
}

impl<S: TimeScale, R: RelativeSampler<S>> CoordinateCrossingSearch<S, R> {
    fn events(mut self) -> Result<Vec<CoordinateCrossingEvent<S>>, Error> {
        let mut previous_epoch = self.interval.start();
        let previous_position = self.evaluate(previous_epoch)?;
        let mut previous_value = self.coordinate_radians(previous_position);
        let mut events = Vec::new();
        while previous_epoch < self.interval.end() {
            let remaining = self.interval.end().duration_since(previous_epoch)?;
            let step = remaining.min(self.options.scan_step());
            let current_epoch = previous_epoch.checked_add(step)?;
            let current_position = self.evaluate(current_epoch)?;
            let current_value = self.coordinate_radians(current_position);
            if previous_value * current_value < 0.0
                || (current_value == 0.0 && previous_value != 0.0)
            {
                let kind = if previous_value < 0.0 {
                    CoordinateCrossingKind::Ascending
                } else {
                    CoordinateCrossingKind::Descending
                };
                let root = BracketedRootSearch::refine(
                    previous_epoch,
                    current_epoch,
                    self.options.time_tolerance(),
                    self.options.max_refinement_iterations(),
                    |epoch| {
                        let position = self.evaluate(epoch)?;
                        Ok(self.coordinate_radians(position))
                    },
                )?;
                let position = self.evaluate(root.instant())?;
                let residual = self.coordinate_radians(position);
                if residual.abs() > self.options.angular_tolerance().as_radians() {
                    return Err(Error::AngularResidualExceeded {
                        event: "signed-coordinate crossing",
                        residual_radians: residual.abs(),
                        tolerance_radians: self.options.angular_tolerance().as_radians(),
                    });
                }
                let event = CoordinateCrossingEvent {
                    query: self.query,
                    origin: self.sampler.origin(),
                    kind,
                    position,
                    value: self.coordinate_value(position),
                    evidence: EventEvidence::new(
                        root.bracket_start(),
                        root.bracket_end(),
                        root.time_uncertainty(),
                        Angle::from_radians(residual)?,
                        root.iterations(),
                        self.evaluations,
                    ),
                };
                Self::push_unique(&mut events, event, self.options.time_tolerance())?;
            }
            previous_epoch = current_epoch;
            previous_value = current_value;
        }
        Ok(events)
    }

    fn evaluate(&mut self, epoch: Instant<S>) -> Result<EventBodyPosition<S>, Error> {
        self.sampler.position(
            self.query.body(),
            epoch,
            &mut self.evaluations,
            self.options.max_evaluations(),
        )
    }

    fn coordinate_radians(&self, position: EventBodyPosition<S>) -> f64 {
        match self.query.coordinate() {
            EventCoordinate::EclipticLatitude => position
                .true_ecliptic()
                .coordinates()
                .latitude()
                .as_radians(),
            EventCoordinate::Declination => position
                .true_equatorial()
                .coordinates()
                .declination()
                .as_radians(),
        }
    }

    fn coordinate_value(&self, position: EventBodyPosition<S>) -> EventCoordinateValue {
        match self.query.coordinate() {
            EventCoordinate::EclipticLatitude => EventCoordinateValue::EclipticLatitude(
                position.true_ecliptic().coordinates().latitude(),
            ),
            EventCoordinate::Declination => EventCoordinateValue::Declination(
                position.true_equatorial().coordinates().declination(),
            ),
        }
    }

    fn push_unique(
        events: &mut Vec<CoordinateCrossingEvent<S>>,
        candidate: CoordinateCrossingEvent<S>,
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

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Events<'context, 'data, E, P> {
    /// Finds all geocentric local minima or maxima of true angular separation.
    pub fn angular_separation_extrema_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        query: AngularSeparationExtremumQuery,
        options: ExtremumSearchOptions,
    ) -> Result<Vec<AngularSeparationExtremumEvent<S>>, Error> {
        let sampler = GeocentricRelativeSampler::new(
            self.astrometry,
            query.bodies().mode(),
            options.light_time(),
        );
        SeparationExtremumSearch::new(interval, query, options, sampler).events()
    }

    /// Finds all local geometric centre-to-centre distance extrema.
    pub fn distance_extrema_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        query: DistanceExtremumQuery,
        options: ExtremumSearchOptions,
    ) -> Result<Vec<DistanceExtremumEvent<S>>, Error> {
        DistanceExtremumSearch {
            ephemeris: self.astrometry.ephemeris(),
            interval,
            query,
            options,
        }
        .events()
    }

    /// Finds all geocentric local extrema of true-ecliptic latitude or true declination.
    pub fn coordinate_extrema_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        query: CoordinateExtremumQuery,
        options: ExtremumSearchOptions,
    ) -> Result<Vec<CoordinateExtremumEvent<S>>, Error> {
        let sampler =
            GeocentricRelativeSampler::new(self.astrometry, query.mode(), options.light_time());
        CoordinateExtremumSearch {
            interval,
            query,
            options,
            sampler,
        }
        .events()
    }

    /// Finds all geocentric ascending and descending ecliptic or equatorial crossings.
    pub fn coordinate_crossings_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        query: CoordinateCrossingQuery,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<CoordinateCrossingEvent<S>>, Error> {
        let sampler =
            GeocentricRelativeSampler::new(self.astrometry, query.mode(), options.light_time());
        CoordinateCrossingSearch {
            interval,
            query,
            options,
            sampler,
            evaluations: 0,
        }
        .events()
    }
}

impl<'context, 'data, 'eop, P: EphemerisProvider + ?Sized>
    Events<'context, 'data, EarthOrientationTable<'eop>, P>
{
    /// Finds all fixed-site local minima or maxima of true angular separation.
    pub fn fixed_site_angular_separation_extrema_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        interval: TimeInterval<S>,
        query: AngularSeparationExtremumQuery,
        options: ExtremumSearchOptions,
    ) -> Result<Vec<AngularSeparationExtremumEvent<S>>, Error> {
        let sampler = FixedSiteRelativeSampler::new(
            self.astrometry,
            site,
            query.bodies().mode(),
            options.light_time(),
        );
        SeparationExtremumSearch::new(interval, query, options, sampler).events()
    }

    /// Finds all fixed-site local extrema of true-ecliptic latitude or true declination.
    pub fn fixed_site_coordinate_extrema_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        interval: TimeInterval<S>,
        query: CoordinateExtremumQuery,
        options: ExtremumSearchOptions,
    ) -> Result<Vec<CoordinateExtremumEvent<S>>, Error> {
        let sampler = FixedSiteRelativeSampler::new(
            self.astrometry,
            site,
            query.mode(),
            options.light_time(),
        );
        CoordinateExtremumSearch {
            interval,
            query,
            options,
            sampler,
        }
        .events()
    }

    /// Finds all fixed-site ascending and descending ecliptic or equatorial crossings.
    pub fn fixed_site_coordinate_crossings_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        interval: TimeInterval<S>,
        query: CoordinateCrossingQuery,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<CoordinateCrossingEvent<S>>, Error> {
        let sampler = FixedSiteRelativeSampler::new(
            self.astrometry,
            site,
            query.mode(),
            options.light_time(),
        );
        CoordinateCrossingSearch {
            interval,
            query,
            options,
            sampler,
            evaluations: 0,
        }
        .events()
    }
}
