#![cfg(feature = "std")]

use approx::assert_abs_diff_eq;
use hyastro::{
    astro::{Astrometry, ReceptionLightTimeOptions},
    ephem::{
        CelestialBody, Coverage, EphemerisProvenance, EphemerisProvider, EphemerisQuery, Error,
        RelativeState, SofaAnalyticEphemeris,
    },
    event::{AngularEventSearchOptions, Events, SolarTerm},
    frame::Bcrs,
    math::{Length, Speed, Vector3},
    time::{
        DateTime, Duration, GeocentricTdb, Gregorian, Instant, TimeContext, TimeInterval,
        TimeScale, Tt,
    },
};

fn tt_instant(first: f64, second: f64) -> Instant<Tt> {
    assert_eq!(first, 2_400_000.5);
    let days_since_1900 = second - 15_020.0;
    let whole_days = days_since_1900.floor();
    let fractional_day = days_since_1900 - whole_days;
    let tai_nanoseconds = whole_days as i128 * Duration::NANOSECONDS_PER_DAY
        + (fractional_day * Duration::NANOSECONDS_PER_DAY as f64).round() as i128
        - 32_184_000_000;
    Instant::<Tt>::from_instant(
        Instant::from_tai_nanoseconds_since_1900(tai_nanoseconds),
        &TimeContext::builtin(),
    )
    .unwrap()
}

fn tdb_instant(first: f64, second: f64) -> Instant<Tt> {
    let initial = tt_instant(first, second);
    let tdb_minus_tt = GeocentricTdb::new().at(initial).unwrap().tdb_minus_tt();
    initial.checked_sub(tdb_minus_tt).unwrap()
}

fn assert_state_components(
    state: RelativeState<Bcrs, Tt>,
    expected_position_au: [f64; 3],
    expected_velocity_au_per_day: [f64; 3],
    tolerance: f64,
) {
    for (actual, expected) in state
        .position()
        .components()
        .map(Length::as_astronomical_units)
        .into_iter()
        .zip(expected_position_au)
    {
        assert_abs_diff_eq!(actual, expected, epsilon = tolerance);
    }
    for (actual, expected) in state
        .velocity()
        .components()
        .map(Speed::as_astronomical_units_per_day)
        .into_iter()
        .zip(expected_velocity_au_per_day)
    {
        assert_abs_diff_eq!(actual, expected, epsilon = tolerance);
    }
}

#[test]
fn sofa_provider_preserves_official_epv00_bcrs_vectors() {
    let epoch = tdb_instant(2_400_000.5, 53_411.525_011_61);
    let provider = SofaAnalyticEphemeris::new();

    let earth = provider
        .state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Earth,
            CelestialBody::SolarSystemBarycenter,
            epoch,
        ))
        .unwrap();
    assert_state_components(
        earth,
        [
            -0.771_410_444_049_111_2,
            0.559_841_206_182_417_1,
            0.242_599_627_772_245_24,
        ],
        [
            -0.010_918_742_681_168_233,
            -0.012_465_254_617_328_615,
            -0.005_404_773_180_966_231,
        ],
        3.0e-13,
    );

    let sun_from_earth = provider
        .state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Sun,
            CelestialBody::Earth,
            epoch,
        ))
        .unwrap();
    assert_state_components(
        sun_from_earth,
        [
            0.775_723_880_929_770_7,
            -0.559_805_224_136_334,
            -0.242_699_846_648_168_7,
        ],
        [
            0.010_918_918_241_473_14,
            0.012_471_872_684_408_45,
            0.005_407_569_418_065_039,
        ],
        3.0e-13,
    );
}

#[test]
fn sofa_provider_preserves_official_moon98_geocentric_vector() {
    let epoch = tt_instant(2_400_000.5, 43_999.9);
    let state = SofaAnalyticEphemeris::new()
        .state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Moon,
            CelestialBody::Earth,
            epoch,
        ))
        .unwrap();

    assert_state_components(
        state,
        [
            -0.002_601_295_959_971_044,
            0.000_613_975_094_430_274_2,
            0.000_264_079_452_822_982_9,
        ],
        [
            -0.000_124_432_150_664_989_5,
            -0.000_521_907_694_267_811_9,
            -0.000_171_613_221_437_846_2,
        ],
        2.0e-11,
    );
}

