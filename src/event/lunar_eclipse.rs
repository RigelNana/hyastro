use core::f64::consts::PI;
use std::{string::String, vec::Vec};

use libm::{asin, sin};

use crate::{
    astro::{
        Astrometry, GeocentricApparentPlace, ObservedPlace, ReceptionLightTimeOptions,
        SolarApparentPlace, VacuumObservedPlace,
    },
    earth::{Earth, FixedSite},
    ephem::{CelestialBody, EphemerisProvenance, EphemerisProvider, SphericalBodyFigure},
    frame::{EquatorialDirection, EquatorialDirectionAt, TrueEquatorEquinoxOfDate},
    math::{Altitude, Angle, Declination, Length, PositionAngle, RightAscension, Separation},
    time::{Duration, EarthOrientationTable, Instant, TimeInterval, TimeScale},
};

use super::{
    AngularEventSearchOptions, Error, EventEvidence, Events, ExtremumEvidence, HorizonCriterion,
    HorizonDiskPoint, HorizonEventKind, HorizonEventSearch, HorizonReference, HorizonSearchOptions,
    MoonPhase,
    search::{BracketedExtremumSearch, BracketedRootSearch},
};

/// A documented convention for enlarging Earth's geometric lunar-eclipse shadows.
///
/// The two scale factors retain the distinction between changing Earth's effective parallax and
/// scaling the completed umbral and penumbral radii. This distinction is necessary because the
/// Danjon and Chauvenet conventions are not algebraically equivalent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarShadowConvention {
    identifier: &'static str,
    earth_parallax_scale: f64,
    shadow_radius_scale: f64,
}

impl LunarShadowConvention {
    /// Pure spherical geometry with no empirical atmospheric enlargement.
    pub const GEOMETRIC: Self = Self {
        identifier: "geometric spherical Earth shadow",
        earth_parallax_scale: 1.0,
        shadow_radius_scale: 1.0,
    };

    /// Danjon's conventional effective-Earth correction used by the NASA Five Millennium catalog.
    ///
    /// The factor `1.01` combines the conventional atmospheric enlargement with a mean
    /// oblateness correction. The completed shadow radii are not scaled a second time.
    pub const DANJON: Self = Self {
        identifier: "Danjon 1.01 effective Earth parallax",
        earth_parallax_scale: 1.01,
        shadow_radius_scale: 1.0,
    };

    /// Chauvenet's traditional two-percent enlargement of both completed shadow radii.
    ///
    /// The `0.998340` factor first converts equatorial lunar parallax to the conventional mean
    /// Earth radius at latitude 45 degrees, then both radii are multiplied by `1.02`.
    pub const CHAUVENET: Self = Self {
        identifier: "Chauvenet 0.998340 parallax and 1.02 shadow scale",
        earth_parallax_scale: 0.998_340,
        shadow_radius_scale: 1.02,
    };

    /// Returns the stable model identifier.
    pub const fn identifier(self) -> &'static str {
        self.identifier
    }

    /// Returns the multiplier applied to equatorial lunar horizontal parallax.
    pub const fn earth_parallax_scale(self) -> f64 {
        self.earth_parallax_scale
    }

    /// Returns the multiplier applied after forming each shadow radius.
    pub const fn shadow_radius_scale(self) -> f64 {
        self.shadow_radius_scale
    }
}

/// Spherical Sun and Moon figures plus one explicit terrestrial-shadow convention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarEclipseModel {
    sun: SphericalBodyFigure,
    moon: SphericalBodyFigure,
    shadow: LunarShadowConvention,
}

impl LunarEclipseModel {
    /// Constructs an explicitly identified lunar-eclipse geometry model.
    pub fn new(
        sun: SphericalBodyFigure,
        moon: SphericalBodyFigure,
        shadow: LunarShadowConvention,
    ) -> Result<Self, Error> {
        if sun.body() != CelestialBody::Sun {
            return Err(Error::InvalidLunarEclipseFigure {
                role: "Sun",
                expected: CelestialBody::Sun,
                actual: sun.body(),
            });
        }
        if moon.body() != CelestialBody::Moon {
            return Err(Error::InvalidLunarEclipseFigure {
                role: "Moon",
                expected: CelestialBody::Moon,
                actual: moon.body(),
            });
        }
        Ok(Self { sun, moon, shadow })
    }

    /// Returns IAU reference spheres and the Danjon convention used by the NASA catalog.
    pub const fn standard() -> Self {
        Self {
            sun: SphericalBodyFigure::IAU_2015_NOMINAL_SUN,
            moon: SphericalBodyFigure::IAU_WGCCRE_2015_MOON,
            shadow: LunarShadowConvention::DANJON,
        }
    }

    /// Returns the spherical solar figure.
    pub const fn sun(self) -> SphericalBodyFigure {
        self.sun
    }

    /// Returns the spherical lunar figure.
    pub const fn moon(self) -> SphericalBodyFigure {
        self.moon
    }

    /// Returns the selected terrestrial-shadow convention.
    pub const fn shadow(self) -> LunarShadowConvention {
        self.shadow
    }
}

/// Search controls and physical model used by global lunar-eclipse calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarEclipseSearchOptions {
    angular_search: AngularEventSearchOptions,
    model: LunarEclipseModel,
}

impl LunarEclipseSearchOptions {
    /// Combines validated angular-event controls with an explicit eclipse model.
    pub const fn new(angular_search: AngularEventSearchOptions, model: LunarEclipseModel) -> Self {
        Self {
            angular_search,
            model,
        }
    }

    /// Returns standard event controls, IAU figures, and the NASA-catalog Danjon convention.
    pub const fn standard() -> Self {
        Self::new(
            AngularEventSearchOptions::standard(),
            LunarEclipseModel::standard(),
        )
    }

    /// Returns the controls used for full-Moon seeding and numerical refinement.
    pub const fn angular_search(self) -> AngularEventSearchOptions {
        self.angular_search
    }

    /// Returns the physical figures and shadow convention.
    pub const fn model(self) -> LunarEclipseModel {
        self.model
    }
}

