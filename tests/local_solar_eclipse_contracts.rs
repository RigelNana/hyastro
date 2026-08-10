use hyastro::{
    astro::Astrometry,
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::SofaAnalyticEphemeris,
    event::{AngularEventSearchOptions, Events, LocalSolarEclipseKind, SolarEclipseSearchOptions},
    math::Angle,
    time::{
        DateTime, DeltaTEstimate, DeltaTModel, Duration, EarthAttitudeOffsetModel,
        EarthOrientationAcceptance, EarthOrientationTable, Gregorian, IersC04, ModifiedJulianDate,
        PredictedEarthOrientation, PredictionDisposition, TimeContext, TimeInterval, Tt, Utc,
    },
};

#[cfg(feature = "anise")]
use hyastro::{
    ephem::{CelestialBody, Ephemeris, KernelManifest, SphericalBodyFigure},
    event::SolarEclipseModel,
    math::Length,
    time::Instant,
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");

fn dallas_site() -> hyastro::earth::FixedSite {
    Earth::wgs84()
        .fixed_site(
            "Dallas, Texas",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(-96.7970).unwrap(),
                GeodeticLatitude::try_from_degrees(32.7767).unwrap(),
                EllipsoidalHeight::from_metres(131.0).unwrap(),
            ),
        )
        .unwrap()
}

#[cfg(feature = "anise")]
fn ut1(
    time: &TimeContext<'_, EarthOrientationTable<'_>>,
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
) -> Instant<Utc> {
    let same_clock_utc = time
        .resolve(
            DateTime::<Gregorian, Utc>::from_components(
                2024, 4, 8, hour, minute, second, nanosecond,
            )
            .unwrap(),
        )
        .unwrap();
    same_clock_utc
        .checked_sub(
            time.earth_orientation_at(same_clock_utc)
                .unwrap()
                .ut1_minus_utc()
                .as_duration(),
        )
        .unwrap()
}

#[test]
fn analytic_local_eclipse_returns_a_complete_ordered_total_sequence() {
    let base = TimeContext::builtin();
    let data = IersC04::parse(C04).unwrap();
    let samples = data
        .try_samples_in(
            &base,
            ModifiedJulianDate::<Utc>::from_parts(60_406.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(60_410.0, 0.0).unwrap(),
            EarthOrientationAcceptance::FinalOnly,
        )
        .unwrap();
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let eop = EarthOrientationTable::new(&samples, "C04 analytic eclipse test", expires).unwrap();
    let time = base.with_earth_orientation(eop);
    let start = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 4, 8, 0, 0, 0, 0).unwrap())
        .unwrap();
    let end = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 4, 9, 0, 0, 0, 0).unwrap())
        .unwrap();
    let ephemeris = SofaAnalyticEphemeris::new();
    let eclipses = Events::new(Astrometry::new(&time, &ephemeris))
        .local_solar_eclipses_in(
            &dallas_site(),
            TimeInterval::new(start, end).unwrap(),
            SolarEclipseSearchOptions::standard(),
        )
        .unwrap();

    assert_eq!(eclipses.len(), 1);
    let eclipse = &eclipses[0];
    assert_eq!(eclipse.kind(), LocalSolarEclipseKind::Total);
    let second = eclipse.second_contact().unwrap();
    let third = eclipse.third_contact().unwrap();
    assert!(eclipse.first_contact().instant() < second.instant());
    assert!(second.instant() < eclipse.maximum().instant());
    assert!(eclipse.maximum().instant() < third.instant());
    assert!(third.instant() < eclipse.fourth_contact().instant());
    assert!(eclipse.maximum().observation().magnitude().as_ratio() > 1.0);
    assert_eq!(
        eclipse.maximum().observation().obscuration().as_ratio(),
        1.0
    );
    assert!(
        eclipse
            .first_contact()
            .observation()
            .solar_disk_is_above_horizon()
    );
    assert_eq!(
        eclipse.ephemeris_provenance().model(),
        SofaAnalyticEphemeris::MODEL
    );
    assert_eq!(
        eclipse.earth_attitude_provenance().source(),
        "C04 analytic eclipse test"
    );
}

