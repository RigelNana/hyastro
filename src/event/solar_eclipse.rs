use core::f64::consts::PI;
use std::{string::String, vec::Vec};

use libm::{acos, sqrt};

use crate::{
    astro::{ApparentDiskSeparation, Astrometry, ReceptionLightTimeOptions, VacuumApparentDisk},
    earth::FixedSite,
    ephem::{CelestialBody, EphemerisProvenance, EphemerisProvider, SphericalBodyFigure},
    frame::HorizontalDirection,
    math::{Angle, PositionAngle},
    time::{
        Duration, EarthAttitudeModelProvenance, EarthOrientationTable, Instant,
        PredictedEarthOrientation, PredictionDisposition, TimeInterval, TimeScale,
    },
};

use super::{
    AngularEventSearchOptions, Error, EventEvidence, Events, ExtremumEvidence, MoonPhase,
    search::{BracketedExtremumSearch, BracketedRootSearch},
};

/// Spherical Sun and Moon figures used for local-disk and global shadow-cone geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarEclipseModel {
    sun: SphericalBodyFigure,
    moon: SphericalBodyFigure,
}

impl SolarEclipseModel {
    /// Constructs an explicitly identified spherical-disk model.
    pub fn new(sun: SphericalBodyFigure, moon: SphericalBodyFigure) -> Result<Self, Error> {
        if sun.body() != CelestialBody::Sun {
            return Err(Error::InvalidSolarEclipseFigure {
                role: "Sun",
                expected: CelestialBody::Sun,
                actual: sun.body(),
            });
        }
        if moon.body() != CelestialBody::Moon {
            return Err(Error::InvalidSolarEclipseFigure {
                role: "Moon",
                expected: CelestialBody::Moon,
                actual: moon.body(),
            });
        }
        Ok(Self { sun, moon })
    }

    /// Returns the IAU 2015 nominal solar sphere and IAU WGCCRE 2015 lunar sphere.
    pub const fn standard() -> Self {
        Self {
            sun: SphericalBodyFigure::IAU_2015_NOMINAL_SUN,
            moon: SphericalBodyFigure::IAU_WGCCRE_2015_MOON,
        }
    }

    /// Returns the spherical solar-limb model.
    pub const fn sun(self) -> SphericalBodyFigure {
        self.sun
    }

    /// Returns the spherical lunar-limb model.
    pub const fn moon(self) -> SphericalBodyFigure {
        self.moon
    }
}

/// Search controls and spherical figures shared by local and global solar eclipses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarEclipseSearchOptions {
    angular_search: AngularEventSearchOptions,
    model: SolarEclipseModel,
}

impl SolarEclipseSearchOptions {
    /// Combines validated angular-event controls with an explicit disk model.
    pub const fn new(angular_search: AngularEventSearchOptions, model: SolarEclipseModel) -> Self {
        Self {
            angular_search,
            model,
        }
    }

    /// Returns seven-day syzygy scanning, millisecond maximum timing, and standard IAU figures.
    pub const fn standard() -> Self {
        Self::new(
            AngularEventSearchOptions::standard(),
            SolarEclipseModel::standard(),
        )
    }

    /// Returns the controls used for new-Moon seeding and numerical refinement.
    pub const fn angular_search(self) -> AngularEventSearchOptions {
        self.angular_search
    }

    /// Returns the solar and lunar spherical figures used for every apparent disk.
    pub const fn model(self) -> SolarEclipseModel {
        self.model
    }
}

/// Fraction of the apparent solar diameter obscured along the line of centres.
///
/// Zero denotes exterior tangency. Values above one are possible during totality.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SolarEclipseMagnitude(f64);

impl SolarEclipseMagnitude {
    fn signed_from_geometry(solar_radius: f64, lunar_radius: f64, separation: f64) -> f64 {
        (solar_radius + lunar_radius - separation) / (2.0 * solar_radius)
    }

    fn from_geometry(solar_radius: f64, lunar_radius: f64, separation: f64) -> Self {
        Self(Self::signed_from_geometry(solar_radius, lunar_radius, separation).max(0.0))
    }

    /// Returns the dimensionless eclipse magnitude.
    pub const fn as_ratio(self) -> f64 {
        self.0
    }
}

/// Fraction of the apparent solar-disk area covered by the apparent lunar disk.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SolarObscuration(f64);

