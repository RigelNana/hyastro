use crate::{
    astro::{Astrometry, ReceptionLightTimeOptions},
    earth::FixedSite,
    ephem::{CelestialBody, EphemerisQuery},
    frame::{
        Bcrs, EclipticDirectionAt, EquatorialDirection, EquatorialDirectionAt, Frames, Gcrs,
        TrueEclipticEquinoxOfDate, TrueEquatorEquinoxOfDate,
    },
    math::{Angle, Direction, Length, Separation},
    time::{EarthOrientationTable, Instant, TimeScale},
};

use super::Error;

/// Selects whether an event criterion uses simultaneous geometry or reception-time apparent places.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AstrometricMode {
    /// Evaluates both bodies at the common event epoch without light-time or astrometric corrections.
    Geometric,
    /// Uses converged reception light time, solar deflection, aberration, and orientation corrections.
    Apparent,
}

/// A validated pair of distinct bodies and the astrometric semantics used to compare them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RelativeBodyQuery {
    target: CelestialBody,
    reference: CelestialBody,
    mode: AstrometricMode,
}

impl RelativeBodyQuery {
    /// Constructs a relative-body query with an explicit target-minus-reference ordering.
    pub fn new(
        target: CelestialBody,
        reference: CelestialBody,
        mode: AstrometricMode,
    ) -> Result<Self, Error> {
        if target == reference {
            return Err(Error::IdenticalEventBodies { body: target });
        }
        Ok(Self {
            target,
            reference,
            mode,
        })
    }

    /// Returns the body whose coordinates form the left side of relative differences.
    pub const fn target(self) -> CelestialBody {
        self.target
    }

    /// Returns the body whose coordinates form the right side of relative differences.
    pub const fn reference(self) -> CelestialBody {
        self.reference
    }

    /// Returns the selected geometric or apparent-place semantics.
    pub const fn mode(self) -> AstrometricMode {
        self.mode
    }
}
/// Identifies the physical origin from which event directions and distances were evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ObservationOrigin {
    /// The centre of the Earth.
    Geocenter,
    /// A caller-supplied fixed terrestrial site.
    FixedSite,
}

/// One body's fully typed direction and range retained at an event instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventBodyPosition<S: TimeScale> {
    body: CelestialBody,
    epoch: Instant<S>,
    true_equatorial: EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>,
    true_ecliptic: EclipticDirectionAt<TrueEclipticEquinoxOfDate, S>,
    distance: Length,
}

impl<S: TimeScale> EventBodyPosition<S> {
    pub(super) const fn new(
        body: CelestialBody,
        epoch: Instant<S>,
        true_equatorial: EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>,
        true_ecliptic: EclipticDirectionAt<TrueEclipticEquinoxOfDate, S>,
        distance: Length,
    ) -> Self {
        Self {
            body,
            epoch,
            true_equatorial,
            true_ecliptic,
            distance,
        }
    }

    /// Returns the represented solar-system body.
    pub const fn body(self) -> CelestialBody {
        self.body
    }

    /// Returns the common observation or geometric epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the direction on the true equator and equinox of date.
    pub const fn true_equatorial(self) -> EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S> {
        self.true_equatorial
    }

    /// Returns the direction on the true ecliptic and equinox of date.
    pub const fn true_ecliptic(self) -> EclipticDirectionAt<TrueEclipticEquinoxOfDate, S> {
        self.true_ecliptic
    }

