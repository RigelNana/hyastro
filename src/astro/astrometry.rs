use core::fmt;

use crate::{
    earth::{FixedSite, TopocentricFrame},
    ephem::{CelestialBody, Ephemeris, EphemerisQuery, RelativeState},
    frame::{
        Bcrs, Cirs, EarthOrientationSolution, EclipticDirectionAt, EclipticLatitude,
        EclipticLongitude, EquatorialDirection, EquatorialDirectionAt, Frames, Gcrs,
        HorizontalDirection, TrueEclipticEquinoxOfDate,
    },
    math::{Direction, Length, Speed, Vector3},
    time::{
        Duration, EarthOrientationTable, Hifitime, Instant, JulianDate, Tdb, TimeContext, TimeScale,
    },
};

use super::Error;

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

/// The geocentric apparent Sun expressed on true ecliptic and equinox of date axes.
///
/// The result includes converged reception light time and SOFA relativistic
/// annual aberration. The axes include IAU 2006 frame bias and precession,
/// IAU 2000A nutation, and true obliquity. It excludes station parallax,
/// atmospheric refraction, solar-limb geometry, and point-mass deflection by
/// the Sun of its own apparent centre.
pub struct SolarApparentEcliptic<S: TimeScale> {
    reception_epoch: Instant<S>,
    emission_epoch: Instant<S>,
    light_time: Duration,
    coordinates: EclipticDirectionAt<TrueEclipticEquinoxOfDate, S>,
    distance: Length,
    iterations: u32,
    light_time_residual: Duration,
}

impl<S: TimeScale> SolarApparentEcliptic<S> {
    /// Returns the geocentric reception epoch.
    pub const fn reception_epoch(self) -> Instant<S> {
        self.reception_epoch
    }

    /// Returns the retarded solar emission epoch.
    pub const fn emission_epoch(self) -> Instant<S> {
        self.emission_epoch
    }

    /// Returns the converged one-way solar light time.
    pub const fn light_time(self) -> Duration {
        self.light_time
    }

    /// Returns the epoch-bound true-ecliptic coordinates.
    pub const fn coordinates(self) -> EclipticDirectionAt<TrueEclipticEquinoxOfDate, S> {
        self.coordinates
    }

    /// Returns the apparent geocentric solar longitude.
    pub const fn longitude(self) -> EclipticLongitude {
        self.coordinates.coordinates().longitude()
    }

    /// Returns the apparent geocentric solar latitude.
    pub const fn latitude(self) -> EclipticLatitude {
        self.coordinates.coordinates().latitude()
    }

    /// Returns the Sun-at-emission to Earth-at-reception distance.
    pub const fn distance(self) -> Length {
        self.distance
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

impl<S: TimeScale> Copy for SolarApparentEcliptic<S> {}

impl<S: TimeScale> Clone for SolarApparentEcliptic<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for SolarApparentEcliptic<S> {
    fn eq(&self, other: &Self) -> bool {
        self.reception_epoch == other.reception_epoch
            && self.emission_epoch == other.emission_epoch
            && self.light_time == other.light_time
            && self.coordinates == other.coordinates
            && self.distance == other.distance
            && self.iterations == other.iterations
            && self.light_time_residual == other.light_time_residual
    }
}

impl<S: TimeScale> fmt::Debug for SolarApparentEcliptic<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SolarApparentEcliptic")
            .field("reception_epoch", &self.reception_epoch)
            .field("emission_epoch", &self.emission_epoch)
            .field("light_time", &self.light_time)
            .field("coordinates", &self.coordinates)
            .field("distance", &self.distance)
            .field("iterations", &self.iterations)
            .field("light_time_residual", &self.light_time_residual)
            .finish()
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
    iterations: u32,
    residual: Duration,
}

/// Astrometric state for one fixed terrestrial observer at one reception epoch.
///
/// The value freezes the site's GCRS state, full observed Earth orientation,
/// Earth ephemeris, and star-independent aberration parameters. Multiple
/// solar-system targets at the same site and epoch can reuse this preparation.
#[derive(Debug, Clone, Copy)]
pub struct FixedObserverAt<'ephemeris, S: TimeScale> {
    ephemeris: &'ephemeris Ephemeris,
    topocentric_frame: TopocentricFrame<S>,
    earth_orientation: EarthOrientationSolution<S>,
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

    /// Computes a finite solar-system target's vacuum observed place.
    ///
    /// The result includes station-aware reception light time, topocentric
    /// parallax, relativistic aberration from the combined barycentric
    /// observer velocity, IAU 2006/2000A Earth orientation, polar motion, and
    /// local horizontal projection. It excludes atmospheric refraction,
    /// Shapiro delay, and point-mass light deflection.
    pub fn vacuum_observed_place(
        self,
        target: CelestialBody,
        options: ReceptionLightTimeOptions,
    ) -> Result<VacuumObservedPlace<S>, Error> {
        let reception = self.solve_reception_light_time(target, options)?;
        let proper_components = sofars::astro::ab(
            &reception.direction.components(),
            &self.parameters.v,
            self.parameters.em,
            self.parameters.bm1,
        );
        let proper_direction = Direction::<Gcrs>::try_from_components(proper_components)?;
        let proper_equatorial = EquatorialDirection::from_direction(proper_direction)?;
        let intermediate = self
            .earth_orientation
            .intermediate_equatorial(proper_equatorial)?;
        let horizontal = self
            .topocentric_frame
            .horizontal_direction(proper_direction)?;

        Ok(VacuumObservedPlace {
            target,
            topocentric_frame: self.topocentric_frame,
            emission_epoch: reception.emission_epoch,
            light_time: reception.light_time,
            intermediate,
            horizontal,
            distance: reception.distance,
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
    horizontal: HorizontalDirection,
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

    /// Computes the geocentric apparent Sun on true ecliptic and equinox of date axes.
    pub fn solar_apparent_ecliptic<S: TimeScale>(
        &self,
        reception_epoch: Instant<S>,
        light_time_options: ReceptionLightTimeOptions,
    ) -> Result<SolarApparentEcliptic<S>, Error> {
        let query = EphemerisQuery::new(CelestialBody::Sun, CelestialBody::Earth, reception_epoch);
        let light_time = self.solve_reception_light_time(query, light_time_options)?;
        let proper_direction = self.geocentric_aberration(
            reception_epoch,
            light_time.result.direction,
            light_time.observer_barycentric,
            light_time.target_reception_position,
        )?;
        let coordinates = Frames::new(self.time)
            .celestial_orientation_at(reception_epoch)?
            .true_ecliptic_from_gcrs(proper_direction)?;

        Ok(SolarApparentEcliptic {
            reception_epoch,
            emission_epoch: light_time.result.emission_epoch,
            light_time: light_time.result.light_time,
            coordinates,
            distance: light_time.result.distance,
            iterations: light_time.result.iterations,
            light_time_residual: light_time.result.residual,
        })
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

        Ok(FixedObserverAt {
            ephemeris: self.ephemeris,
            topocentric_frame,
            earth_orientation,
            barycentric,
            parameters,
        })
    }
}

struct ReceptionComputation<S: TimeScale> {
    result: ReceptionLightTime<S>,
    observer_barycentric: RelativeState<Bcrs, S>,
    target_reception_position: Vector3<Bcrs, Length>,
}