/// Signed fraction of one lunar diameter immersed in a selected terrestrial shadow.
///
/// Negative values mean the lunar disk misses that shadow. Values between zero and one describe
/// a partial immersion, and values of one or greater describe complete immersion.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LunarEclipseMagnitude(f64);

impl LunarEclipseMagnitude {
    fn from_geometry(shadow_radius: f64, lunar_radius: f64, axis_distance: f64) -> Self {
        Self((shadow_radius + lunar_radius - axis_distance) / (2.0 * lunar_radius))
    }

    /// Returns the signed dimensionless magnitude.
    pub const fn as_ratio(self) -> f64 {
        self.0
    }
}

/// Instantaneous geocentric geometry of the Moon relative to Earth's anti-solar shadow axis.
///
/// All angular quantities share true equator and equinox of date axes at one reception epoch.
/// Shadow radii use the selected model's explicit enlargement convention. The physical axis
/// distance is the perpendicular distance at the Moon, not a surface distance on Earth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarShadowGeometry<S: TimeScale> {
    apparent_moon: GeocentricApparentPlace<S>,
    apparent_sun: SolarApparentPlace<S>,
    shadow_axis: EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>,
    axis_separation: Separation,
    axis_distance: Length,
    umbral_radius: Angle,
    penumbral_radius: Angle,
    lunar_semidiameter: Angle,
    position_angle: Option<PositionAngle>,
    umbral_magnitude: LunarEclipseMagnitude,
    penumbral_magnitude: LunarEclipseMagnitude,
}

impl<S: TimeScale> LunarShadowGeometry<S> {
    fn new(
        earth: &Earth,
        apparent_moon: GeocentricApparentPlace<S>,
        apparent_sun: SolarApparentPlace<S>,
        model: LunarEclipseModel,
    ) -> Result<Self, Error> {
        let epoch = apparent_moon.reception_epoch();
        let sun_coordinates = apparent_sun.true_equatorial().coordinates();
        let moon_coordinates = apparent_moon.true_equatorial().coordinates();
        let shadow_axis_coordinates = EquatorialDirection::new(
            RightAscension::wrap_radians(sun_coordinates.right_ascension().as_radians() + PI)?,
            Declination::try_from_radians(-sun_coordinates.declination().as_radians())?,
        );
        let shadow_axis = EquatorialDirectionAt::new(epoch, shadow_axis_coordinates);
        let axis_separation = shadow_axis_coordinates.separation_to(moon_coordinates)?;
        let position_angle = match shadow_axis_coordinates
            .to_spherical()?
            .position_angle_to(moon_coordinates.to_spherical()?)
        {
            Ok(value) => Some(value),
            Err(crate::math::Error::UndefinedPositionAngle) => None,
            Err(error) => return Err(error.into()),
        };

        let earth_radius = earth.reference_ellipsoid().semi_major_axis().as_metres();
        let moon_distance = apparent_moon.distance().as_metres();
        let sun_distance = apparent_sun.distance().as_metres();
        let lunar_parallax = asin((earth_radius / moon_distance).clamp(-1.0, 1.0));
        let solar_parallax = asin((earth_radius / sun_distance).clamp(-1.0, 1.0));
        let solar_semidiameter =
            asin((model.sun().radius().as_metres() / sun_distance).clamp(-1.0, 1.0));
        let lunar_semidiameter_radians =
            asin((model.moon().radius().as_metres() / moon_distance).clamp(-1.0, 1.0));
        let convention = model.shadow();
        let effective_earth = convention.earth_parallax_scale() * lunar_parallax;
        let umbral_radius_radians = convention.shadow_radius_scale()
            * (effective_earth - solar_semidiameter + solar_parallax);
        let penumbral_radius_radians = convention.shadow_radius_scale()
            * (effective_earth + solar_semidiameter + solar_parallax);
        let axis_separation_radians = axis_separation.as_radians();
        let axis_distance = Length::from_metres(moon_distance * sin(axis_separation_radians))?;
        let umbral_magnitude = LunarEclipseMagnitude::from_geometry(
            umbral_radius_radians,
            lunar_semidiameter_radians,
            axis_separation_radians,
        );
        let penumbral_magnitude = LunarEclipseMagnitude::from_geometry(
            penumbral_radius_radians,
            lunar_semidiameter_radians,
            axis_separation_radians,
        );

        Ok(Self {
            apparent_moon,
            apparent_sun,
            shadow_axis,
            axis_separation,
            axis_distance,
            umbral_radius: Angle::from_radians(umbral_radius_radians)?,
            penumbral_radius: Angle::from_radians(penumbral_radius_radians)?,
            lunar_semidiameter: Angle::from_radians(lunar_semidiameter_radians)?,
            position_angle,
            umbral_magnitude,
            penumbral_magnitude,
        })
    }

    /// Returns the common reception epoch.
    pub const fn instant(self) -> Instant<S> {
        self.apparent_moon.reception_epoch()
    }

    /// Returns the apparent geocentric Moon used by the geometry.
    pub const fn apparent_moon(self) -> GeocentricApparentPlace<S> {
        self.apparent_moon
    }

    /// Returns the apparent geocentric Sun used by the geometry.
    pub const fn apparent_sun(self) -> SolarApparentPlace<S> {
        self.apparent_sun
    }

    /// Returns Earth's anti-solar shadow-axis direction.
    pub const fn shadow_axis(self) -> EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S> {
        self.shadow_axis
    }

    /// Returns the Moon-centre angular separation from the shadow axis.
    pub const fn axis_separation(self) -> Separation {
        self.axis_separation
    }

    /// Returns the perpendicular Moon-centre distance from the shadow axis.
    pub const fn axis_distance(self) -> Length {
        self.axis_distance
    }

    /// Returns the selected model's umbral angular radius at lunar distance.
    pub const fn umbral_radius(self) -> Angle {
        self.umbral_radius
    }

    /// Returns the selected model's penumbral angular radius at lunar distance.
    pub const fn penumbral_radius(self) -> Angle {
        self.penumbral_radius
    }