impl SolarObscuration {
    fn from_geometry(solar_radius: f64, lunar_radius: f64, separation: f64) -> Self {
        let radius_sum = solar_radius + lunar_radius;
        if separation >= radius_sum {
            return Self(0.0);
        }

        let radius_difference = (solar_radius - lunar_radius).abs();
        if separation <= radius_difference {
            let ratio = if lunar_radius >= solar_radius {
                1.0
            } else {
                lunar_radius * lunar_radius / (solar_radius * solar_radius)
            };
            return Self(ratio.clamp(0.0, 1.0));
        }

        let solar_squared = solar_radius * solar_radius;
        let lunar_squared = lunar_radius * lunar_radius;
        let separation_squared = separation * separation;
        let solar_angle = acos(
            ((separation_squared + solar_squared - lunar_squared)
                / (2.0 * separation * solar_radius))
                .clamp(-1.0, 1.0),
        );
        let lunar_angle = acos(
            ((separation_squared + lunar_squared - solar_squared)
                / (2.0 * separation * lunar_radius))
                .clamp(-1.0, 1.0),
        );
        let radicand = (-separation + solar_radius + lunar_radius)
            * (separation + solar_radius - lunar_radius)
            * (separation - solar_radius + lunar_radius)
            * (separation + solar_radius + lunar_radius);
        let overlap_area = solar_squared * solar_angle + lunar_squared * lunar_angle
            - 0.5 * sqrt(radicand.max(0.0));
        Self((overlap_area / (PI * solar_squared)).clamp(0.0, 1.0))
    }

    /// Returns the covered-area fraction in `[0, 1]`.
    pub const fn as_ratio(self) -> f64 {
        self.0
    }

    /// Returns the covered-area percentage in `[0, 100]`.
    pub fn as_percent(self) -> f64 {
        self.0 * 100.0
    }
}

/// One coherent topocentric vacuum observation of the solar and lunar disks.
///
/// Both centres share one fixed observer and reception instant. Atmospheric refraction and lunar
/// limb topography are deliberately absent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalSolarEclipseObservation<S: TimeScale> {
    sun: VacuumApparentDisk<S>,
    moon: VacuumApparentDisk<S>,
    separation: ApparentDiskSeparation,
    magnitude: SolarEclipseMagnitude,
    obscuration: SolarObscuration,
    moon_position_angle: Option<PositionAngle>,
}

impl<S: TimeScale> LocalSolarEclipseObservation<S> {
    fn new(sun: VacuumApparentDisk<S>, moon: VacuumApparentDisk<S>) -> Result<Self, Error> {
        let separation = sun.separation_from(moon)?;
        let solar_radius = sun.semidiameter().as_radians();
        let lunar_radius = moon.semidiameter().as_radians();
        let centre_separation = separation.centre_separation().as_radians();
        let sun_direction = sun
            .centre()
            .intermediate_equatorial()
            .coordinates()
            .to_spherical()?;
        let moon_direction = moon
            .centre()
            .intermediate_equatorial()
            .coordinates()
            .to_spherical()?;
        let moon_position_angle = match sun_direction.position_angle_to(moon_direction) {
            Ok(value) => Some(value),
            Err(crate::math::Error::UndefinedPositionAngle) => None,
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            sun,
            moon,
            separation,
            magnitude: SolarEclipseMagnitude::from_geometry(
                solar_radius,
                lunar_radius,
                centre_separation,
            ),
            obscuration: SolarObscuration::from_geometry(
                solar_radius,
                lunar_radius,
                centre_separation,
            ),
            moon_position_angle,
        })
    }

    /// Returns the common fixed-site reception instant.
    pub const fn instant(self) -> Instant<S> {
        self.sun.centre().reception_epoch()
    }

    /// Returns the Sun's topocentric vacuum apparent disk.
    pub const fn sun(self) -> VacuumApparentDisk<S> {
        self.sun
    }

    /// Returns the Moon's topocentric vacuum apparent disk.
    pub const fn moon(self) -> VacuumApparentDisk<S> {
        self.moon
    }

    /// Returns the apparent disk-centre separation and overlap relationship.
    pub const fn separation(self) -> ApparentDiskSeparation {
        self.separation
    }

    /// Returns the fraction of solar diameter obscured along the line of centres.
    pub const fn magnitude(self) -> SolarEclipseMagnitude {
        self.magnitude
    }

    /// Returns the fraction of apparent solar-disk area covered by the Moon.
    pub const fn obscuration(self) -> SolarObscuration {
        self.obscuration
    }

    /// Returns the Moon-centre position angle measured eastward from CIRS north at the Sun.
    ///
    /// The value is absent only when both apparent centres coincide exactly.
    pub const fn moon_position_angle(self) -> Option<PositionAngle> {
        self.moon_position_angle
    }

    /// Returns the Sun-centre vacuum azimuth and altitude.
    pub const fn solar_horizontal(self) -> HorizontalDirection {
        self.sun.centre().horizontal()
    }

    /// Reports whether any part of the vacuum solar disk lies above the astronomical horizon.
    pub fn solar_disk_is_above_horizon(self) -> bool {
        self.solar_horizontal().altitude().as_radians() + self.sun.semidiameter().as_radians() > 0.0
    }
}

