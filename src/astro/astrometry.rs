use core::{f64::consts::FRAC_PI_2, fmt};

use crate::{
    catalog::{CatalogProperMotion, InfiniteCatalogPlace, SpatialCatalogPlace},
    earth::{FixedSite, SiteVelocityModel, TopocentricFrame},
    ephem::{CelestialBody, Ephemeris, EphemerisQuery, RelativeState},
    frame::{
        Bcrs, Cirs, EclipticDirectionAt, EclipticLatitude, EclipticLongitude, EquatorialDirection,
        EquatorialDirectionAt, FrameRotation, Frames, Gcrs, HorizontalDirection, Icrs,
        TrueEclipticEquinoxOfDate, TrueEquatorEquinoxOfDate,
    },
    math::{Angle, Declination, Direction, Length, RightAscension, Speed, Vector3},
    time::{
        Duration, EarthAttitudeTable, EarthOrientationTable, Hifitime, Instant, JulianDate, Tcb,
        Tdb, TimeContext, TimeScale, TimeScaleModel, Ut1,
    },
};

use super::{
    AtmosphericConditions, Error, SolarApparentPlace, SolarLightDeflection, SolarTimeSolution,
};

/// Explicit convergence controls for one-way reception light time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceptionLightTimeOptions {
    time_tolerance: Duration,
    max_iterations: u32,
}

impl ReceptionLightTimeOptions {
    /// Constructs a positive time tolerance and non-zero iteration budget.
    pub fn new(time_tolerance: Duration, max_iterations: u32) -> Result<Self, Error> {
        if time_tolerance <= Duration::ZERO {
            return Err(Error::InvalidLightTimeTolerance {
                nanoseconds: time_tolerance.as_nanoseconds(),
            });
        }
        if max_iterations == 0 {
            return Err(Error::InvalidLightTimeIterationLimit { max_iterations });
        }
        Ok(Self {
            time_tolerance,
            max_iterations,
        })
    }

    /// Returns the standard one-nanosecond tolerance and ten-iteration budget.
    pub const fn standard() -> Self {
        Self {
            time_tolerance: Duration::from_nanoseconds(1),
            max_iterations: 10,
        }
    }

    /// Returns the required absolute fixed-point time residual.
    pub const fn time_tolerance(self) -> Duration {
        self.time_tolerance
    }

    /// Returns the maximum number of target-state iterations.
    pub const fn max_iterations(self) -> u32 {
        self.max_iterations
    }
}

/// A converged one-way reception light-time solution in BCRS axes.
///
/// The target position is evaluated at [`Self::emission_epoch`], while the
/// observer position is evaluated at [`Self::reception_epoch`]. The result
/// intentionally carries no relative velocity because its two contributing
/// states have different epochs.
pub struct ReceptionLightTime<S: TimeScale> {
    target: CelestialBody,
    observer: CelestialBody,
    reception_epoch: Instant<S>,
    emission_epoch: Instant<S>,
    light_time: Duration,
    relative_position: Vector3<Bcrs, Length>,
    direction: Direction<Bcrs>,
    distance: Length,
    iterations: u32,
    residual: Duration,
}

impl<S: TimeScale> ReceptionLightTime<S> {
    /// Returns the observed target.
    pub const fn target(self) -> CelestialBody {
        self.target
    }

    /// Returns the receiving observer.
    pub const fn observer(self) -> CelestialBody {
        self.observer
    }

    /// Returns the observer reception epoch.
    pub const fn reception_epoch(self) -> Instant<S> {
        self.reception_epoch
    }

    /// Returns the target emission epoch.
    pub const fn emission_epoch(self) -> Instant<S> {
        self.emission_epoch
    }

    /// Returns the converged one-way light time.
    pub const fn light_time(self) -> Duration {
        self.light_time
    }

    /// Returns target-at-emission minus observer-at-reception position.
    pub const fn relative_position(self) -> Vector3<Bcrs, Length> {
        self.relative_position
    }

    /// Returns the natural BCRS line-of-sight direction before aberration.
    pub const fn direction(self) -> Direction<Bcrs> {
        self.direction
    }

    /// Returns the target-at-emission to observer-at-reception distance.
    pub const fn distance(self) -> Length {
        self.distance
    }

    /// Returns the number of completed target-state iterations.
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns the final absolute light-time fixed-point residual.
    pub const fn residual(self) -> Duration {
        self.residual
    }
}

impl<S: TimeScale> Copy for ReceptionLightTime<S> {}

impl<S: TimeScale> Clone for ReceptionLightTime<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for ReceptionLightTime<S> {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target
            && self.observer == other.observer
            && self.reception_epoch == other.reception_epoch
            && self.emission_epoch == other.emission_epoch
            && self.light_time == other.light_time
            && self.relative_position == other.relative_position
            && self.direction == other.direction
            && self.distance == other.distance
            && self.iterations == other.iterations
            && self.residual == other.residual
    }
}

impl<S: TimeScale> fmt::Debug for ReceptionLightTime<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReceptionLightTime")
            .field("target", &self.target)
            .field("observer", &self.observer)
            .field("reception_epoch", &self.reception_epoch)
            .field("emission_epoch", &self.emission_epoch)
            .field("light_time", &self.light_time)
            .field("relative_position", &self.relative_position)
            .field("direction", &self.direction)
            .field("distance", &self.distance)
            .field("iterations", &self.iterations)
            .field("residual", &self.residual)
            .finish()
    }
}

/// A finite target's geocentric apparent place at one reception epoch.
///
/// The target is evaluated at the retained emission epoch and the Earth at the
/// retained reception epoch. The result applies finite-distance solar
/// point-mass deflection, relativistic annual aberration, and IAU 2006/2000A
/// orientation. It excludes station parallax, atmospheric refraction, and
/// Shapiro delay.
pub struct GeocentricApparentPlace<S: TimeScale> {
    reception: ReceptionLightTime<S>,
    gcrs_direction: EquatorialDirectionAt<Gcrs, S>,
    true_equatorial: EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>,
    true_ecliptic: EclipticDirectionAt<TrueEclipticEquinoxOfDate, S>,
    solar_light_deflection: SolarLightDeflection<S>,
}

impl<S: TimeScale> GeocentricApparentPlace<S> {
    const fn new(
        reception: ReceptionLightTime<S>,
        gcrs_direction: EquatorialDirectionAt<Gcrs, S>,
        true_equatorial: EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>,
        true_ecliptic: EclipticDirectionAt<TrueEclipticEquinoxOfDate, S>,
        solar_light_deflection: SolarLightDeflection<S>,
    ) -> Self {
        Self {
            reception,
            gcrs_direction,
            true_equatorial,
            true_ecliptic,
            solar_light_deflection,
        }
    }

    /// Returns the observed target.
    pub const fn target(self) -> CelestialBody {
        self.reception.target()
    }

    /// Returns the complete dual-epoch reception light-time solution.
    pub const fn reception_light_time(self) -> ReceptionLightTime<S> {
        self.reception
    }

    /// Returns the geocentric reception epoch.
    pub const fn reception_epoch(self) -> Instant<S> {
        self.reception.reception_epoch()
    }

    /// Returns the retarded target emission epoch.
    pub const fn emission_epoch(self) -> Instant<S> {
        self.reception.emission_epoch()
    }

    /// Returns the converged one-way light time.
    pub const fn light_time(self) -> Duration {
        self.reception.light_time()
    }

    /// Returns the epoch-bound apparent GCRS direction.
    pub const fn gcrs_direction(self) -> EquatorialDirectionAt<Gcrs, S> {
        self.gcrs_direction
    }

    /// Returns the apparent direction on true equator and equinox of date axes.
    pub const fn true_equatorial(self) -> EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S> {
        self.true_equatorial
    }

    /// Returns the apparent direction on true ecliptic and equinox of date axes.
    pub const fn true_ecliptic(self) -> EclipticDirectionAt<TrueEclipticEquinoxOfDate, S> {
        self.true_ecliptic
    }

    /// Returns the apparent geocentric right ascension.
    pub const fn right_ascension(self) -> RightAscension {
        self.true_equatorial.coordinates().right_ascension()
    }