    /// Returns the apparent lunar angular radius.
    pub const fn lunar_semidiameter(self) -> Angle {
        self.lunar_semidiameter
    }

    /// Returns the Moon-centre position angle eastward from celestial north around the shadow axis.
    ///
    /// The value is absent only when the Moon centre lies exactly on the shadow axis.
    pub const fn position_angle(self) -> Option<PositionAngle> {
        self.position_angle
    }

    /// Returns the signed umbral eclipse magnitude.
    pub const fn umbral_magnitude(self) -> LunarEclipseMagnitude {
        self.umbral_magnitude
    }

    /// Returns the signed penumbral eclipse magnitude.
    pub const fn penumbral_magnitude(self) -> LunarEclipseMagnitude {
        self.penumbral_magnitude
    }
}

/// Global lunar-eclipse classification at greatest eclipse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LunarEclipseKind {
    /// The Moon enters only Earth's penumbra.
    Penumbral,
    /// Part, but not all, of the Moon enters Earth's umbra.
    Partial,
    /// The complete lunar disk enters Earth's umbra.
    Total,
}

/// Identity of one global lunar-eclipse contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LunarEclipseContactKind {
    /// P1: exterior ingress into the penumbra.
    PenumbralIngress,
    /// U1: exterior ingress into the umbra.
    UmbralIngress,
    /// U2: interior ingress marking the start of totality.
    TotalityIngress,
    /// U3: interior egress marking the end of totality.
    TotalityEgress,
    /// U4: exterior egress from the umbra.
    UmbralEgress,
    /// P4: exterior egress from the penumbra.
    PenumbralEgress,
}

impl LunarEclipseContactKind {
    const fn description(self) -> &'static str {
        match self {
            Self::PenumbralIngress => "lunar-eclipse P1 contact",
            Self::UmbralIngress => "lunar-eclipse U1 contact",
            Self::TotalityIngress => "lunar-eclipse U2 contact",
            Self::TotalityEgress => "lunar-eclipse U3 contact",
            Self::UmbralEgress => "lunar-eclipse U4 contact",
            Self::PenumbralEgress => "lunar-eclipse P4 contact",
        }
    }
}

/// One refined global lunar-eclipse contact with complete geometry and numerical evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarEclipseContact<S: TimeScale> {
    kind: LunarEclipseContactKind,
    geometry: LunarShadowGeometry<S>,
    evidence: EventEvidence<S>,
}

impl<S: TimeScale> LunarEclipseContact<S> {
    /// Returns the P1/U1/U2/U3/U4/P4 contact identity.
    pub const fn kind(self) -> LunarEclipseContactKind {
        self.kind
    }

    /// Returns the refined contact instant.
    pub const fn instant(self) -> Instant<S> {
        self.geometry.instant()
    }

    /// Returns the instantaneous shadow geometry.
    pub const fn geometry(self) -> LunarShadowGeometry<S> {
        self.geometry
    }

    /// Returns numerical root-refinement evidence.
    pub const fn evidence(self) -> EventEvidence<S> {
        self.evidence
    }
}

/// Greatest global lunar eclipse and bounded-minimum evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarEclipseMaximum<S: TimeScale> {
    geometry: LunarShadowGeometry<S>,
    evidence: ExtremumEvidence<S>,
}

impl<S: TimeScale> LunarEclipseMaximum<S> {
    /// Returns the instant of minimum Moon-to-shadow-axis separation.
    pub const fn instant(self) -> Instant<S> {
        self.geometry.instant()
    }

    /// Returns the complete geometry at greatest eclipse.
    pub const fn geometry(self) -> LunarShadowGeometry<S> {
        self.geometry
    }

    /// Returns bounded-minimum numerical evidence.
    pub const fn evidence(self) -> ExtremumEvidence<S> {
        self.evidence
    }
}

/// One named global lunar-eclipse phase and its complete closed interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarEclipsePhaseInterval<S: TimeScale> {
    kind: LunarEclipseKind,
    interval: TimeInterval<S>,
}

impl<S: TimeScale> LunarEclipsePhaseInterval<S> {
    /// Returns the penumbral, partial, or total phase identity.
    pub const fn kind(self) -> LunarEclipseKind {
        self.kind
    }

    /// Returns the closed phase interval.
    pub const fn interval(self) -> TimeInterval<S> {
        self.interval
    }

    /// Returns the exact phase duration.
    pub fn duration(self) -> Result<Duration, crate::time::Error> {
        self.interval.end().duration_since(self.interval.start())
    }
}

/// Complete geocentric circumstances of one global lunar eclipse.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalLunarEclipse<S: TimeScale> {
    kind: LunarEclipseKind,
    penumbral_ingress: LunarEclipseContact<S>,
    umbral_ingress: Option<LunarEclipseContact<S>>,
    totality_ingress: Option<LunarEclipseContact<S>>,
    maximum: LunarEclipseMaximum<S>,
    totality_egress: Option<LunarEclipseContact<S>>,
    umbral_egress: Option<LunarEclipseContact<S>>,
    penumbral_egress: LunarEclipseContact<S>,
    model: LunarEclipseModel,
    earth: Earth,
    ephemeris: EphemerisProvenance,
}

impl<S: TimeScale> GlobalLunarEclipse<S> {
    /// Returns the penumbral, partial, or total classification.
    pub const fn kind(&self) -> LunarEclipseKind {
        self.kind
    }

    /// Returns P1.
    pub const fn penumbral_ingress(&self) -> LunarEclipseContact<S> {
        self.penumbral_ingress
    }

    /// Returns U1 when the Moon enters the umbra.
    pub const fn umbral_ingress(&self) -> Option<LunarEclipseContact<S>> {
        self.umbral_ingress
    }

    /// Returns U2 for a total eclipse.
    pub const fn totality_ingress(&self) -> Option<LunarEclipseContact<S>> {
        self.totality_ingress
    }

    /// Returns greatest eclipse.
    pub const fn maximum(&self) -> LunarEclipseMaximum<S> {
        self.maximum
    }