/// Local geometric solar-eclipse classification at greatest magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LocalSolarEclipseKind {
    /// The lunar disk overlaps but never lies strictly inside or contains the solar disk.
    Partial,
    /// The smaller lunar disk lies strictly inside the solar disk around maximum eclipse.
    Annular,
    /// The lunar disk strictly contains the solar disk around maximum eclipse.
    Total,
}

/// Identity of one local solar-eclipse contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SolarEclipseContactKind {
    /// C1, first exterior contact or initial tangency.
    First,
    /// C2, beginning of totality or annularity.
    Second,
    /// C3, end of totality or annularity.
    Third,
    /// C4, final exterior contact or final tangency.
    Fourth,
}

impl SolarEclipseContactKind {
    const fn is_internal(self) -> bool {
        matches!(self, Self::Second | Self::Third)
    }

    const fn description(self) -> &'static str {
        match self {
            Self::First => "local solar-eclipse first contact",
            Self::Second => "local solar-eclipse second contact",
            Self::Third => "local solar-eclipse third contact",
            Self::Fourth => "local solar-eclipse fourth contact",
        }
    }
}

/// One refined local solar-eclipse contact with complete disk geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarEclipseContact<S: TimeScale> {
    kind: SolarEclipseContactKind,
    observation: LocalSolarEclipseObservation<S>,
    limb_position_angle: PositionAngle,
    evidence: EventEvidence<S>,
}

impl<S: TimeScale> SolarEclipseContact<S> {
    fn new(
        kind: SolarEclipseContactKind,
        eclipse_kind: LocalSolarEclipseKind,
        observation: LocalSolarEclipseObservation<S>,
        evidence: EventEvidence<S>,
    ) -> Result<Self, Error> {
        let moon_position_angle = observation
            .moon_position_angle()
            .ok_or(crate::math::Error::UndefinedPositionAngle)?;
        let limb_position_angle =
            if kind.is_internal() && eclipse_kind == LocalSolarEclipseKind::Total {
                PositionAngle::wrap_radians(moon_position_angle.as_radians() + PI)?
            } else {
                moon_position_angle
            };
        Ok(Self {
            kind,
            observation,
            limb_position_angle,
            evidence,
        })
    }

    /// Returns the contact identity.
    pub const fn kind(self) -> SolarEclipseContactKind {
        self.kind
    }

    /// Returns the refined contact instant.
    pub const fn instant(self) -> Instant<S> {
        self.observation.instant()
    }

    /// Returns the complete topocentric disk observation at contact.
    pub const fn observation(self) -> LocalSolarEclipseObservation<S> {
        self.observation
    }

    /// Returns the tangency point's solar-limb position angle eastward from CIRS north.
    pub const fn limb_position_angle(self) -> PositionAngle {
        self.limb_position_angle
    }

    /// Returns the numerical root-refinement evidence.
    pub const fn evidence(self) -> EventEvidence<S> {
        self.evidence
    }
}

/// Greatest local solar-eclipse magnitude and its numerical evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalSolarEclipseMaximum<S: TimeScale> {
    observation: LocalSolarEclipseObservation<S>,
    evidence: ExtremumEvidence<S>,
}

impl<S: TimeScale> LocalSolarEclipseMaximum<S> {
    const fn new(
        observation: LocalSolarEclipseObservation<S>,
        evidence: ExtremumEvidence<S>,
    ) -> Self {
        Self {
            observation,
            evidence,
        }
    }

    /// Returns the greatest-magnitude instant.
    pub const fn instant(self) -> Instant<S> {
        self.observation.instant()
    }

    /// Returns the complete topocentric disk observation at greatest magnitude.
    pub const fn observation(self) -> LocalSolarEclipseObservation<S> {
        self.observation
    }

    /// Returns the numerical bounded-extremum evidence.
    pub const fn evidence(self) -> ExtremumEvidence<S> {
        self.evidence
    }
}

/// Complete Earth-attitude provenance retained by a local solar-eclipse result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolarEclipseEarthAttitudeProvenance {
    source: String,
    predicted: bool,
    delta_t_model: Option<String>,
    delta_t_disposition: Option<PredictionDisposition>,
    offset_model: Option<String>,
    offset_disposition: Option<PredictionDisposition>,
}