    /// Returns the apparent geocentric declination.
    pub const fn declination(self) -> Declination {
        self.true_equatorial.coordinates().declination()
    }

    /// Returns the apparent geocentric longitude on the date true ecliptic.
    pub const fn longitude(self) -> EclipticLongitude {
        self.true_ecliptic.coordinates().longitude()
    }

    /// Returns the apparent geocentric latitude on the date true ecliptic.
    pub const fn latitude(self) -> EclipticLatitude {
        self.true_ecliptic.coordinates().latitude()
    }

    /// Returns the target-at-emission to Earth-at-reception distance.
    pub const fn distance(self) -> Length {
        self.reception.distance()
    }

    /// Returns the finite-distance solar point-mass correction diagnostics.
    pub const fn solar_light_deflection(self) -> SolarLightDeflection<S> {
        self.solar_light_deflection
    }

    /// Returns the number of completed light-time iterations.
    pub const fn iterations(self) -> u32 {
        self.reception.iterations()
    }

    /// Returns the final absolute light-time fixed-point residual.
    pub const fn light_time_residual(self) -> Duration {
        self.reception.residual()
    }
}

impl<S: TimeScale> Copy for GeocentricApparentPlace<S> {}

impl<S: TimeScale> Clone for GeocentricApparentPlace<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for GeocentricApparentPlace<S> {
    fn eq(&self, other: &Self) -> bool {
        self.reception == other.reception
            && self.gcrs_direction == other.gcrs_direction
            && self.true_equatorial == other.true_equatorial
            && self.true_ecliptic == other.true_ecliptic
            && self.solar_light_deflection == other.solar_light_deflection
    }
}

impl<S: TimeScale> fmt::Debug for GeocentricApparentPlace<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeocentricApparentPlace")
            .field("reception", &self.reception)
            .field("gcrs_direction", &self.gcrs_direction)
            .field("true_equatorial", &self.true_equatorial)
            .field("true_ecliptic", &self.true_ecliptic)
            .field("solar_light_deflection", &self.solar_light_deflection)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TerrestrialObservationParameters {
    local_earth_rotation_angle: f64,
    local_polar_motion_x: f64,
    local_polar_motion_y: f64,
    latitude_sine: f64,
    latitude_cosine: f64,
    diurnal_aberration: f64,
}

impl TerrestrialObservationParameters {
    fn from_sofa(parameters: sofars::astro::IauAstrom) -> Self {
        Self {
            local_earth_rotation_angle: parameters.eral,
            local_polar_motion_x: parameters.xpl,
            local_polar_motion_y: parameters.ypl,
            latitude_sine: parameters.sphi,
            latitude_cosine: parameters.cphi,
            diurnal_aberration: parameters.diurab,
        }
    }