#[test]
fn sofa_provider_preserves_official_plan94_heliocentric_vector() {
    let epoch = tdb_instant(2_400_000.5, 43_999.9);
    let state = SofaAnalyticEphemeris::new()
        .state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::MercuryBarycenter,
            CelestialBody::Sun,
            epoch,
        ))
        .unwrap();

    assert_state_components(
        state,
        [
            0.294_529_395_925_743_1,
            -0.245_220_417_660_104_96,
            -0.161_542_770_057_197_82,
        ],
        [
            0.014_138_678_714_046_144,
            0.019_465_483_011_047_067,
            0.008_929_809_783_898_905,
        ],
        1.0e-11,
    );
}

#[test]
fn sofa_provider_exposes_plan94_system_accuracy_contracts() {
    let expected = [
        (CelestialBody::MercuryBarycenter, 7.0, 500.0, 334.0),
        (CelestialBody::VenusBarycenter, 7.0, 1_100.0, 1_060.0),
        (CelestialBody::EarthMoonBarycenter, 9.0, 1_300.0, 2_010.0),
        (CelestialBody::MarsBarycenter, 26.0, 9_000.0, 7_690.0),
        (CelestialBody::JupiterBarycenter, 78.0, 82_000.0, 71_700.0),
        (CelestialBody::SaturnBarycenter, 87.0, 263_000.0, 199_000.0),
        (CelestialBody::UranusBarycenter, 86.0, 661_000.0, 564_000.0),
        (CelestialBody::NeptuneBarycenter, 11.0, 248_000.0, 158_000.0),
    ];

    for (body, longitude, radius, rms_position) in expected {
        assert!(SofaAnalyticEphemeris::supports(body));
        let accuracy = SofaAnalyticEphemeris::plan94_accuracy(body).unwrap();
        assert_eq!(accuracy.body(), body);
        assert_eq!(accuracy.maximum_longitude_arcseconds(), longitude);
        assert_eq!(accuracy.maximum_radius_kilometres(), radius);
        assert_eq!(accuracy.rms_position_kilometres(), rms_position);
        assert!(accuracy.maximum_latitude_arcseconds() > 0.0);
        assert!(accuracy.rms_velocity_metres_per_second() > 0.0);
    }
    assert!(SofaAnalyticEphemeris::plan94_accuracy(CelestialBody::Jupiter).is_none());
}

#[test]
fn sofa_provider_intersects_plan94_coverage_only_when_other_models_are_needed() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(1500, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let provider = SofaAnalyticEphemeris::new();
    let planet_sun =
        EphemerisQuery::<Bcrs, _>::new(CelestialBody::JupiterBarycenter, CelestialBody::Sun, epoch);
    let planet_earth = EphemerisQuery::<Bcrs, _>::new(
        CelestialBody::JupiterBarycenter,
        CelestialBody::Earth,
        epoch,
    );

    assert!(provider.coverage(planet_sun).unwrap().contains(epoch));
    assert!(provider.state(planet_sun).is_ok());
    assert!(!provider.coverage(planet_earth).unwrap().contains(epoch));
    assert!(matches!(
        provider.state(planet_earth),
        Err(Error::Coverage { .. })
    ));

    let outside = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(999, 12, 31, 0, 0, 0, 0).unwrap())
        .unwrap();
    assert!(matches!(
        provider.state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::MercuryBarycenter,
            CelestialBody::Sun,
            outside,
        )),
        Err(Error::Coverage { .. })
    ));
}