impl SolarEclipseEarthAttitudeProvenance {
    fn from_model(value: EarthAttitudeModelProvenance<'_>) -> Self {
        Self {
            source: value.source().to_owned(),
            predicted: value.is_predicted(),
            delta_t_model: value.delta_t_model().map(str::to_owned),
            delta_t_disposition: value.delta_t_disposition(),
            offset_model: value.offset_model().map(str::to_owned),
            offset_disposition: value.offset_disposition(),
        }
    }

    /// Returns the table version or prediction-scenario identifier.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns whether the result used an explicit modeled scenario.
    pub const fn is_predicted(&self) -> bool {
        self.predicted
    }

    /// Returns the direct Delta T model identifier, when applicable.
    pub fn delta_t_model(&self) -> Option<&str> {
        self.delta_t_model.as_deref()
    }

    /// Returns whether Delta T was predicted or explicitly assumed.
    pub const fn delta_t_disposition(&self) -> Option<PredictionDisposition> {
        self.delta_t_disposition
    }

    /// Returns the pole-offset model identifier, when applicable.
    pub fn offset_model(&self) -> Option<&str> {
        self.offset_model.as_deref()
    }

    /// Returns whether pole offsets were predicted or explicitly assumed.
    pub const fn offset_disposition(&self) -> Option<PredictionDisposition> {
        self.offset_disposition
    }
}

/// Complete geometric circumstances of one local solar eclipse at a fixed terrestrial site.
///
/// An eclipse belongs to a requested interval when its greatest-magnitude instant lies in that
/// closed interval. Contacts are solved beyond the interval edges when necessary so the returned
/// sequence remains complete. All positions are vacuum apparent positions; atmospheric visibility
/// is not implied.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalSolarEclipse<S: TimeScale> {
    site: FixedSite,
    kind: LocalSolarEclipseKind,
    first_contact: SolarEclipseContact<S>,
    second_contact: Option<SolarEclipseContact<S>>,
    maximum: LocalSolarEclipseMaximum<S>,
    third_contact: Option<SolarEclipseContact<S>>,
    fourth_contact: SolarEclipseContact<S>,
    partial_phase_duration: Duration,
    central_phase_duration: Option<Duration>,
    model: SolarEclipseModel,
    ephemeris: EphemerisProvenance,
    earth_attitude_provenance: SolarEclipseEarthAttitudeProvenance,
}

impl<S: TimeScale> LocalSolarEclipse<S> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        site: FixedSite,
        kind: LocalSolarEclipseKind,
        first_contact: SolarEclipseContact<S>,
        second_contact: Option<SolarEclipseContact<S>>,
        maximum: LocalSolarEclipseMaximum<S>,
        third_contact: Option<SolarEclipseContact<S>>,
        fourth_contact: SolarEclipseContact<S>,
        model: SolarEclipseModel,
        ephemeris: EphemerisProvenance,
        earth_attitude_provenance: SolarEclipseEarthAttitudeProvenance,
    ) -> Result<Self, Error> {
        let partial_phase_duration = fourth_contact
            .instant()
            .duration_since(first_contact.instant())?;
        let central_phase_duration = second_contact
            .zip(third_contact)
            .map(|(second, third)| third.instant().duration_since(second.instant()))
            .transpose()?;
        Ok(Self {
            site,
            kind,
            first_contact,
            second_contact,
            maximum,
            third_contact,
            fourth_contact,
            partial_phase_duration,
            central_phase_duration,
            model,
            ephemeris,
            earth_attitude_provenance,
        })
    }

    /// Returns the fixed terrestrial observing site.
    pub const fn site(&self) -> &FixedSite {
        &self.site
    }

    /// Returns the local partial, annular, or total classification.
    pub const fn kind(&self) -> LocalSolarEclipseKind {
        self.kind
    }

    /// Returns C1, the first exterior contact.
    pub const fn first_contact(&self) -> SolarEclipseContact<S> {
        self.first_contact
    }

    /// Returns C2 for an annular or total eclipse.
    pub const fn second_contact(&self) -> Option<SolarEclipseContact<S>> {
        self.second_contact
    }

    /// Returns the greatest local eclipse magnitude.
    pub const fn maximum(&self) -> LocalSolarEclipseMaximum<S> {
        self.maximum
    }

    /// Returns C3 for an annular or total eclipse.
    pub const fn third_contact(&self) -> Option<SolarEclipseContact<S>> {
        self.third_contact
    }

    /// Returns C4, the fourth exterior contact.
    pub const fn fourth_contact(&self) -> SolarEclipseContact<S> {
        self.fourth_contact
    }

    /// Returns the C1-to-C4 geometric partial-phase duration.
    pub const fn partial_phase_duration(&self) -> Duration {
        self.partial_phase_duration
    }

    /// Returns the C2-to-C3 annular or total phase duration.
    pub const fn central_phase_duration(&self) -> Option<Duration> {
        self.central_phase_duration
    }

    /// Returns the spherical figures used for the apparent limbs.
    pub const fn model(&self) -> SolarEclipseModel {
        self.model
    }

    /// Returns the exact ephemeris model and kernel provenance.
    pub const fn ephemeris_provenance(&self) -> &EphemerisProvenance {
        &self.ephemeris
    }

    /// Returns complete tabulated or predicted Earth-attitude provenance.
    pub const fn earth_attitude_provenance(&self) -> &SolarEclipseEarthAttitudeProvenance {
        &self.earth_attitude_provenance
    }
}