    /// Returns U3 for a total eclipse.
    pub const fn totality_egress(&self) -> Option<LunarEclipseContact<S>> {
        self.totality_egress
    }

    /// Returns U4 when the Moon leaves the umbra.
    pub const fn umbral_egress(&self) -> Option<LunarEclipseContact<S>> {
        self.umbral_egress
    }

    /// Returns P4.
    pub const fn penumbral_egress(&self) -> LunarEclipseContact<S> {
        self.penumbral_egress
    }

    /// Returns the complete penumbral phase P1-P4.
    pub fn penumbral_phase(&self) -> Result<LunarEclipsePhaseInterval<S>, Error> {
        Ok(LunarEclipsePhaseInterval {
            kind: LunarEclipseKind::Penumbral,
            interval: TimeInterval::new(
                self.penumbral_ingress.instant(),
                self.penumbral_egress.instant(),
            )?,
        })
    }

    /// Returns the partial phase U1-U4 when the Moon enters the umbra.
    pub fn partial_phase(&self) -> Result<Option<LunarEclipsePhaseInterval<S>>, Error> {
        match (self.umbral_ingress, self.umbral_egress) {
            (Some(start), Some(end)) => Ok(Some(LunarEclipsePhaseInterval {
                kind: LunarEclipseKind::Partial,
                interval: TimeInterval::new(start.instant(), end.instant())?,
            })),
            _ => Ok(None),
        }
    }

    /// Returns the total phase U2-U3 for a total eclipse.
    pub fn total_phase(&self) -> Result<Option<LunarEclipsePhaseInterval<S>>, Error> {
        match (self.totality_ingress, self.totality_egress) {
            (Some(start), Some(end)) => Ok(Some(LunarEclipsePhaseInterval {
                kind: LunarEclipseKind::Total,
                interval: TimeInterval::new(start.instant(), end.instant())?,
            })),
            _ => Ok(None),
        }
    }

    /// Returns the physical figures and terrestrial-shadow convention.
    pub const fn model(&self) -> LunarEclipseModel {
        self.model
    }

    /// Returns the Earth reference ellipsoid whose equatorial radius defined the shadows.
    pub const fn earth(&self) -> Earth {
        self.earth
    }

    /// Returns exact ephemeris model and kernel provenance.
    pub const fn ephemeris_provenance(&self) -> &EphemerisProvenance {
        &self.ephemeris
    }
}

/// Observer controls for reducing a global lunar eclipse to one fixed site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarEclipseVisibilityOptions {
    horizon: HorizonCriterion,
    low_altitude_threshold: Altitude,
    horizon_search: HorizonSearchOptions,
}

impl LunarEclipseVisibilityOptions {
    /// Constructs explicit visibility, low-altitude, and rise/set controls.
    pub const fn new(
        horizon: HorizonCriterion,
        low_altitude_threshold: Altitude,
        horizon_search: HorizonSearchOptions,
    ) -> Self {
        Self {
            horizon,
            low_altitude_threshold,
            horizon_search,
        }
    }

    /// Uses the model's lunar upper limb, a ten-degree low-altitude threshold, and standard search.
    pub const fn standard(model: LunarEclipseModel) -> Self {
        Self::new(
            HorizonCriterion::geometric_upper_limb(model.moon()),
            Altitude::from_finite(0.174_532_925_199_432_95),
            HorizonSearchOptions::standard(),
        )
    }

    /// Returns the altitude, coordinate stage, and disk point defining visibility.
    pub const fn horizon(self) -> HorizonCriterion {
        self.horizon
    }

    /// Returns the centre-altitude threshold below which samples carry a low-altitude warning.
    pub const fn low_altitude_threshold(self) -> Altitude {
        self.low_altitude_threshold
    }

    /// Returns the rise/set scanning and refinement controls.
    pub const fn horizon_search(self) -> HorizonSearchOptions {
        self.horizon_search
    }
}

/// Named global circumstance sampled for one terrestrial observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LunarEclipseVisibilityStage {
    /// P1.
    PenumbralIngress,
    /// U1.
    UmbralIngress,
    /// U2.
    TotalityIngress,
    /// Greatest eclipse.
    Greatest,
    /// U3.
    TotalityEgress,
    /// U4.
    UmbralEgress,
    /// P4.
    PenumbralEgress,
}

/// Natural-sky background inferred from topocentric vacuum solar-centre altitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LunarEclipseSkyBackground {
    /// The solar centre is at or above the astronomical horizon.
    Daylight,
    /// The solar centre is between 0 and −6 degrees.
    CivilTwilight,
    /// The solar centre is between −6 and −12 degrees.
    NauticalTwilight,
    /// The solar centre is between −12 and −18 degrees.
    AstronomicalTwilight,
    /// The solar centre is below −18 degrees.
    Night,
}

impl LunarEclipseSkyBackground {
    fn from_solar_altitude(altitude: Altitude) -> Self {
        let degrees = altitude.as_degrees();
        if degrees >= 0.0 {
            Self::Daylight
        } else if degrees >= -6.0 {
            Self::CivilTwilight
        } else if degrees >= -12.0 {
            Self::NauticalTwilight
        } else if degrees >= -18.0 {
            Self::AstronomicalTwilight
        } else {
            Self::Night
        }
    }
}

/// One fixed-site Moon observation at an eclipse contact or greatest eclipse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalLunarEclipseSample<S: TimeScale> {
    stage: LunarEclipseVisibilityStage,
    vacuum: VacuumObservedPlace<S>,
    observed: Option<ObservedPlace<S>>,
    solar_altitude: Altitude,
    sky_background: LunarEclipseSkyBackground,
    above_horizon: bool,
    low_altitude: bool,
}

impl<S: TimeScale> LocalLunarEclipseSample<S> {
    /// Returns the sampled eclipse circumstance.
    pub const fn stage(self) -> LunarEclipseVisibilityStage {
        self.stage
    }

    /// Returns the sample instant.
    pub const fn instant(self) -> Instant<S> {
        self.vacuum.reception_epoch()
    }

