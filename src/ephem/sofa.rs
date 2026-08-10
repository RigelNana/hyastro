use crate::{
    frame::Bcrs,
    math::{Length, Speed, Vector3},
    time::{DateTime, GeocentricTdb, Gregorian, Instant, TimeContext, TimeScale, Tt},
};

use super::{
    CelestialBody, Coverage, EphemerisProvenance, EphemerisProvider, EphemerisQuery, Error,
    RelativeState,
};

const MOON_ACCURACY_START_TT_JD: f64 = 2_433_282.5;
const MOON_ACCURACY_END_TT_JD: f64 = 2_488_069.5;
const PLAN94_ACCURACY_START_YEAR: i32 = 1000;
const PLAN94_ACCURACY_END_YEAR: i32 = 3000;

/// Published error statistics for one SOFA `plan94` planetary-system state.
///
/// Maximum angular and radius differences are against JPL DE200 over
/// 1800–2100 (DE406 was essentially the same). RMS position and velocity
/// differences are against DE200 over 1960–2025.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan94Accuracy {
    body: CelestialBody,
    maximum_longitude_arcseconds: f64,
    maximum_latitude_arcseconds: f64,
    maximum_radius_kilometres: f64,
    rms_position_kilometres: f64,
    rms_velocity_metres_per_second: f64,
}

impl Plan94Accuracy {
    const fn new(
        body: CelestialBody,
        maximum_longitude_arcseconds: f64,
        maximum_latitude_arcseconds: f64,
        maximum_radius_kilometres: f64,
        rms_position_kilometres: f64,
        rms_velocity_metres_per_second: f64,
    ) -> Self {
        Self {
            body,
            maximum_longitude_arcseconds,
            maximum_latitude_arcseconds,
            maximum_radius_kilometres,
            rms_position_kilometres,
            rms_velocity_metres_per_second,
        }
    }

    /// Returns the planetary-system barycentre represented by the statistics.
    pub const fn body(self) -> CelestialBody {
        self.body
    }

    /// Returns the maximum ecliptic-longitude difference over 1800–2100.
    pub const fn maximum_longitude_arcseconds(self) -> f64 {
        self.maximum_longitude_arcseconds
    }

    /// Returns the maximum ecliptic-latitude difference over 1800–2100.
    pub const fn maximum_latitude_arcseconds(self) -> f64 {
        self.maximum_latitude_arcseconds
    }

    /// Returns the maximum radius difference in kilometres over 1800–2100.
    pub const fn maximum_radius_kilometres(self) -> f64 {
        self.maximum_radius_kilometres
    }

    /// Returns the RMS Cartesian-position difference over 1960–2025.
    pub const fn rms_position_kilometres(self) -> f64 {
        self.rms_position_kilometres
    }

    /// Returns the RMS Cartesian-velocity difference over 1960–2025.
    pub const fn rms_velocity_metres_per_second(self) -> f64 {
        self.rms_velocity_metres_per_second
    }
}

#[derive(Clone, Copy)]
struct CartesianPv {
    position_au: [f64; 3],
    velocity_au_per_day: [f64; 3],
}

impl CartesianPv {
    const ZERO: Self = Self {
        position_au: [0.0; 3],
        velocity_au_per_day: [0.0; 3],
    };

    fn from_sofa(value: [[f64; 3]; 2]) -> Self {
        Self {
            position_au: value[0],
            velocity_au_per_day: value[1],
        }
    }

    fn checked_add(self, other: Self) -> Result<Self, Error> {
        Self::from_components(
            core::array::from_fn(|index| self.position_au[index] + other.position_au[index]),
            core::array::from_fn(|index| {
                self.velocity_au_per_day[index] + other.velocity_au_per_day[index]
            }),
        )
    }

    fn checked_sub(self, other: Self) -> Result<Self, Error> {
        Self::from_components(
            core::array::from_fn(|index| self.position_au[index] - other.position_au[index]),
            core::array::from_fn(|index| {
                self.velocity_au_per_day[index] - other.velocity_au_per_day[index]
            }),
        )
    }

    fn from_components(
        position_au: [f64; 3],
        velocity_au_per_day: [f64; 3],
    ) -> Result<Self, Error> {
        for value in position_au {
            crate::math::Error::ensure_finite("analytical ephemeris position", value)?;
        }
        for value in velocity_au_per_day {
            crate::math::Error::ensure_finite("analytical ephemeris velocity", value)?;
        }
        Ok(Self {
            position_au,
            velocity_au_per_day,
        })
    }