#[derive(Clone, Copy)]
enum ContactDirection {
    Before,
    After,
}

impl ContactDirection {
    fn advance<S: TimeScale>(self, epoch: Instant<S>, step: Duration) -> Result<Instant<S>, Error> {
        match self {
            Self::Before => Ok(epoch.checked_sub(step)?),
            Self::After => Ok(epoch.checked_add(step)?),
        }
    }

    const fn ordered<S: TimeScale>(
        self,
        inner: Instant<S>,
        outer: Instant<S>,
    ) -> (Instant<S>, Instant<S>) {
        match self {
            Self::Before => (outer, inner),
            Self::After => (inner, outer),
        }
    }
}

#[derive(Clone, Copy)]
enum ContactCriterion {
    External,
    Internal,
}

impl ContactCriterion {
    fn residual<S: TimeScale>(self, observation: LocalSolarEclipseObservation<S>) -> f64 {
        let separation = observation.separation().centre_separation().as_radians();
        let solar_radius = observation.sun().semidiameter().as_radians();
        let lunar_radius = observation.moon().semidiameter().as_radians();
        match self {
            Self::External => separation - solar_radius - lunar_radius,
            Self::Internal => separation - (solar_radius - lunar_radius).abs(),
        }
    }
}

trait LocalSolarEclipseObserver {
    fn observe_local_solar_eclipse<S: TimeScale>(
        &self,
        site: &FixedSite,
        epoch: Instant<S>,
        light_time: ReceptionLightTimeOptions,
        model: SolarEclipseModel,
    ) -> Result<LocalSolarEclipseObservation<S>, Error>;

    fn earth_attitude_provenance(&self) -> EarthAttitudeModelProvenance<'_>;
}

impl<'context, 'data, 'eop, P: EphemerisProvider + ?Sized> LocalSolarEclipseObserver
    for Astrometry<'context, 'data, EarthOrientationTable<'eop>, P>
{
    fn observe_local_solar_eclipse<S: TimeScale>(
        &self,
        site: &FixedSite,
        epoch: Instant<S>,
        light_time: ReceptionLightTimeOptions,
        model: SolarEclipseModel,
    ) -> Result<LocalSolarEclipseObservation<S>, Error> {
        let observer = self.fixed_observer_at(site, epoch)?;
        let sun = observer
            .vacuum_observed_place(CelestialBody::Sun, light_time)?
            .apparent_disk(model.sun())?;
        let moon = observer
            .vacuum_observed_place(CelestialBody::Moon, light_time)?
            .apparent_disk(model.moon())?;
        LocalSolarEclipseObservation::new(sun, moon)
    }

    fn earth_attitude_provenance(&self) -> EarthAttitudeModelProvenance<'_> {
        self.time_context().earth_attitude_provenance()
    }
}

impl<'context, 'data, 'prediction, P: EphemerisProvider + ?Sized> LocalSolarEclipseObserver
    for Astrometry<'context, 'data, PredictedEarthOrientation<'prediction>, P>
{
    fn observe_local_solar_eclipse<S: TimeScale>(
        &self,
        site: &FixedSite,
        epoch: Instant<S>,
        light_time: ReceptionLightTimeOptions,
        model: SolarEclipseModel,
    ) -> Result<LocalSolarEclipseObservation<S>, Error> {
        let observer = self.fixed_observer_with_nominal_rotation_at(site, epoch)?;
        let sun = observer
            .vacuum_observed_place(CelestialBody::Sun, light_time)?
            .apparent_disk(model.sun())?;
        let moon = observer
            .vacuum_observed_place(CelestialBody::Moon, light_time)?
            .apparent_disk(model.moon())?;
        LocalSolarEclipseObservation::new(sun, moon)
    }

    fn earth_attitude_provenance(&self) -> EarthAttitudeModelProvenance<'_> {
        self.time_context().earth_attitude_provenance()
    }
}