    /// Returns the topocentric vacuum Moon place.
    pub const fn vacuum_place(self) -> VacuumObservedPlace<S> {
        self.vacuum
    }

    /// Returns the refracted Moon place when the selected horizon criterion requested one.
    pub const fn observed_place(self) -> Option<ObservedPlace<S>> {
        self.observed
    }

    /// Returns the topocentric vacuum solar-centre altitude at this lunar circumstance.
    pub const fn solar_altitude(self) -> Altitude {
        self.solar_altitude
    }

    /// Returns the daylight, twilight, or night background classification.
    pub const fn sky_background(self) -> LunarEclipseSkyBackground {
        self.sky_background
    }

    /// Returns whether the selected centre or limb is above the selected horizon criterion.
    pub const fn is_above_horizon(self) -> bool {
        self.above_horizon
    }

    /// Returns whether a visible sample's centre altitude is at or below the warning threshold.
    pub const fn is_low_altitude(self) -> bool {
        self.low_altitude
    }
}

/// A visible portion of one nested penumbral, partial, or total eclipse phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisibleLunarEclipsePhase<S: TimeScale> {
    kind: LunarEclipseKind,
    interval: TimeInterval<S>,
    truncated_at_start: bool,
    truncated_at_end: bool,
}

impl<S: TimeScale> VisibleLunarEclipsePhase<S> {
    /// Returns the penumbral, partial, or total phase identity.
    pub const fn kind(self) -> LunarEclipseKind {
        self.kind
    }

    /// Returns the above-horizon intersection with the global phase.
    pub const fn interval(self) -> TimeInterval<S> {
        self.interval
    }

    /// Returns whether moonrise or the requested eclipse interval clipped the global phase start.
    pub const fn is_truncated_at_start(self) -> bool {
        self.truncated_at_start
    }

    /// Returns whether moonset or the requested eclipse interval clipped the global phase end.
    pub const fn is_truncated_at_end(self) -> bool {
        self.truncated_at_end
    }
}

/// Fixed-site visibility of one complete global lunar eclipse.
///
/// The result retains every sampled contact, the complete lunar rise/set search, explicit visible
/// phase intersections, the observer, EOP version, and the original global eclipse.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalLunarEclipseVisibility<S: TimeScale> {
    site: FixedSite,
    eclipse: GlobalLunarEclipse<S>,
    horizon: HorizonEventSearch<S>,
    samples: Vec<LocalLunarEclipseSample<S>>,
    visible_phases: Vec<VisibleLunarEclipsePhase<S>>,
    low_altitude_threshold: Altitude,
    earth_orientation_version: String,
}

impl<S: TimeScale> LocalLunarEclipseVisibility<S> {
    /// Returns the fixed terrestrial observer.
    pub const fn site(&self) -> &FixedSite {
        &self.site
    }

    /// Returns the global eclipse reduced for this observer.
    pub const fn eclipse(&self) -> &GlobalLunarEclipse<S> {
        &self.eclipse
    }

    /// Returns the complete Moon rise/set/transit search over P1-P4.
    pub const fn horizon_events(&self) -> &HorizonEventSearch<S> {
        &self.horizon
    }

    /// Returns contact and greatest-eclipse samples in chronological order.
    pub fn samples(&self) -> &[LocalLunarEclipseSample<S>] {
        &self.samples
    }

    /// Returns every positive-duration above-horizon phase intersection.
    pub fn visible_phases(&self) -> &[VisibleLunarEclipsePhase<S>] {
        &self.visible_phases
    }

    /// Returns the centre-altitude threshold used for low-altitude warnings.
    pub const fn low_altitude_threshold(&self) -> Altitude {
        self.low_altitude_threshold
    }

    /// Returns whether a visible retained sample or rise/set boundary is low.
    pub fn has_low_altitude_warning(&self) -> bool {
        self.samples.iter().any(|sample| sample.is_low_altitude())
            || (!self.visible_phases.is_empty()
                && self.horizon.events().iter().any(|event| {
                    matches!(event.kind(), HorizonEventKind::Rise | HorizonEventKind::Set)
                        && event
                            .observed_place()
                            .map(ObservedPlace::horizontal)
                            .unwrap_or_else(|| event.vacuum_place().horizontal())
                            .altitude()
                            .as_radians()
                            <= self.low_altitude_threshold.as_radians()
                }))
    }

    /// Returns the exact observed Earth-orientation table version.
    pub fn earth_orientation_version(&self) -> &str {
        &self.earth_orientation_version
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
    PenumbralExterior,
    UmbralExterior,
    UmbralInterior,
}

impl ContactCriterion {
    fn residual<S: TimeScale>(self, geometry: LunarShadowGeometry<S>) -> f64 {
        let separation = geometry.axis_separation().as_radians();
        let moon = geometry.lunar_semidiameter().as_radians();
        match self {
            Self::PenumbralExterior => separation - geometry.penumbral_radius().as_radians() - moon,
            Self::UmbralExterior => separation - geometry.umbral_radius().as_radians() - moon,
            Self::UmbralInterior => separation - (geometry.umbral_radius().as_radians() - moon),
        }
    }
}

struct LunarEclipseSampler<'context, 'data, 'earth, E, P: EphemerisProvider + ?Sized> {
    astrometry: Astrometry<'context, 'data, E, P>,
    earth: &'earth Earth,
    light_time: ReceptionLightTimeOptions,
    model: LunarEclipseModel,
}