    fn with_refraction(self, conditions: AtmosphericConditions) -> sofars::astro::IauAstrom {
        let (refraction_a, refraction_b) = conditions.sofa_coefficients();
        sofars::astro::IauAstrom {
            eral: self.local_earth_rotation_angle,
            xpl: self.local_polar_motion_x,
            ypl: self.local_polar_motion_y,
            sphi: self.latitude_sine,
            cphi: self.latitude_cosine,
            diurab: self.diurnal_aberration,
            refa: refraction_a,
            refb: refraction_b,
            ..sofars::astro::IauAstrom::default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BarycentricObserverState<S: TimeScale> {
    position: Vector3<Bcrs, Length>,
    velocity: Vector3<Bcrs, Speed>,
    epoch: Instant<S>,
}

#[derive(Debug, Clone, Copy)]
struct FixedReception<S: TimeScale> {
    emission_epoch: Instant<S>,
    light_time: Duration,
    direction: Direction<Bcrs>,
    distance: Length,
    target_reception_position: Vector3<Bcrs, Length>,
    target_barycentric_position: Vector3<Bcrs, Length>,
    iterations: u32,
    residual: Duration,
}

#[derive(Debug, Clone, Copy)]
struct SolarDeflectionInput<S: TimeScale> {
    target: CelestialBody,
    reception_epoch: Instant<S>,
    emission_epoch: Instant<S>,
    observer_barycentric_position: Vector3<Bcrs, Length>,
    target_reception_position: Vector3<Bcrs, Length>,
    target_emission_position: Vector3<Bcrs, Length>,
    source_direction: Direction<Bcrs>,
    target_distance: Length,
}

#[derive(Debug, Clone, Copy)]
struct SolarDeflectionComputation<S: TimeScale> {
    direction: Direction<Bcrs>,
    diagnostics: SolarLightDeflection<S>,
    sun_reception_position: Vector3<Bcrs, Length>,
}

fn apply_finite_solar_light_deflection<S: TimeScale>(
    ephemeris: &Ephemeris,
    input: SolarDeflectionInput<S>,
) -> Result<SolarDeflectionComputation<S>, Error> {
    if input.target == CelestialBody::Sun {
        let (direction, diagnostics) = SolarLightDeflection::for_sun(
            input.emission_epoch,
            input.source_direction,
            input.target_distance,
        )?;
        return Ok(SolarDeflectionComputation {
            direction,
            diagnostics,
            sun_reception_position: input.target_reception_position,
        });
    }

    let barycentre = CelestialBody::SolarSystemBarycenter;
    let sun_at_reception = ephemeris.state(EphemerisQuery::new(
        CelestialBody::Sun,
        barycentre,
        input.reception_epoch,
    ))?;
    let sun_to_observer_at_reception = input
        .observer_barycentric_position
        .checked_sub(sun_at_reception.position())?;
    let body_to_observer_metres = sun_to_observer_at_reception.canonical_components();
    let source_components = input.source_direction.components();
    let passage_offset_seconds = (source_components[0] * body_to_observer_metres[0]
        + source_components[1] * body_to_observer_metres[1]
        + source_components[2] * body_to_observer_metres[2])
        / Length::METRES_PER_LIGHT_SECOND;
    let passage_offset =
        Duration::from_seconds_f64(passage_offset_seconds.min(0.0)).map_err(Error::from)?;
    let deflector_epoch = input.reception_epoch.checked_add(passage_offset)?;
    let sun_at_passage = ephemeris.state(EphemerisQuery::new(
        CelestialBody::Sun,
        barycentre,
        deflector_epoch,
    ))?;
    let sun_to_observer = input
        .observer_barycentric_position
        .checked_sub(sun_at_passage.position())?;
    let sun_to_target = input
        .target_emission_position
        .checked_sub(sun_at_passage.position())?;
    let (direction, diagnostics) = SolarLightDeflection::apply_to(
        deflector_epoch,
        input.source_direction,
        input.target_distance,
        sun_to_observer,
        sun_to_target,
    )?;

    Ok(SolarDeflectionComputation {
        direction,
        diagnostics,
        sun_reception_position: sun_at_reception.position(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SofaCatalogData {
    right_ascension: f64,
    declination: f64,
    right_ascension_rate: f64,
    declination_rate: f64,
    parallax_arcseconds: f64,
    radial_velocity_kilometres_per_second: f64,
}

impl SofaCatalogData {
    fn from_infinite(catalog: InfiniteCatalogPlace) -> Result<Self, Error> {
        let direction = catalog.direction();
        let declination = direction.declination();
        Ok(Self {
            right_ascension: direction.right_ascension().as_radians(),
            declination: declination.as_radians(),
            right_ascension_rate: catalog
                .proper_motion()
                .right_ascension_radians_per_julian_year_at(declination)?,
            declination_rate: catalog
                .proper_motion()
                .declination_radians_per_julian_year(),
            parallax_arcseconds: 0.0,
            radial_velocity_kilometres_per_second: 0.0,
        })
    }

    fn from_spatial(catalog: SpatialCatalogPlace) -> Result<Self, Error> {
        let data = catalog.sofa_catalog_data()?;
        Ok(Self {
            right_ascension: data[0],
            declination: data[1],
            right_ascension_rate: data[2],
            declination_rate: data[3],
            parallax_arcseconds: data[4],
            radial_velocity_kilometres_per_second: data[5],
        })
    }

    fn coordinate_components(self, elapsed_julian_years: f64, observer: [f64; 3]) -> [f64; 3] {
        sofars::astro::pmpx(
            self.right_ascension,
            self.declination,
            self.right_ascension_rate,
            self.declination_rate,
            self.parallax_arcseconds,
            self.radial_velocity_kilometres_per_second,
            elapsed_julian_years,
            observer,
        )
    }

    fn roemer_time_offset_julian_years(self, observer: [f64; 3]) -> f64 {
        let (right_ascension_sine, right_ascension_cosine) = self.right_ascension.sin_cos();
        let (declination_sine, declination_cosine) = self.declination.sin_cos();
        let reference_direction = [
            right_ascension_cosine * declination_cosine,
            right_ascension_sine * declination_cosine,
            declination_sine,
        ];
        let projected_observer = reference_direction[0] * observer[0]
            + reference_direction[1] * observer[1]
            + reference_direction[2] * observer[2];
        let light_time_julian_years_per_astronomical_unit = Length::METRES_PER_AU
            / Length::METRES_PER_LIGHT_SECOND
            / CatalogProperMotion::SECONDS_PER_JULIAN_YEAR;
        projected_observer * light_time_julian_years_per_astronomical_unit
    }
}

/// A catalog place propagated to one observation epoch in ICRS.
///
/// This stage contains space motion evaluated at the solar-system barycentre.
/// Observer displacement, solar light deflection, aberration, Earth
/// orientation, and refraction have not yet been applied. `C` preserves
/// whether the source is infinite or has physical six-parameter space motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AstrometricCatalogPlace<S: TimeScale, C = InfiniteCatalogPlace> {
    catalog: C,
    epoch: Instant<S>,
    tcb_epoch: JulianDate<Tcb>,
    elapsed_julian_years: f64,
    sofa_data: SofaCatalogData,
    direction: EquatorialDirection<Icrs>,
}

impl<S: TimeScale> AstrometricCatalogPlace<S, InfiniteCatalogPlace> {
    /// Exact infinite-distance propagation model identifier.
    pub const MODEL: &'static str = "IAU SOFA pmpx, zero parallax and radial velocity";

    /// Propagates an infinite catalog place to one physical observation epoch.
    pub fn from_catalog(catalog: InfiniteCatalogPlace, epoch: Instant<S>) -> Result<Self, Error> {
        Self::from_sofa_data(
            catalog,
            catalog.reference_epoch(),
            SofaCatalogData::from_infinite(catalog)?,
            epoch,
        )
    }
}

impl<S: TimeScale> AstrometricCatalogPlace<S, SpatialCatalogPlace> {
    /// Exact finite-distance propagation model identifier.
    pub const MODEL: &'static str =
        "IAU SOFA pmpx, full parallax, proper motion, and radial velocity";

    /// Propagates a spatial catalog place to one physical observation epoch.
    pub fn from_spatial_catalog(
        catalog: SpatialCatalogPlace,
        epoch: Instant<S>,
    ) -> Result<Self, Error> {
        Self::from_sofa_data(
            catalog,
            catalog.reference_epoch(),
            SofaCatalogData::from_spatial(catalog)?,
            epoch,
        )
    }
}

impl<S: TimeScale, C: Copy> AstrometricCatalogPlace<S, C> {
    fn from_sofa_data(
        catalog: C,
        reference_epoch: JulianDate<Tcb>,
        sofa_data: SofaCatalogData,
        epoch: Instant<S>,
    ) -> Result<Self, Error> {
        let tcb_epoch = JulianDate::<Tcb>::from_instant(epoch, &Hifitime::new())?;
        let (epoch_first, epoch_second) = tcb_epoch.parts();
        let (reference_first, reference_second) = reference_epoch.parts();
        let elapsed_julian_years =
            ((epoch_first - reference_first) + (epoch_second - reference_second)) / 365.25;
        let components = sofa_data.coordinate_components(elapsed_julian_years, [0.0; 3]);
        let direction =
            EquatorialDirection::from_direction(Direction::try_from_components(components)?)?;

        Ok(Self {
            catalog,
            epoch,
            tcb_epoch,
            elapsed_julian_years,
            sofa_data,
            direction,
        })
    }

    /// Returns the source catalog place at its reference epoch.
    pub const fn catalog_place(self) -> C {
        self.catalog
    }

    /// Returns the physical observation epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the observation epoch as a two-part TCB Julian Date.
    pub const fn tcb_epoch(self) -> JulianDate<Tcb> {
        self.tcb_epoch
    }

    /// Returns elapsed TCB Julian years from the catalog reference epoch.
    pub const fn elapsed_julian_years(self) -> f64 {
        self.elapsed_julian_years
    }

    /// Returns the space-motion-propagated ICRS direction at the SSB.
    pub const fn direction(self) -> EquatorialDirectionAt<Icrs, S> {
        EquatorialDirectionAt::new(self.epoch, self.direction)
    }
}

/// A finite-distance spatial catalog place propagated to an observation epoch.
pub type AstrometricSpatialCatalogPlace<S> = AstrometricCatalogPlace<S, SpatialCatalogPlace>;

/// Observer-dependent angular corrections applied to a catalog place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CatalogPlaceCorrections {
    proper_motion_roemer: Angle,
    parallax: Angle,
    solar_light_deflection: Angle,
    observer_aberration: Angle,
}

impl CatalogPlaceCorrections {
    fn angle_between_components(left: [f64; 3], right: [f64; 3]) -> Result<Angle, Error> {
        Ok(Direction::<Icrs>::try_from_components(left)?
            .angle_to(Direction::<Icrs>::try_from_components(right)?)?)
    }

    /// Returns the observer-dependent Roemer modulation of proper motion.
    pub const fn proper_motion_roemer(self) -> Angle {
        self.proper_motion_roemer
    }

    /// Returns annual plus diurnal parallax from the observer's SSB displacement.
    pub const fn parallax(self) -> Angle {
        self.parallax
    }

    /// Returns the solar gravitational light-deflection angle.
    pub const fn solar_light_deflection(self) -> Angle {
        self.solar_light_deflection
    }

    /// Returns the aberration angle from the observer's barycentric velocity.
    pub const fn observer_aberration(self) -> Angle {
        self.observer_aberration
    }
}

/// Vacuum observed place of a catalog source at a fixed site.
///
/// The result has passed through space motion, observer-dependent Roemer and
/// parallax terms, solar light deflection, relativistic aberration, IAU
/// 2006/2000A bias-precession-nutation, polar motion, Earth rotation, and local
/// horizontal projection. It deliberately has no emission epoch or iterative
/// solar-system light time.
#[derive(Debug, Clone, Copy)]
pub struct VacuumObservedCatalogPlace<S: TimeScale, C = InfiniteCatalogPlace> {
    astrometric: AstrometricCatalogPlace<S, C>,
    intermediate: EquatorialDirectionAt<Cirs, S>,
    topocentric_frame: TopocentricFrame<S>,
    horizontal: HorizontalDirection,
    observation_parameters: TerrestrialObservationParameters,
    corrections: CatalogPlaceCorrections,
}

impl<S: TimeScale, C: Copy> VacuumObservedCatalogPlace<S, C> {
    /// Exact ICRS-to-vacuum-observed model identifier.
    pub const MODEL: &'static str = "IAU SOFA pmpx -> ldsun -> ab -> IAU 2006/2000A BPN -> atioq";

    /// Returns the propagated astrometric input stage.
    pub const fn astrometric(self) -> AstrometricCatalogPlace<S, C> {
        self.astrometric
    }

    /// Returns the shared physical observation epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.astrometric.epoch()
    }

    /// Returns the epoch-bound runtime topocentric frame.
    pub const fn topocentric_frame(self) -> TopocentricFrame<S> {
        self.topocentric_frame
    }

    /// Returns topocentric CIRS intermediate right ascension and declination.
    pub const fn intermediate_equatorial(self) -> EquatorialDirectionAt<Cirs, S> {
        self.intermediate
    }

    /// Returns local vacuum azimuth and altitude.
    pub const fn horizontal(self) -> HorizontalDirection {
        self.horizontal
    }

    /// Returns structured observer-dependent correction magnitudes.
    pub const fn corrections(self) -> CatalogPlaceCorrections {
        self.corrections
    }

    /// Applies SOFA atmospheric refraction and advances to observed place.
    pub fn apply_refraction(
        self,
        conditions: AtmosphericConditions,
    ) -> Result<ObservedCatalogPlace<S, C>, Error> {
        let coordinates = self.intermediate.coordinates();
        let parameters = self.observation_parameters.with_refraction(conditions);
        let (azimuth, zenith_distance, _, _, _) = sofars::astro::atioq(
            coordinates.right_ascension().as_radians(),
            coordinates.declination().as_radians(),
            &parameters,
        );
        let altitude = FRAC_PI_2 - zenith_distance;
        let (altitude_sine, altitude_cosine) = altitude.sin_cos();
        let (azimuth_sine, azimuth_cosine) = azimuth.sin_cos();
        let horizontal = HorizontalDirection::from_enu_components([
            altitude_cosine * azimuth_sine,
            altitude_cosine * azimuth_cosine,
            altitude_sine,
        ])?;
        let vacuum_zenith_distance = self.horizontal.zenith_distance()?.as_radians();
        let observed_zenith_distance = horizontal.zenith_distance()?.as_radians();
        let correction = RefractionCorrection {
            amount: Angle::from_radians(vacuum_zenith_distance - observed_zenith_distance)?,
            accuracy: RefractionAccuracy::from_zenith_distance(zenith_distance),
        };

        Ok(ObservedCatalogPlace {
            vacuum: self,
            horizontal,
            conditions,
            correction,
        })
    }
}

/// Atmospherically refracted observed place of a catalog source.
#[derive(Debug, Clone, Copy)]
pub struct ObservedCatalogPlace<S: TimeScale, C = InfiniteCatalogPlace> {
    vacuum: VacuumObservedCatalogPlace<S, C>,
    horizontal: HorizontalDirection,
    conditions: AtmosphericConditions,
    correction: RefractionCorrection,
}

impl<S: TimeScale, C: Copy> ObservedCatalogPlace<S, C> {
    /// Returns the preceding vacuum stage.
    pub const fn vacuum(self) -> VacuumObservedCatalogPlace<S, C> {
        self.vacuum
    }

    /// Returns the shared physical observation epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.vacuum.epoch()
    }

    /// Returns local refracted azimuth and altitude.
    pub const fn horizontal(self) -> HorizontalDirection {
        self.horizontal
    }

    /// Returns the atmospheric inputs used by SOFA.
    pub const fn atmospheric_conditions(self) -> AtmosphericConditions {
        self.conditions
    }

    /// Returns the applied atmospheric angular correction.
    pub const fn refraction(self) -> RefractionCorrection {
        self.correction
    }
}

/// Vacuum observed place of a finite-distance spatial catalog source.
pub type VacuumObservedSpatialCatalogPlace<S> = VacuumObservedCatalogPlace<S, SpatialCatalogPlace>;

/// Refracted observed place of a finite-distance spatial catalog source.
pub type ObservedSpatialCatalogPlace<S> = ObservedCatalogPlace<S, SpatialCatalogPlace>;

/// Astrometric state for one fixed terrestrial observer at one reception epoch.
///
/// The value freezes the site's GCRS state, its observed GCRS-to-CIRS attitude,
/// barycentric state, and SOFA observation parameters so multiple targets at
/// the same site and epoch can reuse this preparation.
#[derive(Debug, Clone, Copy)]
pub struct FixedObserverAt<'ephemeris, S: TimeScale> {
    ephemeris: &'ephemeris Ephemeris,
    topocentric_frame: TopocentricFrame<S>,
    gcrs_to_cirs: FrameRotation<Gcrs, Cirs, S>,
    barycentric: BarycentricObserverState<S>,
    parameters: sofars::astro::IauAstrom,
}

impl<S: TimeScale> FixedObserverAt<'_, S> {
    /// Returns the fixed reception epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.barycentric.epoch
    }