struct LocalSolarEclipseSampler<'context, 'data, 'site, E, P: EphemerisProvider + ?Sized> {
    astrometry: Astrometry<'context, 'data, E, P>,
    site: &'site FixedSite,
    light_time: ReceptionLightTimeOptions,
    model: SolarEclipseModel,
}

impl<'context, 'data, 'site, E, P: EphemerisProvider + ?Sized>
    LocalSolarEclipseSampler<'context, 'data, 'site, E, P>
where
    Astrometry<'context, 'data, E, P>: LocalSolarEclipseObserver,
{
    const MAXIMUM_HALF_WINDOW: Duration =
        Duration::from_nanoseconds(12 * 60 * 60 * Duration::NANOSECONDS_PER_SECOND);
    const CONTACT_STEP: Duration =
        Duration::from_nanoseconds(30 * 60 * Duration::NANOSECONDS_PER_SECOND);
    const MAX_CONTACT_STEPS: u32 = 24;

    const fn new(
        astrometry: Astrometry<'context, 'data, E, P>,
        site: &'site FixedSite,
        options: SolarEclipseSearchOptions,
    ) -> Self {
        Self {
            astrometry,
            site,
            light_time: options.angular_search().light_time(),
            model: options.model(),
        }
    }

    fn consume_observation(evaluations: &mut u32, maximum: u32) -> Result<(), Error> {
        if maximum.saturating_sub(*evaluations) < 2 {
            return Err(Error::EvaluationLimitExceeded { maximum });
        }
        *evaluations += 2;
        Ok(())
    }

    fn observe<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<LocalSolarEclipseObservation<S>, Error> {
        Self::consume_observation(evaluations, maximum_evaluations)?;
        self.astrometry
            .observe_local_solar_eclipse(self.site, epoch, self.light_time, self.model)
    }

    fn maximum_near<S: TimeScale>(
        &self,
        seed: Instant<S>,
        options: SolarEclipseSearchOptions,
    ) -> Result<LocalSolarEclipseMaximum<S>, Error> {
        let controls = options.angular_search();
        let start = seed.checked_sub(Self::MAXIMUM_HALF_WINDOW)?;
        let end = seed.checked_add(Self::MAXIMUM_HALF_WINDOW)?;
        let mut evaluations = 0_u32;
        let refined = BracketedExtremumSearch::refine_minimum(
            start,
            seed,
            end,
            controls.time_tolerance(),
            controls.max_refinement_iterations(),
            |epoch| {
                self.observe(epoch, &mut evaluations, controls.max_evaluations())
                    .map(|observation| {
                        -SolarEclipseMagnitude::signed_from_geometry(
                            observation.sun().semidiameter().as_radians(),
                            observation.moon().semidiameter().as_radians(),
                            observation.separation().centre_separation().as_radians(),
                        )
                    })
            },
        )?;
        let observation = self.observe(
            refined.instant(),
            &mut evaluations,
            controls.max_evaluations(),
        )?;
        Ok(LocalSolarEclipseMaximum::new(
            observation,
            ExtremumEvidence::new(
                refined.bracket_start(),
                refined.bracket_end(),
                refined.time_uncertainty(),
                refined.iterations(),
                evaluations,
            ),
        ))
    }

    fn classify<S: TimeScale>(
        maximum: LocalSolarEclipseObservation<S>,
        tolerance: Angle,
    ) -> Option<LocalSolarEclipseKind> {
        let exterior = ContactCriterion::External.residual(maximum);
        if exterior > tolerance.as_radians() {
            return None;
        }
        let solar_radius = maximum.sun().semidiameter().as_radians();
        let lunar_radius = maximum.moon().semidiameter().as_radians();
        let internal = ContactCriterion::Internal.residual(maximum);
        if internal >= -tolerance.as_radians() {
            Some(LocalSolarEclipseKind::Partial)
        } else if lunar_radius < solar_radius {
            Some(LocalSolarEclipseKind::Annular)
        } else {
            Some(LocalSolarEclipseKind::Total)
        }
    }

    fn tangent_contact<S: TimeScale>(
        maximum: LocalSolarEclipseMaximum<S>,
        kind: SolarEclipseContactKind,
        eclipse_kind: LocalSolarEclipseKind,
    ) -> Result<SolarEclipseContact<S>, Error> {
        let observation = maximum.observation();
        let evidence = maximum.evidence();
        SolarEclipseContact::new(
            kind,
            eclipse_kind,
            observation,
            EventEvidence::new(
                evidence.bracket_start(),
                evidence.bracket_end(),
                evidence.time_uncertainty(),
                Angle::from_radians(ContactCriterion::External.residual(observation))?,
                0,
                evidence.evaluations(),
            ),
        )
    }

    fn refine_contact<S: TimeScale>(
        &self,
        maximum: LocalSolarEclipseObservation<S>,
        direction: ContactDirection,
        criterion: ContactCriterion,
        kind: SolarEclipseContactKind,
        eclipse_kind: LocalSolarEclipseKind,
        options: SolarEclipseSearchOptions,
    ) -> Result<SolarEclipseContact<S>, Error> {
        let controls = options.angular_search();
        let mut evaluations = 0_u32;
        let mut inner_epoch = maximum.instant();
        let mut bracket = None;
        for _ in 0..Self::MAX_CONTACT_STEPS {
            let outer_epoch = direction.advance(inner_epoch, Self::CONTACT_STEP)?;
            let outer = self.observe(outer_epoch, &mut evaluations, controls.max_evaluations())?;
            if criterion.residual(outer) >= 0.0 {
                bracket = Some(direction.ordered(inner_epoch, outer_epoch));
                break;
            }
            inner_epoch = outer_epoch;
        }
        let (bracket_start, bracket_end) =
            bracket.ok_or(Error::SolarEclipseContactNotBracketed {
                contact: kind.description(),
                maximum_tai_nanoseconds: maximum.instant().tai_nanoseconds_since_1900(),
            })?;
        let time_tolerance = Duration::from_nanoseconds(1);
        let root = BracketedRootSearch::refine(
            bracket_start,
            bracket_end,
            time_tolerance,
            controls.max_refinement_iterations(),
            |epoch| {
                self.observe(epoch, &mut evaluations, controls.max_evaluations())
                    .map(|observation| criterion.residual(observation))
            },
        )?;
        let observation =
            self.observe(root.instant(), &mut evaluations, controls.max_evaluations())?;
        let residual = criterion.residual(observation);
        if residual.abs() > controls.angular_tolerance().as_radians() {
            return Err(Error::AngularResidualExceeded {
                event: kind.description(),
                residual_radians: residual.abs(),
                tolerance_radians: controls.angular_tolerance().as_radians(),
            });
        }
        SolarEclipseContact::new(
            kind,
            eclipse_kind,
            observation,
            EventEvidence::new(
                root.bracket_start(),
                root.bracket_end(),
                root.time_uncertainty(),
                Angle::from_radians(residual)?,
                root.iterations(),
                evaluations,
            ),
        )
    }
}