impl<'context, 'data, 'earth, E, P: EphemerisProvider + ?Sized>
    LunarEclipseSampler<'context, 'data, 'earth, E, P>
{
    const MAXIMUM_HALF_WINDOW: Duration =
        Duration::from_nanoseconds(12 * 60 * 60 * Duration::NANOSECONDS_PER_SECOND);
    const CONTACT_STEP: Duration =
        Duration::from_nanoseconds(30 * 60 * Duration::NANOSECONDS_PER_SECOND);
    const MAX_CONTACT_STEPS: u32 = 16;

    const fn new(
        astrometry: Astrometry<'context, 'data, E, P>,
        earth: &'earth Earth,
        options: LunarEclipseSearchOptions,
    ) -> Self {
        Self {
            astrometry,
            earth,
            light_time: options.angular_search().light_time(),
            model: options.model(),
        }
    }

    fn observe<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<LunarShadowGeometry<S>, Error> {
        if maximum_evaluations.saturating_sub(*evaluations) < 2 {
            return Err(Error::EvaluationLimitExceeded {
                maximum: maximum_evaluations,
            });
        }
        *evaluations += 2;
        let apparent_moon = self.astrometry.geocentric_apparent_place(
            CelestialBody::Moon,
            epoch,
            self.light_time,
        )?;
        let apparent_sun = self
            .astrometry
            .solar_apparent_place(epoch, self.light_time)?;
        LunarShadowGeometry::new(self.earth, apparent_moon, apparent_sun, self.model)
    }

    fn maximum_near<S: TimeScale>(
        &self,
        seed: Instant<S>,
        options: LunarEclipseSearchOptions,
    ) -> Result<LunarEclipseMaximum<S>, Error> {
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
                    .map(|geometry| geometry.axis_separation().as_radians())
            },
        )?;
        let geometry = self.observe(
            refined.instant(),
            &mut evaluations,
            controls.max_evaluations(),
        )?;
        Ok(LunarEclipseMaximum {
            geometry,
            evidence: ExtremumEvidence::new(
                refined.bracket_start(),
                refined.bracket_end(),
                refined.time_uncertainty(),
                refined.iterations(),
                evaluations,
            ),
        })
    }

    fn classify<S: TimeScale>(
        geometry: LunarShadowGeometry<S>,
        tolerance: Angle,
    ) -> Option<LunarEclipseKind> {
        let tolerance = tolerance.as_radians();
        if ContactCriterion::PenumbralExterior.residual(geometry) > tolerance {
            return None;
        }
        if ContactCriterion::UmbralExterior.residual(geometry) > tolerance {
            return Some(LunarEclipseKind::Penumbral);
        }
        if ContactCriterion::UmbralInterior.residual(geometry) >= -tolerance {
            Some(LunarEclipseKind::Partial)
        } else {
            Some(LunarEclipseKind::Total)
        }
    }

    fn tangent_contact<S: TimeScale>(
        maximum: LunarEclipseMaximum<S>,
        criterion: ContactCriterion,
        kind: LunarEclipseContactKind,
    ) -> Result<LunarEclipseContact<S>, Error> {
        let geometry = maximum.geometry();
        let evidence = maximum.evidence();
        Ok(LunarEclipseContact {
            kind,
            geometry,
            evidence: EventEvidence::new(
                evidence.bracket_start(),
                evidence.bracket_end(),
                evidence.time_uncertainty(),
                Angle::from_radians(criterion.residual(geometry))?,
                0,
                evidence.evaluations(),
            ),
        })
    }

    fn refine_contact<S: TimeScale>(
        &self,
        maximum: LunarShadowGeometry<S>,
        direction: ContactDirection,
        criterion: ContactCriterion,
        kind: LunarEclipseContactKind,
        options: LunarEclipseSearchOptions,
    ) -> Result<LunarEclipseContact<S>, Error> {
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
            bracket.ok_or(Error::LunarEclipseContactNotBracketed {
                contact: kind.description(),
                maximum_tai_nanoseconds: maximum.instant().tai_nanoseconds_since_1900(),
            })?;
        let microsecond = Duration::from_nanoseconds(1_000);
        let time_tolerance = controls.time_tolerance().min(microsecond);
        let root = BracketedRootSearch::refine(
            bracket_start,
            bracket_end,
            time_tolerance,
            controls.max_refinement_iterations(),
            |epoch| {
                self.observe(epoch, &mut evaluations, controls.max_evaluations())
                    .map(|geometry| criterion.residual(geometry))
            },
        )?;
        let geometry =
            self.observe(root.instant(), &mut evaluations, controls.max_evaluations())?;
        let residual = criterion.residual(geometry);
        if residual.abs() > controls.angular_tolerance().as_radians() {
            return Err(Error::AngularResidualExceeded {
                event: kind.description(),
                residual_radians: residual.abs(),
                tolerance_radians: controls.angular_tolerance().as_radians(),
            });
        }
        Ok(LunarEclipseContact {
            kind,
            geometry,
            evidence: EventEvidence::new(
                root.bracket_start(),
                root.bracket_end(),
                root.time_uncertainty(),
                Angle::from_radians(residual)?,
                root.iterations(),
                evaluations,
            ),
        })
    }

    fn contact_pair<S: TimeScale>(
        &self,
        maximum: LunarEclipseMaximum<S>,
        criterion: ContactCriterion,
        ingress: LunarEclipseContactKind,
        egress: LunarEclipseContactKind,
        options: LunarEclipseSearchOptions,
    ) -> Result<(LunarEclipseContact<S>, LunarEclipseContact<S>), Error> {
        let residual = criterion.residual(maximum.geometry());
        if residual.abs() <= options.angular_search().angular_tolerance().as_radians() {
            return Ok((
                Self::tangent_contact(maximum, criterion, ingress)?,
                Self::tangent_contact(maximum, criterion, egress)?,
            ));
        }
        Ok((
            self.refine_contact(
                maximum.geometry(),
                ContactDirection::Before,
                criterion,
                ingress,
                options,
            )?,
            self.refine_contact(
                maximum.geometry(),
                ContactDirection::After,
                criterion,
                egress,
                options,
            )?,
        ))
    }
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Events<'context, 'data, E, P> {
    /// Finds complete global lunar eclipses whose greatest instants lie in a closed interval.
    ///
    /// Apparent geocentric full Moons seed bounded minima of the Moon-to-shadow-axis separation.
    /// P1/P4 solve penumbral exterior tangencies, U1/U4 solve umbral exterior tangencies, and
    /// U2/U3 solve umbral interior tangencies. Results retain apparent positions, physical model,
    /// numerical evidence, and ephemeris provenance. Atmospheric visibility from a terrestrial
    /// site is a separate observer workflow.
    pub fn global_lunar_eclipses_in<S: TimeScale>(
        &self,
        earth: &Earth,
        interval: TimeInterval<S>,
        options: LunarEclipseSearchOptions,
    ) -> Result<Vec<GlobalLunarEclipse<S>>, Error> {
        let controls = options.angular_search();
        let seed_interval = TimeInterval::new(
            interval
                .start()
                .checked_sub(LunarEclipseSampler::<E, P>::MAXIMUM_HALF_WINDOW)?,
            interval
                .end()
                .checked_add(LunarEclipseSampler::<E, P>::MAXIMUM_HALF_WINDOW)?,
        )?;
        let seeds = self.moon_phase_angle_in(
            seed_interval,
            MoonPhase::FullMoon.target_longitude_difference(),
            controls,
        )?;
        let sampler = LunarEclipseSampler::new(self.astrometry, earth, options);
        let ephemeris = self
            .astrometry
            .ephemeris()
            .provenance()
            .map_err(crate::astro::Error::from)?;
        let mut eclipses = Vec::new();

        for seed in seeds {
            let maximum = sampler.maximum_near(seed.instant(), options)?;
            if !interval.contains(maximum.instant()) {
                continue;
            }
            let Some(kind) = LunarEclipseSampler::<E, P>::classify(
                maximum.geometry(),
                controls.angular_tolerance(),
            ) else {
                continue;
            };
            let (penumbral_ingress, penumbral_egress) = sampler.contact_pair(
                maximum,
                ContactCriterion::PenumbralExterior,
                LunarEclipseContactKind::PenumbralIngress,
                LunarEclipseContactKind::PenumbralEgress,
                options,
            )?;
            let (umbral_ingress, umbral_egress) =
                if matches!(kind, LunarEclipseKind::Partial | LunarEclipseKind::Total) {
                    let (ingress, egress) = sampler.contact_pair(
                        maximum,
                        ContactCriterion::UmbralExterior,
                        LunarEclipseContactKind::UmbralIngress,
                        LunarEclipseContactKind::UmbralEgress,
                        options,
                    )?;
                    (Some(ingress), Some(egress))
                } else {
                    (None, None)
                };
            let (totality_ingress, totality_egress) = if kind == LunarEclipseKind::Total {
                let (ingress, egress) = sampler.contact_pair(
                    maximum,
                    ContactCriterion::UmbralInterior,
                    LunarEclipseContactKind::TotalityIngress,
                    LunarEclipseContactKind::TotalityEgress,
                    options,
                )?;
                (Some(ingress), Some(egress))
            } else {
                (None, None)
            };
            eclipses.push(GlobalLunarEclipse {
                kind,
                penumbral_ingress,
                umbral_ingress,
                totality_ingress,
                maximum,
                totality_egress,
                umbral_egress,
                penumbral_egress,
                model: options.model(),
                earth: *earth,
                ephemeris: ephemeris.clone(),
            });
        }

        eclipses.sort_by_key(|eclipse| eclipse.maximum().instant().tai_nanoseconds_since_1900());
        eclipses.dedup_by_key(|eclipse| eclipse.maximum().instant().tai_nanoseconds_since_1900());
        Ok(eclipses)
    }
}