    fn position(self) -> Result<Vector3<Bcrs, Length>, Error> {
        Ok(Vector3::new(
            Length::from_astronomical_units(self.position_au[0])?,
            Length::from_astronomical_units(self.position_au[1])?,
            Length::from_astronomical_units(self.position_au[2])?,
        ))
    }

    fn velocity(self) -> Result<Vector3<Bcrs, Speed>, Error> {
        Ok(Vector3::new(
            Speed::from_astronomical_units_per_day(self.velocity_au_per_day[0])?,
            Speed::from_astronomical_units_per_day(self.velocity_au_per_day[1])?,
            Speed::from_astronomical_units_per_day(self.velocity_au_per_day[2])?,
        ))
    }
}

/// Limited-precision, allocation-free solar-system ephemeris from IAU SOFA.
///
/// Earth and Sun barycentric states come from SOFA `epv00`; the Moon's
/// geocentric GCRS state comes from `moon98`. `plan94` supplies heliocentric
/// states for the Mercury, Venus, Earth-Moon, Mars, Jupiter, Saturn, Uranus,
/// and Neptune system barycentres. The planetary identities deliberately use
/// system barycentres: PLAN94's orbital elements and published JPL comparisons
/// represent planetary systems, not satellite-resolved body centres.
///
/// SOFA aligns `epv00` with BCRS axes. `moon98` and `plan94` use J2000 mean
/// equator and equinox axes; their frame-bias difference from BCRS is below
/// these analytical models' published error bounds.
///
/// Sun/Earth/SSB queries use the `epv00` 1900–2100 accuracy interval. Queries
/// involving the Moon use the narrower `moon98` 1950–2100 interval. A
/// PLAN94-system query relative to the Sun or another PLAN94 system has
/// 1000–3000 coverage; combining it with Earth, Moon, or the SSB intersects
/// that range with the applicable `epv00` or `moon98` range. Unsupported
/// bodies, dates outside those declared intervals, and analytical failures
/// return structured errors rather than extrapolated states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SofaAnalyticEphemeris;

impl SofaAnalyticEphemeris {
    /// Stable model identifier for result provenance.
    pub const MODEL: &'static str = "SOFA epv00 + moon98 + plan94";

    /// RMS heliocentric Earth position error against DE405 over 1900–2100.
    pub const EARTH_HELIOCENTRIC_POSITION_RMS_KILOMETRES: f64 = 3.7;

    /// RMS barycentric Earth position error against DE405 over 1900–2100.
    pub const EARTH_BARYCENTRIC_POSITION_RMS_KILOMETRES: f64 = 4.6;

    /// RMS geocentric Moon direction error against ELP/MPP02 over 1950–2100.
    pub const MOON_DIRECTION_RMS_ARCSECONDS: f64 = 2.9;

    /// RMS geocentric Moon position error against ELP/MPP02 over 1950–2100.
    pub const MOON_POSITION_RMS_KILOMETRES: f64 = 6.1;

    /// Returns PLAN94's published per-system error statistics, when applicable.
    pub const fn plan94_accuracy(body: CelestialBody) -> Option<Plan94Accuracy> {
        match body {
            CelestialBody::MercuryBarycenter => {
                Some(Plan94Accuracy::new(body, 7.0, 1.0, 500.0, 334.0, 0.437))
            }
            CelestialBody::VenusBarycenter => {
                Some(Plan94Accuracy::new(body, 7.0, 1.0, 1_100.0, 1_060.0, 0.855))
            }
            CelestialBody::EarthMoonBarycenter => {
                Some(Plan94Accuracy::new(body, 9.0, 1.0, 1_300.0, 2_010.0, 0.815))
            }
            CelestialBody::MarsBarycenter => {
                Some(Plan94Accuracy::new(body, 26.0, 1.0, 9_000.0, 7_690.0, 1.98))
            }
            CelestialBody::JupiterBarycenter => Some(Plan94Accuracy::new(
                body, 78.0, 6.0, 82_000.0, 71_700.0, 7.70,
            )),
            CelestialBody::SaturnBarycenter => Some(Plan94Accuracy::new(
                body, 87.0, 14.0, 263_000.0, 199_000.0, 19.4,
            )),
            CelestialBody::UranusBarycenter => Some(Plan94Accuracy::new(
                body, 86.0, 7.0, 661_000.0, 564_000.0, 16.4,
            )),
            CelestialBody::NeptuneBarycenter => Some(Plan94Accuracy::new(
                body, 11.0, 2.0, 248_000.0, 158_000.0, 14.4,
            )),
            _ => None,
        }
    }