// USNO 2024 local circumstances for 32.7767° N, 96.7970° W, height 131 m:
// https://aa.usno.navy.mil/data/Eclipse2024
// USNO uses a 696000 km solar radius and reports UT1 contact times.
#[cfg(feature = "anise")]
#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-series BSP"]
fn de440_dallas_totality_matches_usno_local_circumstances() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let base = TimeContext::builtin();
    let data = IersC04::parse(C04).unwrap();
    let samples = data
        .try_samples_in(
            &base,
            ModifiedJulianDate::<Utc>::from_parts(60_406.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(60_410.0, 0.0).unwrap(),
            EarthOrientationAcceptance::FinalOnly,
        )
        .unwrap();
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let eop = EarthOrientationTable::new(&samples, "IERS C04 USNO comparison", expires).unwrap();
    let time = base.with_earth_orientation(eop);
    let start = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 4, 8, 0, 0, 0, 0).unwrap())
        .unwrap();
    let end = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 4, 9, 0, 0, 0, 0).unwrap())
        .unwrap();
    let usno_model = SolarEclipseModel::new(
        SphericalBodyFigure::new(
            CelestialBody::Sun,
            "USNO 2024 adopted 696000 km solar radius",
            Length::from_kilometres(696_000.0).unwrap(),
        )
        .unwrap(),
        SphericalBodyFigure::IAU_WGCCRE_2015_MOON,
    )
    .unwrap();
    let options = SolarEclipseSearchOptions::new(
        hyastro::event::AngularEventSearchOptions::standard(),
        usno_model,
    );
    let eclipses = Events::new(Astrometry::new(&time, &ephemeris))
        .local_solar_eclipses_in(
            &dallas_site(),
            TimeInterval::new(start, end).unwrap(),
            options,
        )
        .unwrap();

    assert_eq!(eclipses.len(), 1);
    let eclipse = &eclipses[0];
    assert_eq!(eclipse.kind(), LocalSolarEclipseKind::Total);
    let second = eclipse.second_contact().unwrap();
    let third = eclipse.third_contact().unwrap();
    let expected = [
        ut1(&time, 17, 23, 14, 600_000_000),
        ut1(&time, 18, 40, 37, 700_000_000),
        ut1(&time, 18, 42, 33, 0),
        ut1(&time, 18, 44, 30, 200_000_000),
        ut1(&time, 20, 2, 35, 700_000_000),
    ];
    let actual = [
        eclipse.first_contact().instant(),
        second.instant(),
        eclipse.maximum().instant(),
        third.instant(),
        eclipse.fourth_contact().instant(),
    ];
    let time_errors = actual
        .map(|actual| actual.tai_nanoseconds_since_1900())
        .into_iter()
        .zip(expected.map(|expected| expected.tai_nanoseconds_since_1900()))
        .map(|(actual, expected)| actual.abs_diff(expected) as f64 / 1.0e9)
        .collect::<Vec<_>>();
    assert!(
        time_errors.iter().all(|error| *error < 6.0),
        "USNO contact errors {time_errors:?}"
    );
    let position_angles = [
        eclipse.first_contact().limb_position_angle().as_degrees(),
        second.limb_position_angle().as_degrees(),
        third.limb_position_angle().as_degrees(),
        eclipse.fourth_contact().limb_position_angle().as_degrees(),
    ];
    for (actual, expected) in position_angles.into_iter().zip([226.2, 19.1, 255.5, 49.2]) {
        assert!(
            (actual - expected).abs() < 1.0,
            "position angle {actual} vs {expected}"
        );
    }
    assert!((eclipse.maximum().observation().magnitude().as_ratio() - 1.015).abs() < 0.0015);
    assert_eq!(
        eclipse.maximum().observation().obscuration().as_ratio(),
        1.0
    );
    assert!((eclipse.partial_phase_duration().as_seconds_f64() - 9_561.1).abs() < 2.0);
    assert!((eclipse.central_phase_duration().unwrap().as_seconds_f64() - 232.5).abs() < 2.0);
}

#[test]
fn local_search_finds_totality_when_geocentric_new_moon_precedes_the_site_maximum() {
    let base = TimeContext::builtin();
    let validity_start = base
        .resolve(DateTime::<Gregorian, Tt>::from_components(2035, 8, 30, 0, 0, 0, 0).unwrap())
        .unwrap();
    let validity_end = base
        .resolve(DateTime::<Gregorian, Tt>::from_components(2035, 9, 6, 0, 0, 0, 0).unwrap())
        .unwrap();
    let delta_t = DeltaTModel::constant(
        "NASA 2035 path-table Delta T 80.6 s",
        TimeInterval::new(validity_start, validity_end).unwrap(),
        DeltaTEstimate::new(Duration::from_seconds_f64(80.6).unwrap(), None),
    )
    .unwrap();
    let prediction = PredictedEarthOrientation::new(
        "NASA 2035 path-table scenario",
        delta_t,
        EarthAttitudeOffsetModel::assumed_zero(),
    )
    .unwrap();
    let time = base.with_predicted_earth_orientation(prediction);
    let site = Earth::wgs84()
        .fixed_site(
            "119.7446 E, 40.0000 N",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(119.7446).unwrap(),
                GeodeticLatitude::try_from_degrees(40.0).unwrap(),
                EllipsoidalHeight::from_metres(0.0).unwrap(),
            ),
        )
        .unwrap();
    let start = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2035, 9, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let end = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2035, 9, 3, 0, 0, 0, 0).unwrap())
        .unwrap();
    let standard = AngularEventSearchOptions::standard();
    let angular = AngularEventSearchOptions::new(
        standard.scan_step(),
        standard.time_tolerance(),
        Angle::from_radians(2.0e-11).unwrap(),
        standard.max_refinement_iterations(),
        standard.max_evaluations(),
        standard.light_time(),
    )
    .unwrap();
    let options =
        SolarEclipseSearchOptions::new(angular, SolarEclipseSearchOptions::standard().model());

    let ephemeris = SofaAnalyticEphemeris::new();
    let eclipses = Events::new(Astrometry::new(&time, &ephemeris))
        .local_solar_eclipses_in(&site, TimeInterval::new(start, end).unwrap(), options)
        .unwrap();

    assert_eq!(eclipses.len(), 1);
    assert_eq!(eclipses[0].kind(), LocalSolarEclipseKind::Total);
    let provenance = eclipses[0].earth_attitude_provenance();
    assert!(provenance.is_predicted());
    assert_eq!(
        provenance.delta_t_model(),
        Some("NASA 2035 path-table Delta T 80.6 s")
    );
    assert_eq!(
        provenance.delta_t_disposition(),
        Some(PredictionDisposition::Assumed)
    );
    assert_eq!(
        provenance.offset_model(),
        Some("zero polar motion and celestial-pole offsets")
    );
    assert_eq!(
        provenance.offset_disposition(),
        Some(PredictionDisposition::Assumed)
    );
}