impl<'context, 'data, 'eop, P: EphemerisProvider + ?Sized>
    Events<'context, 'data, EarthOrientationTable<'eop>, P>
{
    /// Reduces one global lunar eclipse to fixed-site visibility using observed Earth rotation.
    ///
    /// Moonrise and moonset are solved over P1-P4 with the selected centre-or-limb criterion.
    /// Each nested global phase is intersected with the resulting above-horizon intervals.
    /// Contacts and greatest eclipse retain topocentric places and explicit low-altitude flags.
    pub fn local_lunar_eclipse_visibility<S: TimeScale>(
        &self,
        site: &FixedSite,
        eclipse: &GlobalLunarEclipse<S>,
        options: LunarEclipseVisibilityOptions,
    ) -> Result<LocalLunarEclipseVisibility<S>, Error> {
        let ephemeris = self
            .astrometry
            .ephemeris()
            .provenance()
            .map_err(crate::astro::Error::from)?;
        if &ephemeris != eclipse.ephemeris_provenance() {
            return Err(Error::LunarEclipseVisibilityEphemerisMismatch);
        }
        if site.reference_ellipsoid() != eclipse.earth().reference_ellipsoid() {
            return Err(Error::LunarEclipseVisibilityEarthMismatch);
        }

        let penumbral_phase = eclipse.penumbral_phase()?;
        let horizon = self.horizon_events_in(
            site,
            CelestialBody::Moon,
            penumbral_phase.interval(),
            options.horizon(),
            options.horizon_search(),
        )?;
        let mut stages = Vec::new();
        stages.push((
            LunarEclipseVisibilityStage::PenumbralIngress,
            eclipse.penumbral_ingress().instant(),
        ));
        if let Some(contact) = eclipse.umbral_ingress() {
            stages.push((
                LunarEclipseVisibilityStage::UmbralIngress,
                contact.instant(),
            ));
        }
        if let Some(contact) = eclipse.totality_ingress() {
            stages.push((
                LunarEclipseVisibilityStage::TotalityIngress,
                contact.instant(),
            ));
        }
        stages.push((
            LunarEclipseVisibilityStage::Greatest,
            eclipse.maximum().instant(),
        ));
        if let Some(contact) = eclipse.totality_egress() {
            stages.push((
                LunarEclipseVisibilityStage::TotalityEgress,
                contact.instant(),
            ));
        }
        if let Some(contact) = eclipse.umbral_egress() {
            stages.push((LunarEclipseVisibilityStage::UmbralEgress, contact.instant()));
        }
        stages.push((
            LunarEclipseVisibilityStage::PenumbralEgress,
            eclipse.penumbral_egress().instant(),
        ));
        stages.sort_by_key(|(_, instant)| instant.tai_nanoseconds_since_1900());

        let mut samples = Vec::with_capacity(stages.len());
        for (stage, instant) in stages {
            samples.push(self.local_lunar_eclipse_sample(site, stage, instant, options)?);
        }
        let starts_visible = samples
            .first()
            .is_some_and(|sample| sample.is_above_horizon());
        let above_horizon = Self::lunar_above_horizon_intervals(
            penumbral_phase.interval(),
            &horizon,
            starts_visible,
        )?;
        let mut phases = Vec::new();
        phases.push(penumbral_phase);
        if let Some(partial) = eclipse.partial_phase()? {
            phases.push(partial);
        }
        if let Some(total) = eclipse.total_phase()? {
            phases.push(total);
        }
        let mut visible_phases = Vec::new();
        for phase in phases {
            for visible in &above_horizon {
                let start = if phase.interval().start() > visible.start() {
                    phase.interval().start()
                } else {
                    visible.start()
                };
                let end = if phase.interval().end() < visible.end() {
                    phase.interval().end()
                } else {
                    visible.end()
                };
                if start < end {
                    visible_phases.push(VisibleLunarEclipsePhase {
                        kind: phase.kind(),
                        interval: TimeInterval::new(start, end)?,
                        truncated_at_start: start > phase.interval().start(),
                        truncated_at_end: end < phase.interval().end(),
                    });
                }
            }
        }
        visible_phases.sort_by_key(|phase| {
            (
                phase.interval().start().tai_nanoseconds_since_1900(),
                match phase.kind() {
                    LunarEclipseKind::Penumbral => 0_u8,
                    LunarEclipseKind::Partial => 1,
                    LunarEclipseKind::Total => 2,
                },
            )
        });

        Ok(LocalLunarEclipseVisibility {
            site: site.clone(),
            eclipse: eclipse.clone(),
            horizon,
            samples,
            visible_phases,
            low_altitude_threshold: options.low_altitude_threshold(),
            earth_orientation_version: self
                .astrometry
                .time_context()
                .earth_orientation()
                .version()
                .to_owned(),
        })
    }

    fn local_lunar_eclipse_sample<S: TimeScale>(
        &self,
        site: &FixedSite,
        stage: LunarEclipseVisibilityStage,
        instant: Instant<S>,
        options: LunarEclipseVisibilityOptions,
    ) -> Result<LocalLunarEclipseSample<S>, Error> {
        let observer = self.astrometry.fixed_observer_at(site, instant)?;
        let vacuum = observer
            .vacuum_observed_place(CelestialBody::Moon, options.horizon_search().light_time())?;
        let solar_altitude = observer
            .vacuum_observed_place(CelestialBody::Sun, options.horizon_search().light_time())?
            .horizontal()
            .altitude();
        let sky_background = LunarEclipseSkyBackground::from_solar_altitude(solar_altitude);
        let observed = match options.horizon().reference() {
            HorizonReference::Vacuum => None,
            HorizonReference::Refracted(conditions) => Some(vacuum.apply_refraction(conditions)?),
        };
        let horizontal = observed
            .map(ObservedPlace::horizontal)
            .unwrap_or_else(|| vacuum.horizontal());
        let limb_offset = match options.horizon().disk_point() {
            HorizonDiskPoint::Center => 0.0,
            HorizonDiskPoint::UpperLimb(figure) => {
                vacuum.apparent_disk(figure)?.semidiameter().as_radians()
            }
            HorizonDiskPoint::LowerLimb(figure) => {
                -vacuum.apparent_disk(figure)?.semidiameter().as_radians()
            }
        };
        let above_horizon = horizontal.altitude().as_radians() + limb_offset
            >= options.horizon().altitude().as_radians();
        let low_altitude = above_horizon
            && horizontal.altitude().as_radians() <= options.low_altitude_threshold().as_radians();
        Ok(LocalLunarEclipseSample {
            stage,
            vacuum,
            observed,
            above_horizon,
            solar_altitude,
            sky_background,
            low_altitude,
        })
    }

    fn lunar_above_horizon_intervals<S: TimeScale>(
        interval: TimeInterval<S>,
        horizon: &HorizonEventSearch<S>,
        starts_visible: bool,
    ) -> Result<Vec<TimeInterval<S>>, Error> {
        let mut visible_start = starts_visible.then_some(interval.start());
        let mut intervals = Vec::new();
        for event in horizon.events() {
            match event.kind() {
                HorizonEventKind::Rise => {
                    if visible_start.is_none() {
                        visible_start = Some(event.instant());
                    }
                }
                HorizonEventKind::Set => {
                    if let Some(start) = visible_start.take()
                        && start < event.instant()
                    {
                        intervals.push(TimeInterval::new(start, event.instant())?);
                    }
                }
                HorizonEventKind::UpperTransit | HorizonEventKind::LowerTransit => {}
            }
        }
        if let Some(start) = visible_start
            && start < interval.end()
        {
            intervals.push(TimeInterval::new(start, interval.end())?);
        }
        Ok(intervals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_model_names_nasa_catalog_shadow_convention() {
        let model = LunarEclipseModel::standard();
        assert_eq!(model.sun().body(), CelestialBody::Sun);
        assert_eq!(model.moon().body(), CelestialBody::Moon);
        assert_eq!(model.shadow(), LunarShadowConvention::DANJON);
        assert_eq!(model.shadow().earth_parallax_scale(), 1.01);
        assert_eq!(model.shadow().shadow_radius_scale(), 1.0);
    }

    #[test]
    fn signed_magnitude_preserves_misses_partial_and_complete_immersion() {
        assert_eq!(
            LunarEclipseMagnitude::from_geometry(1.0, 0.25, 1.5).as_ratio(),
            -0.5
        );
        assert_eq!(
            LunarEclipseMagnitude::from_geometry(1.0, 0.25, 1.0).as_ratio(),
            0.5
        );
        assert_eq!(
            LunarEclipseMagnitude::from_geometry(1.0, 0.25, 0.75).as_ratio(),
            1.0
        );
    }
}