fn search_local_solar_eclipses_in<'context, 'data, E, P: EphemerisProvider + ?Sized, S: TimeScale>(
    events: &Events<'context, 'data, E, P>,
    site: &FixedSite,
    interval: TimeInterval<S>,
    options: SolarEclipseSearchOptions,
) -> Result<Vec<LocalSolarEclipse<S>>, Error>
where
    Astrometry<'context, 'data, E, P>: LocalSolarEclipseObserver,
{
    let controls = options.angular_search();
    let seed_interval = TimeInterval::new(
        interval
            .start()
            .checked_sub(LocalSolarEclipseSampler::<E, P>::MAXIMUM_HALF_WINDOW)?,
        interval
            .end()
            .checked_add(LocalSolarEclipseSampler::<E, P>::MAXIMUM_HALF_WINDOW)?,
    )?;
    let seeds = events.moon_phase_angle_in(
        seed_interval,
        MoonPhase::NewMoon.target_longitude_difference(),
        controls,
    )?;
    let sampler = LocalSolarEclipseSampler::new(events.astrometry, site, options);
    let ephemeris = events
        .astrometry
        .ephemeris()
        .provenance()
        .map_err(crate::astro::Error::from)?;
    let earth_attitude_provenance = SolarEclipseEarthAttitudeProvenance::from_model(
        events.astrometry.earth_attitude_provenance(),
    );
    let mut eclipses = Vec::new();

    for seed in seeds {
        let maximum = sampler.maximum_near(seed.instant(), options)?;
        if !interval.contains(maximum.instant()) {
            continue;
        }
        let Some(kind) = LocalSolarEclipseSampler::<E, P>::classify(
            maximum.observation(),
            controls.angular_tolerance(),
        ) else {
            continue;
        };
        let exterior_residual = ContactCriterion::External.residual(maximum.observation());
        let tangent = exterior_residual.abs() <= controls.angular_tolerance().as_radians();
        let (first_contact, fourth_contact) = if tangent {
            (
                LocalSolarEclipseSampler::<E, P>::tangent_contact(
                    maximum,
                    SolarEclipseContactKind::First,
                    kind,
                )?,
                LocalSolarEclipseSampler::<E, P>::tangent_contact(
                    maximum,
                    SolarEclipseContactKind::Fourth,
                    kind,
                )?,
            )
        } else {
            (
                sampler.refine_contact(
                    maximum.observation(),
                    ContactDirection::Before,
                    ContactCriterion::External,
                    SolarEclipseContactKind::First,
                    kind,
                    options,
                )?,
                sampler.refine_contact(
                    maximum.observation(),
                    ContactDirection::After,
                    ContactCriterion::External,
                    SolarEclipseContactKind::Fourth,
                    kind,
                    options,
                )?,
            )
        };
        let (second_contact, third_contact) = if matches!(
            kind,
            LocalSolarEclipseKind::Annular | LocalSolarEclipseKind::Total
        ) {
            (
                Some(sampler.refine_contact(
                    maximum.observation(),
                    ContactDirection::Before,
                    ContactCriterion::Internal,
                    SolarEclipseContactKind::Second,
                    kind,
                    options,
                )?),
                Some(sampler.refine_contact(
                    maximum.observation(),
                    ContactDirection::After,
                    ContactCriterion::Internal,
                    SolarEclipseContactKind::Third,
                    kind,
                    options,
                )?),
            )
        } else {
            (None, None)
        };
        eclipses.push(LocalSolarEclipse::new(
            site.clone(),
            kind,
            first_contact,
            second_contact,
            maximum,
            third_contact,
            fourth_contact,
            options.model(),
            ephemeris.clone(),
            earth_attitude_provenance.clone(),
        )?);
    }

    eclipses.sort_by_key(|eclipse| eclipse.maximum().instant().tai_nanoseconds_since_1900());
    eclipses.dedup_by_key(|eclipse| eclipse.maximum().instant().tai_nanoseconds_since_1900());
    Ok(eclipses)
}