    /// Returns the epoch-bound runtime topocentric frame.
    pub const fn topocentric_frame(self) -> TopocentricFrame<S> {
        self.topocentric_frame
    }

    /// Returns the observer's BCRS position relative to the solar-system barycentre.
    pub const fn barycentric_position(self) -> Vector3<Bcrs, Length> {
        self.barycentric.position
    }

    /// Returns the observer's BCRS velocity, including orbital and site motion.
    pub const fn barycentric_velocity(self) -> Vector3<Bcrs, Speed> {
        self.barycentric.velocity
    }

    /// Returns the model used to derive the fixed site's inertial velocity.
    pub const fn velocity_model(self) -> SiteVelocityModel {
        self.topocentric_frame.velocity_model()
    }

    /// Computes an infinite catalog source's vacuum observed place.
    ///
    /// Zero parallax and radial velocity are structural properties of the
    /// source type. The observer-dependent Roemer term remains active.
    pub fn vacuum_observed_catalog_place(
        self,
        astrometric: AstrometricCatalogPlace<S>,
    ) -> Result<VacuumObservedCatalogPlace<S>, Error> {
        self.vacuum_observed_catalog(astrometric)
    }

    /// Computes a finite-distance spatial catalog source's vacuum observed place.
    ///
    /// The chain applies full proper motion, radial motion, annual and diurnal
    /// parallax, the observer-dependent Roemer term, solar light deflection,
    /// relativistic aberration, Earth orientation, and local projection.
    pub fn vacuum_observed_spatial_catalog_place(
        self,
        astrometric: AstrometricSpatialCatalogPlace<S>,
    ) -> Result<VacuumObservedSpatialCatalogPlace<S>, Error> {
        self.vacuum_observed_catalog(astrometric)
    }

    fn vacuum_observed_catalog<C: Copy>(
        self,
        astrometric: AstrometricCatalogPlace<S, C>,
    ) -> Result<VacuumObservedCatalogPlace<S, C>, Error> {
        let catalog_epoch = astrometric.epoch().tai_nanoseconds_since_1900();
        let observer_epoch = self.epoch().tai_nanoseconds_since_1900();
        if catalog_epoch != observer_epoch {
            return Err(Error::CatalogPlaceEpochMismatch {
                catalog_tai_nanoseconds: catalog_epoch,
                observer_tai_nanoseconds: observer_epoch,
            });
        }

        let sofa_data = astrometric.sofa_data;
        let elapsed_julian_years = astrometric.elapsed_julian_years();
        let roemer_components = sofa_data.coordinate_components(
            elapsed_julian_years + sofa_data.roemer_time_offset_julian_years(self.parameters.eb),
            [0.0; 3],
        );
        let coordinate_components =
            sofa_data.coordinate_components(elapsed_julian_years, self.parameters.eb);
        let natural_components = sofars::astro::ldsun(
            coordinate_components,
            self.parameters.eh,
            self.parameters.em,
        );
        let proper_components = sofars::astro::ab(
            &natural_components,
            &self.parameters.v,
            self.parameters.em,
            self.parameters.bm1,
        );
        let mut intermediate_components = [0.0; 3];
        sofars::vm::rxp(
            &self.parameters.bpn,
            &proper_components,
            &mut intermediate_components,
        );
        let intermediate_coordinates = EquatorialDirection::from_direction(
            Direction::<Cirs>::try_from_components(intermediate_components)?,
        )?;
        let intermediate = EquatorialDirectionAt::new(self.epoch(), intermediate_coordinates);
        let (azimuth, zenith_distance, _, _, _) = sofars::astro::atioq(
            intermediate_coordinates.right_ascension().as_radians(),
            intermediate_coordinates.declination().as_radians(),
            &self.parameters,
        );
        let altitude = FRAC_PI_2 - zenith_distance;
        let (altitude_sine, altitude_cosine) = altitude.sin_cos();
        let (azimuth_sine, azimuth_cosine) = azimuth.sin_cos();
        let horizontal = HorizontalDirection::from_enu_components([
            altitude_cosine * azimuth_sine,
            altitude_cosine * azimuth_cosine,
            altitude_sine,
        ])?;
        let astrometric_components = astrometric
            .direction()
            .coordinates()
            .to_direction()?
            .components();
        let corrections = CatalogPlaceCorrections {
            proper_motion_roemer: CatalogPlaceCorrections::angle_between_components(
                astrometric_components,
                roemer_components,
            )?,
            parallax: CatalogPlaceCorrections::angle_between_components(
                roemer_components,
                coordinate_components,
            )?,
            solar_light_deflection: CatalogPlaceCorrections::angle_between_components(
                coordinate_components,
                natural_components,
            )?,
            observer_aberration: CatalogPlaceCorrections::angle_between_components(
                natural_components,
                proper_components,
            )?,
        };

        Ok(VacuumObservedCatalogPlace {
            astrometric,
            intermediate,
            topocentric_frame: self.topocentric_frame,
            horizontal,
            observation_parameters: TerrestrialObservationParameters::from_sofa(self.parameters),
            corrections,
        })
    }