    /// Constructs the stateless analytical provider.
    pub const fn new() -> Self {
        Self
    }

    /// Reports whether this provider models a body identity.
    pub const fn supports(body: CelestialBody) -> bool {
        matches!(
            body,
            CelestialBody::SolarSystemBarycenter
                | CelestialBody::MercuryBarycenter
                | CelestialBody::VenusBarycenter
                | CelestialBody::EarthMoonBarycenter
                | CelestialBody::MarsBarycenter
                | CelestialBody::JupiterBarycenter
                | CelestialBody::SaturnBarycenter
                | CelestialBody::UranusBarycenter
                | CelestialBody::NeptuneBarycenter
                | CelestialBody::Sun
                | CelestialBody::Earth
                | CelestialBody::Moon
        )
    }

    /// Evaluates one geometric BCRS target-minus-centre state.
    pub fn state<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<RelativeState<Bcrs, S>, Error> {
        <Self as EphemerisProvider>::state(self, query)
    }

    /// Returns the provider's inclusive validated coverage for one query.
    pub fn coverage<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<Coverage<Bcrs, S>, Error> {
        <Self as EphemerisProvider>::coverage(self, query)
    }

    fn ensure_supported(body: CelestialBody) -> Result<(), Error> {
        if Self::supports(body) {
            Ok(())
        } else {
            Err(Error::UnsupportedBody {
                body,
                provider: Self::MODEL,
            })
        }
    }

    fn coverage_error<S: TimeScale>(query: EphemerisQuery<Bcrs, S>) -> Error {
        Error::Coverage {
            target: query.target(),
            center: query.center(),
            epoch_tai_nanoseconds: query.epoch().tai_nanoseconds_since_1900(),
        }
    }

    const fn plan94_index(body: CelestialBody) -> Option<i32> {
        match body {
            CelestialBody::MercuryBarycenter => Some(1),
            CelestialBody::VenusBarycenter => Some(2),
            CelestialBody::EarthMoonBarycenter => Some(3),
            CelestialBody::MarsBarycenter => Some(4),
            CelestialBody::JupiterBarycenter => Some(5),
            CelestialBody::SaturnBarycenter => Some(6),
            CelestialBody::UranusBarycenter => Some(7),
            CelestialBody::NeptuneBarycenter => Some(8),
            _ => None,
        }
    }

    const fn requires_epv00(body: CelestialBody) -> bool {
        matches!(
            body,
            CelestialBody::SolarSystemBarycenter | CelestialBody::Earth | CelestialBody::Moon
        )
    }

    fn plan94_state<S: TimeScale>(
        body: CelestialBody,
        query: EphemerisQuery<Bcrs, S>,
        tdb_first: f64,
        tdb_second: f64,
    ) -> Result<CartesianPv, Error> {
        let index = Self::plan94_index(body).ok_or(Error::UnsupportedBody {
            body,
            provider: Self::MODEL,
        })?;
        let (state, status) =
            sofars::eph::plan94(tdb_first, tdb_second, index).map_err(|status| {
                Error::AnalyticalModelFailure {
                    body,
                    provider: Self::MODEL,
                    status,
                }
            })?;
        match status {
            0 => Ok(CartesianPv::from_sofa(state)),
            1 => Err(Self::coverage_error(query)),
            _ => Err(Error::AnalyticalModelFailure {
                body,
                provider: Self::MODEL,
                status,
            }),
        }
    }

    fn heliocentric_state<S: TimeScale>(
        body: CelestialBody,
        query: EphemerisQuery<Bcrs, S>,
        tdb_first: f64,
        tdb_second: f64,
        earth_heliocentric: CartesianPv,
        earth_barycentric: CartesianPv,
        moon_geocentric: CartesianPv,
    ) -> Result<CartesianPv, Error> {
        match body {
            CelestialBody::SolarSystemBarycenter => {
                earth_heliocentric.checked_sub(earth_barycentric)
            }
            CelestialBody::Sun => Ok(CartesianPv::ZERO),
            CelestialBody::Earth => Ok(earth_heliocentric),
            CelestialBody::Moon => earth_heliocentric.checked_add(moon_geocentric),
            _ => Self::plan94_state(body, query, tdb_first, tdb_second),
        }
    }