#[test]
fn sofa_provider_reports_body_coverage_and_provenance_boundaries() {
    let epoch = tt_instant(2_400_000.5, 53_411.525_011_61);
    let provider = SofaAnalyticEphemeris::new();
    let query = EphemerisQuery::<Bcrs, _>::new(CelestialBody::Moon, CelestialBody::Earth, epoch);

    let coverage = provider.coverage(query).unwrap();
    assert!(coverage.contains(epoch));
    assert!(coverage.start() < coverage.end());
    assert_eq!(
        provider.provenance().unwrap().model(),
        SofaAnalyticEphemeris::MODEL
    );
    assert!(
        provider
            .state(EphemerisQuery::<Bcrs, _>::new(
                CelestialBody::JupiterBarycenter,
                CelestialBody::Earth,
                epoch,
            ))
            .is_ok()
    );
    assert!(matches!(
        provider.state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Jupiter,
            CelestialBody::Earth,
            epoch,
        )),
        Err(Error::UnsupportedBody {
            body: CelestialBody::Jupiter,
            provider: SofaAnalyticEphemeris::MODEL,
        })
    ));

    assert!(matches!(
        EphemerisProvenance::try_from_model(""),
        Err(Error::EmptyModelIdentifier)
    ));
    let earlier = epoch
        .checked_sub(Duration::from_seconds(1).unwrap())
        .unwrap();
    assert!(matches!(
        Coverage::<Bcrs, _>::try_new(CelestialBody::Moon, CelestialBody::Earth, epoch, earlier,),
        Err(Error::InvalidCoverageInterval { .. })
    ));
}

#[derive(Debug, Clone, Copy)]
struct FixedOneAuEphemeris;

impl FixedOneAuEphemeris {
    fn position<F>(body: CelestialBody) -> Result<Vector3<F, Length>, Error> {
        let x = match body {
            CelestialBody::Sun => Length::from_astronomical_units(1.0)?,
            CelestialBody::Earth | CelestialBody::SolarSystemBarycenter => {
                Length::from_metres(0.0)?
            }
            _ => {
                return Err(Error::UnsupportedBody {
                    body,
                    provider: "fixed one-au test ephemeris",
                });
            }
        };
        Ok(Vector3::new(
            x,
            Length::from_metres(0.0)?,
            Length::from_metres(0.0)?,
        ))
    }
}

impl EphemerisProvider for FixedOneAuEphemeris {
    fn state<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<RelativeState<Bcrs, S>, Error> {
        if query.target() == query.center() {
            return RelativeState::zero(query.target(), query.epoch());
        }
        let position =
            Self::position(query.target())?.checked_sub(Self::position(query.center())?)?;
        let zero = Speed::from_metres_per_second(0.0)?;
        RelativeState::try_new(
            query.target(),
            query.center(),
            position,
            Vector3::new(zero, zero, zero),
            query.epoch(),
        )
    }

    fn coverage<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<Coverage<Bcrs, S>, Error> {
        let span = Duration::from_days(1)?;
        Coverage::try_new(
            query.target(),
            query.center(),
            query.epoch().checked_sub(span).unwrap(),
            query.epoch().checked_add(span).unwrap(),
        )
    }

    fn provenance(&self) -> Result<EphemerisProvenance, Error> {
        EphemerisProvenance::try_from_model("fixed one-au test ephemeris")
    }
}

#[test]
fn astrometry_accepts_an_external_provider_without_kernel_types() {
    let epoch = tt_instant(2_400_000.5, 53_411.525_011_61);
    let time = TimeContext::builtin();
    let provider = FixedOneAuEphemeris;
    let astrometry = Astrometry::new(&time, &provider);

    let result = astrometry
        .reception_light_time(
            EphemerisQuery::<Bcrs, _>::new(CelestialBody::Sun, CelestialBody::Earth, epoch),
            ReceptionLightTimeOptions::standard(),
        )
        .unwrap();

    assert_abs_diff_eq!(
        result.distance().as_astronomical_units(),
        1.0,
        epsilon = 1.0e-15
    );
    assert!((499.0..500.0).contains(&result.light_time().as_seconds_f64()));
    assert_eq!(result.iterations(), 1);
    assert_eq!(
        astrometry.ephemeris().provenance().unwrap().model(),
        "fixed one-au test ephemeris"
    );
}