    /// Computes a finite solar-system target's vacuum observed place.
    ///
    /// The result includes station-aware reception light time, topocentric
    /// parallax, finite-distance solar light deflection, relativistic
    /// aberration from the combined barycentric observer velocity, IAU
    /// 2006/2000A Earth orientation, polar motion, and local horizontal
    /// projection. It excludes atmospheric refraction and Shapiro delay.
    pub fn vacuum_observed_place(
        self,
        target: CelestialBody,
        options: ReceptionLightTimeOptions,
    ) -> Result<VacuumObservedPlace<S>, Error> {
        let reception = self.solve_reception_light_time(target, options)?;
        let deflection = apply_finite_solar_light_deflection(
            self.ephemeris,
            SolarDeflectionInput {
                target,
                reception_epoch: self.barycentric.epoch,
                emission_epoch: reception.emission_epoch,
                observer_barycentric_position: self.barycentric.position,
                target_reception_position: reception.target_reception_position,
                target_emission_position: reception.target_barycentric_position,
                source_direction: reception.direction,
                target_distance: reception.distance,
            },
        )?;
        let natural_direction = deflection.direction;
        let solar_light_deflection = deflection.diagnostics;
        let proper_components = sofars::astro::ab(
            &natural_direction.components(),
            &self.parameters.v,
            self.parameters.em,
            self.parameters.bm1,
        );
        let proper_direction = Direction::<Gcrs>::try_from_components(proper_components)?;
        let intermediate = EquatorialDirectionAt::new(
            self.epoch(),
            EquatorialDirection::from_direction(
                self.gcrs_to_cirs.apply_direction(proper_direction)?,
            )?,
        );
        let horizontal = self
            .topocentric_frame
            .horizontal_direction(proper_direction)?;

        Ok(VacuumObservedPlace {
            target,
            topocentric_frame: self.topocentric_frame,
            emission_epoch: reception.emission_epoch,
            light_time: reception.light_time,
            intermediate,
            observation_parameters: TerrestrialObservationParameters::from_sofa(self.parameters),
            horizontal,
            distance: reception.distance,
            solar_light_deflection,
            iterations: reception.iterations,
            light_time_residual: reception.residual,
        })
    }

    fn solve_reception_light_time(
        self,
        target: CelestialBody,
        options: ReceptionLightTimeOptions,
    ) -> Result<FixedReception<S>, Error> {
        let barycentre = CelestialBody::SolarSystemBarycenter;
        let target_reception = self.ephemeris.state(EphemerisQuery::new(
            target,
            barycentre,
            self.barycentric.epoch,
        ))?;
        let initial_position = target_reception
            .position()
            .checked_sub(self.barycentric.position)?;
        let (initial_distance, _) =
            Self::line_of_sight(target, self.barycentric.epoch, initial_position)?;
        let mut light_time = Self::duration_from_distance(initial_distance)?;
        let mut last_residual = Duration::ZERO;
        let mut last_emission = self.barycentric.epoch.checked_sub(light_time)?;

        for iteration in 1..=options.max_iterations {
            let emission_epoch = self.barycentric.epoch.checked_sub(light_time)?;
            let target_emission =
                self.ephemeris
                    .state(EphemerisQuery::new(target, barycentre, emission_epoch))?;
            let relative_position = target_emission
                .position()
                .checked_sub(self.barycentric.position)?;
            let (distance, direction) =
                Self::line_of_sight(target, emission_epoch, relative_position)?;
            let candidate = Self::duration_from_distance(distance)?;
            let residual = candidate.checked_sub(light_time)?.checked_abs()?;
            if residual <= options.time_tolerance {
                return Ok(FixedReception {
                    emission_epoch,
                    light_time,
                    direction,
                    distance,
                    target_reception_position: target_reception.position(),
                    target_barycentric_position: target_emission.position(),
                    iterations: iteration,
                    residual,
                });
            }
            light_time = candidate;
            last_residual = residual;
            last_emission = emission_epoch;
        }

        Err(Error::FixedSiteLightTimeDidNotConverge {
            target,
            iterations: options.max_iterations,
            residual_nanoseconds: last_residual.as_nanoseconds(),
            emission_tai_nanoseconds: last_emission.tai_nanoseconds_since_1900(),
        })
    }

    fn line_of_sight(
        target: CelestialBody,
        epoch: Instant<S>,
        position: Vector3<Bcrs, Length>,
    ) -> Result<(Length, Direction<Bcrs>), Error> {
        let distance = position.magnitude()?;
        if distance.as_metres() == 0.0 {
            return Err(Error::UndefinedFixedSiteLineOfSight {
                target,
                epoch_tai_nanoseconds: epoch.tai_nanoseconds_since_1900(),
            });
        }
        Ok((distance, position.direction()?))
    }

    fn duration_from_distance(distance: Length) -> Result<Duration, Error> {
        Duration::from_seconds_f64(distance.as_metres() / Length::METRES_PER_LIGHT_SECOND)
            .map_err(Error::from)
    }
}

/// A finite target's topocentric vacuum observed place.
///
/// The target is evaluated at the retained emission epoch and the fixed site
/// at the retained reception epoch. The horizontal and CIRS coordinates share
/// one correction chain. No atmospheric or other propagation-medium
/// correction has been applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VacuumObservedPlace<S: TimeScale> {
    target: CelestialBody,
    topocentric_frame: TopocentricFrame<S>,
    emission_epoch: Instant<S>,
    light_time: Duration,
    intermediate: EquatorialDirectionAt<Cirs, S>,
    observation_parameters: TerrestrialObservationParameters,
    horizontal: HorizontalDirection,
    solar_light_deflection: SolarLightDeflection<S>,
    distance: Length,
    iterations: u32,
    light_time_residual: Duration,
}

impl<S: TimeScale> VacuumObservedPlace<S> {
    /// Returns the observed target.
    pub const fn target(self) -> CelestialBody {
        self.target
    }

    /// Returns the fixed-site reception epoch.
    pub const fn reception_epoch(self) -> Instant<S> {
        self.topocentric_frame.epoch()
    }

    /// Returns the retarded target emission epoch.
    pub const fn emission_epoch(self) -> Instant<S> {
        self.emission_epoch
    }

    /// Returns the epoch-bound runtime topocentric frame.
    pub const fn topocentric_frame(self) -> TopocentricFrame<S> {
        self.topocentric_frame
    }

    /// Returns topocentric CIRS intermediate right ascension and declination.
    pub const fn intermediate_equatorial(self) -> EquatorialDirectionAt<Cirs, S> {
        self.intermediate
    }

    /// Returns local vacuum azimuth and altitude.
    pub const fn horizontal(self) -> HorizontalDirection {
        self.horizontal
    }

    /// Returns target-at-emission to site-at-reception distance.
    pub const fn distance(self) -> Length {
        self.distance
    }

    /// Returns finite-distance solar light-deflection diagnostics.
    pub const fn solar_light_deflection(self) -> SolarLightDeflection<S> {
        self.solar_light_deflection
    }

    /// Returns the converged one-way reception light time.
    pub const fn light_time(self) -> Duration {
        self.light_time
    }