impl<'context, 'data, 'eop, P: EphemerisProvider + ?Sized>
    Events<'context, 'data, EarthOrientationTable<'eop>, P>
{
    /// Finds complete local solar-eclipse circumstances using tabulated complete EOP.
    ///
    /// Geocentric New Moons seed fixed-site contact searches. The calculation
    /// retains measured Earth rotation, vacuum apparent positions, spherical
    /// limbs, and exact ephemeris and EOP provenance.
    pub fn local_solar_eclipses_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        interval: TimeInterval<S>,
        options: SolarEclipseSearchOptions,
    ) -> Result<Vec<LocalSolarEclipse<S>>, Error> {
        search_local_solar_eclipses_in(self, site, interval, options)
    }
}

impl<'context, 'data, 'prediction, P: EphemerisProvider + ?Sized>
    Events<'context, 'data, PredictedEarthOrientation<'prediction>, P>
{
    /// Finds complete future local solar-eclipse circumstances from an explicit scenario.
    ///
    /// UT1 is derived directly from the scenario's Delta T model; no future UTC
    /// or leap-second mapping is required. Pole models, assumptions, and their
    /// provenance are retained in every result. Site velocity uses nominal
    /// Earth rotation because predicted scenarios carry no measured LOD.
    pub fn local_solar_eclipses_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        interval: TimeInterval<S>,
        options: SolarEclipseSearchOptions,
    ) -> Result<Vec<LocalSolarEclipse<S>>, Error> {
        search_local_solar_eclipses_in(self, site, interval, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::TAU;

    #[test]
    fn circular_overlap_covers_separation_partial_and_containment_limits() {
        assert_eq!(
            SolarObscuration::from_geometry(1.0, 0.5, 1.5).as_ratio(),
            0.0
        );
        let partial = SolarObscuration::from_geometry(1.0, 1.0, 1.0).as_ratio();
        assert!((partial - 0.391_002_218_955_770_75).abs() < 2.0e-15);
        assert_eq!(
            SolarObscuration::from_geometry(1.0, 0.5, 0.0).as_ratio(),
            0.25
        );
        assert_eq!(
            SolarObscuration::from_geometry(1.0, 2.0, 0.0).as_ratio(),
            1.0
        );
    }

    #[test]
    fn standard_model_retains_authoritative_spherical_figures() {
        let model = SolarEclipseModel::standard();
        assert_eq!(model.sun().body(), CelestialBody::Sun);
        assert_eq!(model.moon().body(), CelestialBody::Moon);
    }

    #[test]
    fn total_internal_contact_uses_opposite_solar_limb() {
        let position_angle = PositionAngle::try_from_degrees(35.0).unwrap();
        let opposite = PositionAngle::wrap_radians(position_angle.as_radians() + PI).unwrap();
        assert!((opposite.as_degrees() - 215.0).abs() < 1.0e-14);
        assert!(opposite.as_radians() < TAU);
    }
}