#[test]
fn default_analytic_backend_runs_an_event_workflow_without_kernels() {
    let time = TimeContext::builtin();
    let provider = SofaAnalyticEphemeris::new();
    let astrometry = Astrometry::new(&time, &provider);
    let start = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 3, 16, 0, 0, 0, 0).unwrap())
        .unwrap();
    let end = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 3, 25, 0, 0, 0, 0).unwrap())
        .unwrap();

    let terms = Events::new(astrometry)
        .solar_terms_in(
            TimeInterval::new(start, end).unwrap(),
            AngularEventSearchOptions::standard(),
        )
        .unwrap();

    assert_eq!(terms.len(), 1);
    assert_eq!(terms[0].term(), SolarTerm::SpringEquinox);
    assert!(start < terms[0].instant() && terms[0].instant() < end);
}

#[test]
fn default_analytic_backend_runs_lunar_illumination_without_kernels() {
    let time = TimeContext::builtin();
    let provider = SofaAnalyticEphemeris::new();
    let astrometry = Astrometry::new(&time, &provider);
    let epoch = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 3, 25, 7, 0, 0, 0).unwrap())
        .unwrap();

    let illumination = astrometry
        .lunar_illumination_at(epoch, ReceptionLightTimeOptions::standard())
        .unwrap();

    assert_eq!(illumination.reception_epoch(), epoch);
    assert!(illumination.illuminated_fraction().as_ratio() > 0.99);
    assert!(illumination.phase_angle().as_degrees() < 10.0);
}

#[cfg(feature = "anise")]
#[test]
#[ignore = "requires HYASTRO_DE440S to name a local de440s.bsp"]
fn de440s_plan94_position_differences_stay_within_published_bounds() {
    use hyastro::ephem::{Ephemeris, KernelManifest};

    const ARCSECONDS_TO_RADIANS: f64 = core::f64::consts::PI / 648_000.0;
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let reference = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let analytic = SofaAnalyticEphemeris::new();
    let time = TimeContext::builtin();
    let bodies = [
        CelestialBody::MercuryBarycenter,
        CelestialBody::VenusBarycenter,
        CelestialBody::EarthMoonBarycenter,
        CelestialBody::MarsBarycenter,
        CelestialBody::JupiterBarycenter,
        CelestialBody::SaturnBarycenter,
        CelestialBody::UranusBarycenter,
        CelestialBody::NeptuneBarycenter,
    ];

    for year in [1900, 2000, 2024, 2100] {
        let epoch = time
            .resolve(DateTime::<Gregorian, Tt>::from_components(year, 1, 1, 12, 0, 0, 0).unwrap())
            .unwrap();
        for body in bodies {
            let query = EphemerisQuery::<Bcrs, _>::new(body, CelestialBody::Sun, epoch);
            let actual = analytic.state(query).unwrap();
            let expected = reference.state(query).unwrap();
            let position_difference = actual
                .position()
                .checked_sub(expected.position())
                .unwrap()
                .magnitude()
                .unwrap()
                .as_kilometres();
            let velocity_difference = actual
                .velocity()
                .checked_sub(expected.velocity())
                .unwrap()
                .magnitude()
                .unwrap()
                .as_metres_per_second();
            let reference_radius = expected.position().magnitude().unwrap().as_kilometres();
            let accuracy = SofaAnalyticEphemeris::plan94_accuracy(body).unwrap();
            let angular_bound = accuracy
                .maximum_longitude_arcseconds()
                .hypot(accuracy.maximum_latitude_arcseconds())
                * ARCSECONDS_TO_RADIANS;
            let position_bound =
                accuracy.maximum_radius_kilometres() + reference_radius * angular_bound;

            eprintln!(
                "{year} {body}: position difference {position_difference:.3} km (published component bound {position_bound:.3} km), velocity difference {velocity_difference:.6} m/s"
            );
            assert!(
                position_difference <= position_bound,
                "{year} {body}: {position_difference} km exceeds {position_bound} km"
            );
        }
    }
}