    /// Returns the number of completed light-time iterations.
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns the final absolute light-time fixed-point residual.
    pub const fn light_time_residual(self) -> Duration {
        self.light_time_residual
    }
}

impl<S: TimeScale> VacuumObservedPlace<S> {
    /// Applies SOFA atmospheric refraction and advances to an observed-place stage.
    ///
    /// The retained CIRS direction is transformed with the same Earth-rotation,
    /// polar-motion, and site geometry used by the vacuum result. The observer's
    /// rotational velocity was already included in the barycentric aberration,
    /// so the SOFA terrestrial step deliberately applies no second diurnal
    /// aberration.
    pub fn apply_refraction(
        self,
        conditions: AtmosphericConditions,
    ) -> Result<ObservedPlace<S>, Error> {
        let coordinates = self.intermediate.coordinates();
        let parameters = self.observation_parameters.with_refraction(conditions);
        let (azimuth, zenith_distance, _, _, _) = sofars::astro::atioq(
            coordinates.right_ascension().as_radians(),
            coordinates.declination().as_radians(),
            &parameters,
        );
        let altitude = FRAC_PI_2 - zenith_distance;
        let (altitude_sine, altitude_cosine) = altitude.sin_cos();
        let (azimuth_sine, azimuth_cosine) = azimuth.sin_cos();
        let horizontal = HorizontalDirection::from_enu_components([
            altitude_cosine * azimuth_sine,
            altitude_cosine * azimuth_cosine,
            altitude_sine,
        ])?;
        let vacuum_zenith_distance = self.horizontal.zenith_distance()?.as_radians();
        let observed_zenith_distance = horizontal.zenith_distance()?.as_radians();
        let correction = RefractionCorrection {
            amount: Angle::from_radians(vacuum_zenith_distance - observed_zenith_distance)?,
            accuracy: RefractionAccuracy::from_zenith_distance(zenith_distance),
        };

        Ok(ObservedPlace {
            vacuum: self,
            horizontal,
            conditions,
            correction,
        })
    }
}

/// SOFA refraction accuracy class determined from observed zenith distance.
///
/// These classes report the applicability of the simple atmospheric model;
/// they are not uncertainties inferred from the caller's meteorological data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefractionAccuracy {
    /// Observed zenith distance is below 70 degrees, SOFA's nominal range.
    Nominal,
    /// Observed zenith distance is from 70 up to 85 degrees.
    HighZenithDistance,
    /// Observed zenith distance is from 85 through 90 degrees.
    NearHorizon,
    /// The target is below the astronomical horizon, outside validated accuracy.
    BelowHorizon,
}

impl RefractionAccuracy {
    fn from_zenith_distance(radians: f64) -> Self {
        let degrees = radians.to_degrees();
        if degrees < 70.0 {
            Self::Nominal
        } else if degrees < 85.0 {
            Self::HighZenithDistance
        } else if degrees <= 90.0 {
            Self::NearHorizon
        } else {
            Self::BelowHorizon
        }
    }
}

/// Atmospheric angular correction and declared SOFA model applicability.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefractionCorrection {
    amount: Angle,
    accuracy: RefractionAccuracy,
}

impl RefractionCorrection {
    /// Exact production model identifier.
    pub const MODEL: &'static str = "IAU SOFA refco/atioq via sofars 0.6.1";

    /// Returns vacuum minus observed zenith distance.
    pub const fn amount(self) -> Angle {
        self.amount
    }

    /// Returns the applicability class at the observed zenith distance.
    pub const fn accuracy(self) -> RefractionAccuracy {
        self.accuracy
    }
}

/// A finite target's topocentric place after atmospheric refraction.
///
/// This stage retains its source [`VacuumObservedPlace`] so refraction cannot
/// be applied twice through the type-safe correction chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObservedPlace<S: TimeScale> {
    vacuum: VacuumObservedPlace<S>,
    horizontal: HorizontalDirection,
    conditions: AtmosphericConditions,
    correction: RefractionCorrection,
}

impl<S: TimeScale> ObservedPlace<S> {
    /// Returns the source vacuum place and all non-atmospheric diagnostics.
    pub const fn vacuum(self) -> VacuumObservedPlace<S> {
        self.vacuum
    }

    /// Returns the observed target.
    pub const fn target(self) -> CelestialBody {
        self.vacuum.target()
    }

    /// Returns the fixed-site reception epoch.
    pub const fn reception_epoch(self) -> Instant<S> {
        self.vacuum.reception_epoch()
    }

    /// Returns the retarded target emission epoch.
    pub const fn emission_epoch(self) -> Instant<S> {
        self.vacuum.emission_epoch()
    }

    /// Returns local azimuth and refraction-affected altitude.
    pub const fn horizontal(self) -> HorizontalDirection {
        self.horizontal
    }

    /// Returns the atmospheric observations supplied for this result.
    pub const fn atmospheric_conditions(self) -> AtmosphericConditions {
        self.conditions
    }

    /// Returns the applied refraction correction and applicability class.
    pub const fn refraction(self) -> RefractionCorrection {
        self.correction
    }
}

/// Astrometric correction algorithms backed by one time context and ephemeris.
pub struct Astrometry<'context, 'data, E> {
    time: &'context TimeContext<'data, E>,
    ephemeris: &'context Ephemeris,
}

impl<'context, 'data, E> Copy for Astrometry<'context, 'data, E> {}

impl<'context, 'data, E> Clone for Astrometry<'context, 'data, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'context, 'data, E> Astrometry<'context, 'data, E> {
    /// Constructs astrometric algorithms from explicit immutable dependencies.
    pub const fn new(
        time: &'context TimeContext<'data, E>,
        ephemeris: &'context Ephemeris,
    ) -> Self {
        Self { time, ephemeris }
    }

    /// Returns the time context used for frame and scale evaluation.
    pub const fn time_context(self) -> &'context TimeContext<'data, E> {
        self.time
    }