    /// Returns the body-to-observer range used by the selected astrometric mode.
    pub const fn distance(self) -> Length {
        self.distance
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RelativeObservation<S: TimeScale> {
    target: EventBodyPosition<S>,
    reference: EventBodyPosition<S>,
    longitude_difference: Angle,
    right_ascension_difference: Angle,
    separation: Separation,
}

impl<S: TimeScale> RelativeObservation<S> {
    fn new(target: EventBodyPosition<S>, reference: EventBodyPosition<S>) -> Result<Self, Error> {
        let longitude_difference = Angle::from_radians(Angle::wrap_zero_tau(
            target
                .true_ecliptic()
                .coordinates()
                .longitude()
                .as_radians()
                - reference
                    .true_ecliptic()
                    .coordinates()
                    .longitude()
                    .as_radians(),
            "target-minus-reference ecliptic longitude",
        )?)?;
        let right_ascension_difference = Angle::from_radians(Angle::wrap_zero_tau(
            target
                .true_equatorial()
                .coordinates()
                .right_ascension()
                .as_radians()
                - reference
                    .true_equatorial()
                    .coordinates()
                    .right_ascension()
                    .as_radians(),
            "target-minus-reference right ascension",
        )?)?;
        let separation = target
            .true_equatorial()
            .coordinates()
            .separation_to(reference.true_equatorial().coordinates())?;
        Ok(Self {
            target,
            reference,
            longitude_difference,
            right_ascension_difference,
            separation,
        })
    }

    pub(super) const fn target(self) -> EventBodyPosition<S> {
        self.target
    }

    pub(super) const fn reference(self) -> EventBodyPosition<S> {
        self.reference
    }

    pub(super) const fn longitude_difference(self) -> Angle {
        self.longitude_difference
    }

    pub(super) const fn right_ascension_difference(self) -> Angle {
        self.right_ascension_difference
    }

    pub(super) const fn separation(self) -> Separation {
        self.separation
    }
}

pub(super) trait RelativeSampler<S: TimeScale> {
    fn origin(&self) -> ObservationOrigin;

    fn position(
        &self,
        body: CelestialBody,
        epoch: Instant<S>,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<EventBodyPosition<S>, Error>;

    fn sample(
        &self,
        target: CelestialBody,
        reference: CelestialBody,
        epoch: Instant<S>,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<RelativeObservation<S>, Error> {
        let target = self.position(target, epoch, evaluations, maximum_evaluations)?;
        let reference = self.position(reference, epoch, evaluations, maximum_evaluations)?;
        RelativeObservation::new(target, reference)
    }
}

pub(super) struct GeocentricRelativeSampler<'context, 'data, E> {
    astrometry: Astrometry<'context, 'data, E>,
    mode: AstrometricMode,
    light_time: ReceptionLightTimeOptions,
}

impl<'context, 'data, E> GeocentricRelativeSampler<'context, 'data, E> {
    pub(super) const fn new(
        astrometry: Astrometry<'context, 'data, E>,
        mode: AstrometricMode,
        light_time: ReceptionLightTimeOptions,
    ) -> Self {
        Self {
            astrometry,
            mode,
            light_time,
        }
    }

    fn consume_one(evaluations: &mut u32, maximum: u32) -> Result<(), Error> {
        if *evaluations >= maximum {
            return Err(Error::EvaluationLimitExceeded { maximum });
        }
        *evaluations += 1;
        Ok(())
    }

    fn geometric_position<S: TimeScale>(
        &self,
        body: CelestialBody,
        epoch: Instant<S>,
    ) -> Result<EventBodyPosition<S>, Error> {
        let state = self
            .astrometry
            .ephemeris()
            .state(EphemerisQuery::<Bcrs, S>::new(
                body,
                CelestialBody::Earth,
                epoch,
            ))
            .map_err(crate::astro::Error::from)?;
        let distance = state.position().magnitude()?;
        let direction = state.position().direction()?;
        let gcrs = EquatorialDirection::<Gcrs>::from_direction(
            Direction::<Gcrs>::try_from_components(direction.components())?,
        )?;
        let celestial = Frames::new(self.astrometry.time_context())
            .celestial_orientation_at(epoch)
            .map_err(crate::astro::Error::from)?;
        let true_equatorial = celestial
            .true_equatorial(gcrs)
            .map_err(crate::astro::Error::from)?;
        let true_ecliptic = celestial
            .true_ecliptic_from_gcrs(gcrs)
            .map_err(crate::astro::Error::from)?;
        Ok(EventBodyPosition::new(
            body,
            epoch,
            true_equatorial,
            true_ecliptic,
            distance,
        ))
    }

    fn apparent_position<S: TimeScale>(
        &self,
        body: CelestialBody,
        epoch: Instant<S>,
    ) -> Result<EventBodyPosition<S>, Error> {
        let apparent = self
            .astrometry
            .geocentric_apparent_place(body, epoch, self.light_time)?;
        Ok(EventBodyPosition::new(
            body,
            epoch,
            apparent.true_equatorial(),
            apparent.true_ecliptic(),
            apparent.distance(),
        ))
    }
}

impl<S: TimeScale, E> RelativeSampler<S> for GeocentricRelativeSampler<'_, '_, E> {
    fn origin(&self) -> ObservationOrigin {
        ObservationOrigin::Geocenter
    }

    fn position(
        &self,
        body: CelestialBody,
        epoch: Instant<S>,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<EventBodyPosition<S>, Error> {
        Self::consume_one(evaluations, maximum_evaluations)?;
        match self.mode {
            AstrometricMode::Geometric => self.geometric_position(body, epoch),
            AstrometricMode::Apparent => self.apparent_position(body, epoch),
        }
    }
}

pub(super) struct FixedSiteRelativeSampler<'context, 'data, 'site, 'eop> {
    astrometry: Astrometry<'context, 'data, EarthOrientationTable<'eop>>,
    site: &'site FixedSite,
    mode: AstrometricMode,
    light_time: ReceptionLightTimeOptions,
}

impl<'context, 'data, 'site, 'eop> FixedSiteRelativeSampler<'context, 'data, 'site, 'eop> {
    pub(super) const fn new(
        astrometry: Astrometry<'context, 'data, EarthOrientationTable<'eop>>,
        site: &'site FixedSite,
        mode: AstrometricMode,
        light_time: ReceptionLightTimeOptions,
    ) -> Self {
        Self {
            astrometry,
            site,
            mode,
            light_time,
        }
    }

    fn consume_one(evaluations: &mut u32, maximum: u32) -> Result<(), Error> {
        if *evaluations >= maximum {
            return Err(Error::EvaluationLimitExceeded { maximum });
        }
        *evaluations += 1;
        Ok(())
    }

    fn position_from_gcrs<S: TimeScale>(
        &self,
        body: CelestialBody,
        epoch: Instant<S>,
        direction: EquatorialDirection<Gcrs>,
        distance: Length,
    ) -> Result<EventBodyPosition<S>, Error> {
        let celestial = Frames::new(self.astrometry.time_context())
            .celestial_orientation_at(epoch)
            .map_err(crate::astro::Error::from)?;
        let true_equatorial = celestial
            .true_equatorial(direction)
            .map_err(crate::astro::Error::from)?;
        let true_ecliptic = celestial
            .true_ecliptic_from_gcrs(direction)
            .map_err(crate::astro::Error::from)?;
        Ok(EventBodyPosition::new(
            body,
            epoch,
            true_equatorial,
            true_ecliptic,
            distance,
        ))
    }

    fn geometric_position<S: TimeScale>(
        &self,
        body: CelestialBody,
        epoch: Instant<S>,
    ) -> Result<EventBodyPosition<S>, Error> {
        let observer = self.astrometry.fixed_observer_at(self.site, epoch)?;
        let state = self
            .astrometry
            .ephemeris()
            .state(EphemerisQuery::<Bcrs, S>::new(
                body,
                CelestialBody::SolarSystemBarycenter,
                epoch,
            ))
            .map_err(crate::astro::Error::from)?;
        let position = state
            .position()
            .checked_sub(observer.barycentric_position())?;
        let distance = position.magnitude()?;
        let direction = position.direction()?;
        let gcrs = EquatorialDirection::<Gcrs>::from_direction(
            Direction::<Gcrs>::try_from_components(direction.components())?,
        )?;
        self.position_from_gcrs(body, epoch, gcrs, distance)
    }

    fn apparent_position<S: TimeScale>(
        &self,
        body: CelestialBody,
        epoch: Instant<S>,
    ) -> Result<EventBodyPosition<S>, Error> {
        let observer = self.astrometry.fixed_observer_at(self.site, epoch)?;
        let apparent = observer.vacuum_observed_place(body, self.light_time)?;
        let earth_orientation = Frames::new(self.astrometry.time_context())
            .earth_orientation_at(epoch)
            .map_err(crate::astro::Error::from)?;
        let gcrs = earth_orientation
            .gcrs_from_intermediate_equatorial(apparent.intermediate_equatorial())
            .map_err(crate::astro::Error::from)?;
        self.position_from_gcrs(body, epoch, gcrs, apparent.distance())
    }
}

impl<S: TimeScale> RelativeSampler<S> for FixedSiteRelativeSampler<'_, '_, '_, '_> {
    fn origin(&self) -> ObservationOrigin {
        ObservationOrigin::FixedSite
    }

    fn position(
        &self,
        body: CelestialBody,
        epoch: Instant<S>,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<EventBodyPosition<S>, Error> {
        Self::consume_one(evaluations, maximum_evaluations)?;
        match self.mode {
            AstrometricMode::Geometric => self.geometric_position(body, epoch),
            AstrometricMode::Apparent => self.apparent_position(body, epoch),
        }
    }
}