    fn coverage_bound<S: TimeScale>(year: i32, hour: u8) -> Result<Instant<S>, Error> {
        let label = DateTime::<Gregorian, Tt>::from_components(year, 1, 1, hour, 0, 0, 0)?;
        Ok(TimeContext::builtin().resolve(label)?.retag())
    }
}

impl EphemerisProvider for SofaAnalyticEphemeris {
    fn state<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<RelativeState<Bcrs, S>, Error> {
        if query.target() == query.center() {
            return RelativeState::zero(query.target(), query.epoch());
        }
        let coverage = self.coverage(query)?;
        if !coverage.contains(query.epoch()) {
            return Err(Self::coverage_error(query));
        }

        let time = GeocentricTdb::new().at(query.epoch())?;
        let (tdb_first, tdb_second) = time.barycentric_dynamical_time().parts();
        let uses_epv00 =
            Self::requires_epv00(query.target()) || Self::requires_epv00(query.center());
        let (earth_heliocentric, earth_barycentric) = if uses_epv00 {
            let (heliocentric, barycentric) = sofars::eph::epv00(tdb_first, tdb_second)
                .ok_or_else(|| Self::coverage_error(query))?;
            (
                CartesianPv::from_sofa(heliocentric),
                CartesianPv::from_sofa(barycentric),
            )
        } else {
            (CartesianPv::ZERO, CartesianPv::ZERO)
        };

        let uses_moon =
            query.target() == CelestialBody::Moon || query.center() == CelestialBody::Moon;
        let moon_geocentric = if uses_moon {
            let terrestrial_time = time.terrestrial_time();
            if !(MOON_ACCURACY_START_TT_JD..=MOON_ACCURACY_END_TT_JD)
                .contains(&terrestrial_time.as_f64_lossy())
            {
                return Err(Self::coverage_error(query));
            }
            let (tt_first, tt_second) = terrestrial_time.parts();
            CartesianPv::from_sofa(sofars::eph::moon98(tt_first, tt_second))
        } else {
            CartesianPv::ZERO
        };

        let target = Self::heliocentric_state(
            query.target(),
            query,
            tdb_first,
            tdb_second,
            earth_heliocentric,
            earth_barycentric,
            moon_geocentric,
        )?;
        let center = Self::heliocentric_state(
            query.center(),
            query,
            tdb_first,
            tdb_second,
            earth_heliocentric,
            earth_barycentric,
            moon_geocentric,
        )?;
        let relative = target.checked_sub(center)?;

        RelativeState::try_new(
            query.target(),
            query.center(),
            relative.position()?,
            relative.velocity()?,
            query.epoch(),
        )
    }

    fn coverage<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<Coverage<Bcrs, S>, Error> {
        if query.target() == query.center() {
            return Ok(Coverage::from_ordered(
                query.target(),
                query.center(),
                query.epoch(),
                query.epoch(),
            ));
        }
        Self::ensure_supported(query.target())?;
        Self::ensure_supported(query.center())?;

        let uses_moon =
            query.target() == CelestialBody::Moon || query.center() == CelestialBody::Moon;
        let uses_epv00 =
            Self::requires_epv00(query.target()) || Self::requires_epv00(query.center());
        let (start, end) = if uses_moon {
            (
                Self::coverage_bound(1950, 0)?,
                Self::coverage_bound(2100, 0)?,
            )
        } else if uses_epv00 {
            (
                Self::coverage_bound(1900, 12)?,
                Self::coverage_bound(2100, 0)?,
            )
        } else {
            (
                Self::coverage_bound(PLAN94_ACCURACY_START_YEAR, 0)?,
                Self::coverage_bound(PLAN94_ACCURACY_END_YEAR, 0)?,
            )
        };
        Ok(Coverage::from_ordered(
            query.target(),
            query.center(),
            start,
            end,
        ))
    }

    fn provenance(&self) -> Result<EphemerisProvenance, Error> {
        EphemerisProvenance::try_from_model(Self::MODEL)
    }
}