    /// Returns the ephemeris used for geometric state evaluation.
    pub const fn ephemeris(self) -> &'context Ephemeris {
        self.ephemeris
    }

    /// Solves one-way reception light time with strict dual-epoch semantics.
    pub fn reception_light_time<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
        options: ReceptionLightTimeOptions,
    ) -> Result<ReceptionLightTime<S>, Error> {
        self.solve_reception_light_time(query, options)
            .map(|solution| solution.result)
    }

    /// Computes a finite target's geocentric apparent place at one reception epoch.
    ///
    /// The Earth is fixed at the reception epoch while the target is iterated
    /// to its retarded emission epoch. The result then applies finite-distance
    /// solar point-mass deflection, relativistic annual aberration, and IAU
    /// 2006/2000A orientation. It excludes station parallax, atmospheric
    /// refraction, and Shapiro delay.
    pub fn geocentric_apparent_place<S: TimeScale>(
        &self,
        target: CelestialBody,
        reception_epoch: Instant<S>,
        light_time_options: ReceptionLightTimeOptions,
    ) -> Result<GeocentricApparentPlace<S>, Error> {
        let query = EphemerisQuery::new(target, CelestialBody::Earth, reception_epoch);
        let light_time = self.solve_reception_light_time(query, light_time_options)?;
        let deflection = apply_finite_solar_light_deflection(
            self.ephemeris,
            SolarDeflectionInput {
                target,
                reception_epoch,
                emission_epoch: light_time.result.emission_epoch,
                observer_barycentric_position: light_time.observer_barycentric.position(),
                target_reception_position: light_time.target_reception_position,
                target_emission_position: light_time.target_emission_position,
                source_direction: light_time.result.direction,
                target_distance: light_time.result.distance,
            },
        )?;
        let natural_direction = deflection.direction;
        let solar_light_deflection = deflection.diagnostics;
        let sun_reception_position = deflection.sun_reception_position;
        let proper_direction = self.geocentric_aberration(
            reception_epoch,
            natural_direction,
            light_time.observer_barycentric,
            sun_reception_position,
        )?;
        let celestial = Frames::new(self.time).celestial_orientation_at(reception_epoch)?;
        let gcrs_direction = EquatorialDirectionAt::new(reception_epoch, proper_direction);
        let true_equatorial = celestial.true_equatorial(proper_direction)?;
        let true_ecliptic = celestial.true_ecliptic_from_gcrs(proper_direction)?;

        Ok(GeocentricApparentPlace::new(
            light_time.result,
            gcrs_direction,
            true_equatorial,
            true_ecliptic,
            solar_light_deflection,
        ))
    }

    /// Computes the geocentric apparent place of the Sun at one reception epoch.
    pub fn solar_apparent_place<S: TimeScale>(
        &self,
        reception_epoch: Instant<S>,
        light_time_options: ReceptionLightTimeOptions,
    ) -> Result<SolarApparentPlace<S>, Error> {
        self.geocentric_apparent_place(CelestialBody::Sun, reception_epoch, light_time_options)
            .map(SolarApparentPlace::new)
    }

    /// Computes coherent Greenwich and local solar-time quantities.
    ///
    /// Mean solar time is driven by UT1. Apparent solar time is derived from
    /// the geocentric apparent Sun on true equator and equinox of date axes and
    /// IAU 2006/2000A Greenwich apparent sidereal time.
    pub fn solar_time<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        light_time_options: ReceptionLightTimeOptions,
    ) -> Result<SolarTimeSolution<S>, Error>
    where
        TimeContext<'data, E>: TimeScaleModel<Ut1>,
    {
        let sidereal_time = Frames::new(self.time).sidereal_time_at(epoch)?;
        let apparent_sun = self.solar_apparent_place(epoch, light_time_options)?;
        SolarTimeSolution::new(apparent_sun, sidereal_time)
    }

    fn solve_reception_light_time<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
        options: ReceptionLightTimeOptions,
    ) -> Result<ReceptionComputation<S>, Error> {
        let target = query.target();
        let observer = query.center();
        let reception_epoch = query.epoch();
        if target == observer {
            return Err(Error::UndefinedIdentityObservation { body: target });
        }

        let barycentre = CelestialBody::SolarSystemBarycenter;
        let observer_barycentric =
            self.ephemeris
                .state(EphemerisQuery::new(observer, barycentre, reception_epoch))?;
        let target_reception =
            self.ephemeris
                .state(EphemerisQuery::new(target, barycentre, reception_epoch))?;
        let initial_position = target_reception
            .position()
            .checked_sub(observer_barycentric.position())?;
        let (initial_distance, _) =
            Self::line_of_sight(target, observer, reception_epoch, initial_position)?;
        let mut light_time = Self::duration_from_distance(initial_distance)?;
        let mut last_residual = Duration::ZERO;
        let mut last_emission = reception_epoch.checked_sub(light_time)?;

        for iteration in 1..=options.max_iterations {
            let emission_epoch = reception_epoch.checked_sub(light_time)?;
            let target_emission =
                self.ephemeris
                    .state(EphemerisQuery::new(target, barycentre, emission_epoch))?;
            let relative_position = target_emission
                .position()
                .checked_sub(observer_barycentric.position())?;
            let (distance, direction) =
                Self::line_of_sight(target, observer, emission_epoch, relative_position)?;
            let candidate = Self::duration_from_distance(distance)?;
            let residual = candidate.checked_sub(light_time)?.checked_abs()?;
            if residual <= options.time_tolerance {
                return Ok(ReceptionComputation {
                    result: ReceptionLightTime {
                        target,
                        observer,
                        reception_epoch,
                        emission_epoch,
                        light_time,
                        relative_position,
                        direction,
                        distance,
                        iterations: iteration,
                        residual,
                    },
                    observer_barycentric,
                    target_reception_position: target_reception.position(),
                    target_emission_position: target_emission.position(),
                });
            }
            light_time = candidate;
            last_residual = residual;
            last_emission = emission_epoch;
        }

        Err(Error::LightTimeDidNotConverge {
            target,
            observer,
            iterations: options.max_iterations,
            residual_nanoseconds: last_residual.as_nanoseconds(),
            emission_tai_nanoseconds: last_emission.tai_nanoseconds_since_1900(),
        })
    }

    fn geocentric_aberration<S: TimeScale>(
        &self,
        reception_epoch: Instant<S>,
        natural_direction: Direction<Bcrs>,
        earth_barycentric: RelativeState<Bcrs, S>,
        sun_reception_position: Vector3<Bcrs, Length>,
    ) -> Result<EquatorialDirection<Gcrs>, Error> {
        let earth_speed = earth_barycentric.velocity().magnitude()?;
        if earth_speed.as_metres_per_second() >= Length::METRES_PER_LIGHT_SECOND {
            return Err(Error::ObserverAtOrAboveLightSpeed {
                observer: CelestialBody::Earth,
                speed_metres_per_second: earth_speed.as_metres_per_second(),
            });
        }

        let earth_heliocentric = earth_barycentric
            .position()
            .checked_sub(sun_reception_position)?;
        Self::line_of_sight(
            CelestialBody::Earth,
            CelestialBody::Sun,
            reception_epoch,
            earth_heliocentric,
        )?;

        let tdb = JulianDate::<Tdb>::from_instant(reception_epoch, &Hifitime::new())?;
        let (tdb_first, tdb_second) = tdb.parts();
        let earth_barycentric_pv = Self::barycentric_pv(earth_barycentric);
        let earth_heliocentric_position = Self::position_as_astronomical_units(earth_heliocentric);
        let mut parameters = sofars::astro::IauAstrom::default();
        sofars::astro::apcg(
            tdb_first,
            tdb_second,
            &earth_barycentric_pv,
            &earth_heliocentric_position,
            &mut parameters,
        );
        let proper = sofars::astro::ab(
            &natural_direction.components(),
            &parameters.v,
            parameters.em,
            parameters.bm1,
        );
        EquatorialDirection::from_direction(Direction::<Gcrs>::try_from_components(proper)?)
            .map_err(Error::from)
    }

    fn line_of_sight<S: TimeScale>(
        target: CelestialBody,
        observer: CelestialBody,
        epoch: Instant<S>,
        position: Vector3<Bcrs, Length>,
    ) -> Result<(Length, Direction<Bcrs>), Error> {
        let distance = position.magnitude()?;
        if distance.as_metres() == 0.0 {
            return Err(Error::UndefinedLineOfSight {
                target,
                observer,
                epoch_tai_nanoseconds: epoch.tai_nanoseconds_since_1900(),
            });
        }
        Ok((distance, position.direction()?))
    }

    fn duration_from_distance(distance: Length) -> Result<Duration, Error> {
        Duration::from_seconds_f64(distance.as_metres() / Length::METRES_PER_LIGHT_SECOND)
            .map_err(Error::from)
    }

    fn barycentric_pv<S: TimeScale>(state: RelativeState<Bcrs, S>) -> [[f64; 3]; 2] {
        let [x, y, z] = state.position().components();
        let [velocity_x, velocity_y, velocity_z] = state.velocity().components();
        [
            [
                x.as_astronomical_units(),
                y.as_astronomical_units(),
                z.as_astronomical_units(),
            ],
            [
                velocity_x.as_astronomical_units_per_day(),
                velocity_y.as_astronomical_units_per_day(),
                velocity_z.as_astronomical_units_per_day(),
            ],
        ]
    }

    fn position_as_astronomical_units(position: Vector3<Bcrs, Length>) -> [f64; 3] {
        let [x, y, z] = position.components();
        [
            x.as_astronomical_units(),
            y.as_astronomical_units(),
            z.as_astronomical_units(),
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn fixed_observer_from_topocentric_frame<S: TimeScale>(
        &self,
        site: &FixedSite,
        topocentric_frame: TopocentricFrame<S>,
        gcrs_to_cirs: FrameRotation<Gcrs, Cirs, S>,
        tio_locator_radians: f64,
        earth_rotation_angle_radians: f64,
        polar_motion_x_radians: f64,
        polar_motion_y_radians: f64,
    ) -> Result<FixedObserverAt<'context, S>, Error> {
        let epoch = topocentric_frame.epoch();
        let barycentre = CelestialBody::SolarSystemBarycenter;
        let earth_barycentric =
            self.ephemeris
                .state(EphemerisQuery::new(CelestialBody::Earth, barycentre, epoch))?;
        let sun_barycentric =
            self.ephemeris
                .state(EphemerisQuery::new(CelestialBody::Sun, barycentre, epoch))?;
        let geocentric_state = topocentric_frame.observer_state();
        let geocentric_position = Vector3::<Bcrs, Length>::from_array(
            geocentric_state.position().position().components(),
        );
        let geocentric_velocity =
            Vector3::<Bcrs, Speed>::from_array(geocentric_state.velocity().components());
        let barycentric = BarycentricObserverState {
            position: earth_barycentric
                .position()
                .checked_add(geocentric_position)?,
            velocity: earth_barycentric
                .velocity()
                .checked_add(geocentric_velocity)?,
            epoch,
        };
        let observer_speed = barycentric.velocity.magnitude()?;
        if observer_speed.as_metres_per_second() >= Length::METRES_PER_LIGHT_SECOND {
            return Err(Error::FixedObserverAtOrAboveLightSpeed {
                speed_metres_per_second: observer_speed.as_metres_per_second(),
            });
        }

        let earth_heliocentric = earth_barycentric
            .position()
            .checked_sub(sun_barycentric.position())?;
        Self::line_of_sight(
            CelestialBody::Earth,
            CelestialBody::Sun,
            epoch,
            earth_heliocentric,
        )?;
        let tdb = JulianDate::<Tdb>::from_instant(epoch, &Hifitime::new())?;
        let (tdb_first, tdb_second) = tdb.parts();
        let [site_x, site_y, site_z] = geocentric_state.position().position().components();
        let [site_velocity_x, site_velocity_y, site_velocity_z] =
            geocentric_state.velocity().components();
        let geocentric_pv = [
            [site_x.as_metres(), site_y.as_metres(), site_z.as_metres()],
            [
                site_velocity_x.as_metres_per_second(),
                site_velocity_y.as_metres_per_second(),
                site_velocity_z.as_metres_per_second(),
            ],
        ];
        let earth_barycentric_pv = Self::barycentric_pv(earth_barycentric);
        let earth_heliocentric_position = Self::position_as_astronomical_units(earth_heliocentric);
        let mut parameters = sofars::astro::IauAstrom::default();
        sofars::astro::apcs(
            tdb_first,
            tdb_second,
            &geocentric_pv,
            &earth_barycentric_pv,
            &earth_heliocentric_position,
            &mut parameters,
        );
        let geodetic_position = site.geodetic_position();
        sofars::astro::apio(
            tio_locator_radians,
            earth_rotation_angle_radians,
            geodetic_position.longitude().as_radians(),
            geodetic_position.latitude().as_radians(),
            geodetic_position.height().as_metres(),
            polar_motion_x_radians,
            polar_motion_y_radians,
            0.0,
            0.0,
            &mut parameters,
        );
        // APCS already incorporated the selected site velocity into the
        // barycentric aberration vector. Disable APIO's redundant nominal
        // diurnal-aberration step.
        parameters.diurab = 0.0;

        Ok(FixedObserverAt {
            ephemeris: self.ephemeris,
            topocentric_frame,
            gcrs_to_cirs,
            barycentric,
            parameters,
        })
    }
}

impl<'context, 'data, 'eop> Astrometry<'context, 'data, EarthOrientationTable<'eop>> {
    /// Prepares one reusable fixed-site observer at a reception epoch.
    ///
    /// The construction requires complete observed EOP because the resulting
    /// barycentric velocity includes the measured Earth-rotation rate and
    /// frame-rate corrections used by the site's GCRS state.
    pub fn fixed_observer_at<S: TimeScale>(
        &self,
        site: &FixedSite,
        epoch: Instant<S>,
    ) -> Result<FixedObserverAt<'context, S>, Error> {
        let earth_orientation = Frames::new(self.time).earth_orientation_at(epoch)?;
        let topocentric_frame = site.topocentric_frame_from_orientation(earth_orientation)?;
        let observations = earth_orientation.observations();
        self.fixed_observer_from_topocentric_frame(
            site,
            topocentric_frame,
            earth_orientation.gcrs_to_cirs(),
            earth_orientation.tio_locator().as_radians(),
            earth_orientation.earth_rotation_angle().as_radians(),
            observations.polar_motion_x().as_angle().as_radians(),
            observations.polar_motion_y().as_angle().as_radians(),
        )
    }
}

impl<'context, 'data, 'eop> Astrometry<'context, 'data, EarthAttitudeTable<'eop>> {
    /// Prepares a fixed-site observer using observed attitude and nominal Earth rotation.
    ///
    /// The returned observer retains `UT1−UTC`, polar motion, and celestial-pole
    /// corrections. Its site velocity explicitly uses the IERS conventional
    /// nominal angular speed because this context carries no length-of-day
    /// observation.
    pub fn fixed_observer_with_nominal_rotation_at<S: TimeScale>(
        &self,
        site: &FixedSite,
        epoch: Instant<S>,
    ) -> Result<FixedObserverAt<'context, S>, Error> {
        let attitude = Frames::new(self.time).earth_attitude_at(epoch)?;
        let topocentric_frame =
            site.topocentric_frame_from_attitude_with_nominal_rotation(attitude)?;
        let observations = attitude.observations();
        self.fixed_observer_from_topocentric_frame(
            site,
            topocentric_frame,
            attitude.gcrs_to_cirs(),
            attitude.tio_locator().as_radians(),
            attitude.earth_rotation_angle().as_radians(),
            observations.polar_motion_x().as_angle().as_radians(),
            observations.polar_motion_y().as_angle().as_radians(),
        )
    }
}

struct ReceptionComputation<S: TimeScale> {
    result: ReceptionLightTime<S>,
    observer_barycentric: RelativeState<Bcrs, S>,
    target_reception_position: Vector3<Bcrs, Length>,
    target_emission_position: Vector3<Bcrs, Length>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrestrial_parameter_adapter_reproduces_sofa_atio13_vector() {
        let right_ascension = 2.710_121_572_969_039;
        let declination = 0.172_937_136_721_823_04;
        let utc_first = 2_456_384.5;
        let utc_second = 0.969_254_051;
        let ut1_minus_utc = 0.155_067_5;
        let longitude = -0.527_800_806;
        let latitude = -1.234_585_6;
        let height_metres = 2_738.0;
        let polar_motion_x = 2.472_307_37e-7;
        let polar_motion_y = 1.826_404_64e-6;
        let conditions = AtmosphericConditions::new(
            super::super::AtmosphericPressure::from_hectopascals(731.0).unwrap(),
            super::super::AirTemperature::from_degrees_celsius(12.8).unwrap(),
            super::super::RelativeHumidity::from_fraction(0.59).unwrap(),
            super::super::ObservingWavelength::from_micrometres(0.55).unwrap(),
        );
        let mut source = sofars::astro::IauAstrom::default();
        sofars::astro::apio13(
            utc_first,
            utc_second,
            ut1_minus_utc,
            longitude,
            latitude,
            height_metres,
            polar_motion_x,
            polar_motion_y,
            conditions.pressure().as_hectopascals(),
            conditions.temperature().as_degrees_celsius(),
            conditions.relative_humidity().as_fraction(),
            conditions.wavelength().as_micrometres(),
            &mut source,
        )
        .unwrap();

        let parameters =
            TerrestrialObservationParameters::from_sofa(source).with_refraction(conditions);
        let (azimuth, zenith_distance, hour_angle, observed_declination, observed_right_ascension) =
            sofars::astro::atioq(right_ascension, declination, &parameters);

        let tolerance_radians = 1.0e-12;
        assert!((azimuth - 0.092_339_522_248_951_22).abs() < tolerance_radians);
        assert!((zenith_distance - 1.407_758_704_513_55).abs() < tolerance_radians);
        assert!((hour_angle - -0.092_476_198_798_816_98).abs() < tolerance_radians);
        assert!((observed_declination - 0.171_765_343_575_623_48).abs() < tolerance_radians);
        assert!((observed_right_ascension - 2.710_085_107_988_480_7).abs() < tolerance_radians);
    }
}
