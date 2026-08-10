use approx::assert_abs_diff_eq;
#[cfg(feature = "anise")]
use hyastro::ephem::{Ephemeris, KernelManifest};
use hyastro::{
    astro::Astrometry,
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::SofaAnalyticEphemeris,
    event::{
        Events, HorizonEventKind, LunarEclipseKind, LunarEclipseSearchOptions,
        LunarEclipseSkyBackground, LunarEclipseVisibilityOptions, LunarEclipseVisibilityStage,
        LunarShadowConvention,
    },
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, IersC04,
        ModifiedJulianDate, TimeContext, TimeInterval, Tt, Utc,
    },
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
#[test]
fn analytic_2022_total_lunar_eclipse_retains_complete_global_circumstances()
-> Result<(), Box<dyn std::error::Error>> {
    let time = TimeContext::builtin();
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let start = time.resolve(DateTime::<Gregorian, Tt>::from_components(
        2022, 11, 8, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Tt>::from_components(
        2022, 11, 9, 0, 0, 0, 0,
    )?)?;

    let eclipses = events.global_lunar_eclipses_in(
        &Earth::wgs84(),
        TimeInterval::new(start, end)?,
        LunarEclipseSearchOptions::standard(),
    )?;

    assert_eq!(eclipses.len(), 1);
    let eclipse = &eclipses[0];
    assert_eq!(eclipse.kind(), LunarEclipseKind::Total);
    assert_eq!(eclipse.model().shadow(), LunarShadowConvention::DANJON);
    assert_eq!(eclipse.earth(), Earth::wgs84());
    assert!(eclipse.umbral_ingress().is_some());
    assert!(eclipse.totality_ingress().is_some());
    assert!(eclipse.totality_egress().is_some());
    assert!(eclipse.umbral_egress().is_some());
    assert!(eclipse.penumbral_ingress().instant() < eclipse.maximum().instant());
    assert!(eclipse.maximum().instant() < eclipse.penumbral_egress().instant());
    assert_abs_diff_eq!(
        eclipse.maximum().geometry().umbral_magnitude().as_ratio(),
        1.3589,
        epsilon = 0.03
    );
    assert_abs_diff_eq!(
        eclipse
            .maximum()
            .geometry()
            .penumbral_magnitude()
            .as_ratio(),
        2.4143,
        epsilon = 0.04
    );
    assert_abs_diff_eq!(
        eclipse
            .total_phase()
            .unwrap()
            .unwrap()
            .duration()?
            .as_seconds_f64(),
        85.0 * 60.0,
        epsilon = 180.0
    );

    Ok::<(), Box<dyn std::error::Error>>(())
}

#[test]
fn dallas_visibility_retains_moonset_clipping_and_low_altitude_samples()
-> Result<(), Box<dyn std::error::Error>> {
    let base = TimeContext::builtin();
    let data = IersC04::parse(C04)?;
    let samples = data.try_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(59_890.0, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(59_893.0, 0.0)?,
        EarthOrientationAcceptance::FinalOnly,
    )?;
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let eop = EarthOrientationTable::new(&samples, "C04 2022 lunar eclipse", expires)?;
    let time = base.with_earth_orientation(eop);
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let start = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2022, 11, 8, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2022, 11, 9, 0, 0, 0, 0,
    )?)?;
    let eclipses = events.global_lunar_eclipses_in(
        &Earth::wgs84(),
        TimeInterval::new(start, end)?,
        LunarEclipseSearchOptions::standard(),
    )?;
    let eclipse = &eclipses[0];

    let visibility = events.local_lunar_eclipse_visibility(
        &dallas_site(),
        eclipse,
        LunarEclipseVisibilityOptions::standard(eclipse.model()),
    )?;

    assert_eq!(visibility.samples().len(), 7);
    assert_eq!(
        visibility.samples()[3].stage(),
        LunarEclipseVisibilityStage::Greatest
    );
    assert!(
        visibility
            .horizon_events()
            .events()
            .iter()
            .any(|event| event.kind() == HorizonEventKind::Set)
    );
    assert!(
        visibility
            .visible_phases()
            .iter()
            .any(|phase| phase.kind() == LunarEclipseKind::Total)
    );
    assert!(
        visibility
            .visible_phases()
            .iter()
            .any(|phase| phase.is_truncated_at_end())
    );
    assert!(visibility.has_low_altitude_warning());
    for sample in visibility.samples() {
        let altitude = sample.solar_altitude().as_degrees();
        match sample.sky_background() {
            LunarEclipseSkyBackground::Daylight => assert!(altitude >= 0.0),
            LunarEclipseSkyBackground::CivilTwilight => {
                assert!((-6.0..0.0).contains(&altitude))
            }
            LunarEclipseSkyBackground::NauticalTwilight => {
                assert!((-12.0..-6.0).contains(&altitude))
            }
            LunarEclipseSkyBackground::AstronomicalTwilight => {
                assert!((-18.0..-12.0).contains(&altitude))
            }
            LunarEclipseSkyBackground::Night => assert!(altitude < -18.0),
            _ => unreachable!("unexpected sky-background extension"),
        }
    }
    let mismatched_site = Earth::grs80().fixed_site(
        "Dallas on GRS 80",
        GeodeticPosition::new(
            GeodeticLongitude::try_from_degrees(-96.7970)?,
            GeodeticLatitude::try_from_degrees(32.7767)?,
            EllipsoidalHeight::from_metres(131.0)?,
        ),
    )?;
    assert!(matches!(
        events.local_lunar_eclipse_visibility(
            &mismatched_site,
            eclipse,
            LunarEclipseVisibilityOptions::standard(eclipse.model()),
        ),
        Err(hyastro::event::Error::LunarEclipseVisibilityEarthMismatch)
    ));
    assert_eq!(
        visibility.earth_orientation_version(),
        "C04 2022 lunar eclipse"
    );

    Ok(())
}

#[test]
fn analytic_2023_search_distinguishes_penumbral_and_partial_eclipses()
-> Result<(), Box<dyn std::error::Error>> {
    let time = TimeContext::builtin();
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let start = time.resolve(DateTime::<Gregorian, Tt>::from_components(
        2023, 1, 1, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Tt>::from_components(
        2024, 1, 1, 0, 0, 0, 0,
    )?)?;
    let eclipses = events.global_lunar_eclipses_in(
        &Earth::wgs84(),
        TimeInterval::new(start, end)?,
        LunarEclipseSearchOptions::standard(),
    )?;

    assert_eq!(eclipses.len(), 2);
    assert_eq!(eclipses[0].kind(), LunarEclipseKind::Penumbral);
    assert!(eclipses[0].umbral_ingress().is_none());
    assert_eq!(eclipses[1].kind(), LunarEclipseKind::Partial);
    assert!(eclipses[1].umbral_ingress().is_some());
    assert!(eclipses[1].totality_ingress().is_none());

    Ok(())
}

#[test]
fn analytic_catalog_examples_distinguish_2024_penumbral_and_2025_total()
-> Result<(), Box<dyn std::error::Error>> {
    let time = TimeContext::builtin();
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let scenarios = [
        ((2024, 3, 25), (2024, 3, 26), LunarEclipseKind::Penumbral),
        ((2025, 3, 14), (2025, 3, 15), LunarEclipseKind::Total),
    ];

    for ((start_year, start_month, start_day), (end_year, end_month, end_day), expected_kind) in
        scenarios
    {
        let start = time.resolve(DateTime::<Gregorian, Tt>::from_components(
            start_year,
            start_month,
            start_day,
            0,
            0,
            0,
            0,
        )?)?;
        let end = time.resolve(DateTime::<Gregorian, Tt>::from_components(
            end_year, end_month, end_day, 0, 0, 0, 0,
        )?)?;
        let eclipses = events.global_lunar_eclipses_in(
            &Earth::wgs84(),
            TimeInterval::new(start, end)?,
            LunarEclipseSearchOptions::standard(),
        )?;

        assert_eq!(eclipses.len(), 1);
        let eclipse = &eclipses[0];
        assert_eq!(eclipse.kind(), expected_kind);
        assert_eq!(
            eclipse.umbral_ingress().is_some(),
            expected_kind != LunarEclipseKind::Penumbral
        );
        assert_eq!(
            eclipse.totality_ingress().is_some(),
            expected_kind == LunarEclipseKind::Total
        );
    }

    Ok(())
}

#[test]
#[cfg(feature = "anise")]
#[ignore = "requires HYASTRO_DE440S to name a local de440-series BSP"]
fn de440_2022_contacts_and_magnitudes_match_nasa_catalog() -> Result<(), Box<dyn std::error::Error>>
{
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path])?)?;
    let time = TimeContext::builtin();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let start = time.resolve(DateTime::<Gregorian, Tt>::from_components(
        2022, 11, 8, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Tt>::from_components(
        2022, 11, 9, 0, 0, 0, 0,
    )?)?;
    let eclipses = events.global_lunar_eclipses_in(
        &Earth::wgs84(),
        TimeInterval::new(start, end)?,
        LunarEclipseSearchOptions::standard(),
    )?;
    let eclipse = &eclipses[0];
    let expected = [
        (
            eclipse.penumbral_ingress().instant(),
            (8, 3, 2, 300_000_000),
        ),
        (
            eclipse.umbral_ingress().unwrap().instant(),
            (9, 9, 59, 300_000_000),
        ),
        (
            eclipse.totality_ingress().unwrap().instant(),
            (10, 17, 22, 800_000_000),
        ),
        (eclipse.maximum().instant(), (11, 0, 22, 0)),
        (
            eclipse.totality_egress().unwrap().instant(),
            (11, 43, 2, 300_000_000),
        ),
        (
            eclipse.umbral_egress().unwrap().instant(),
            (12, 50, 34, 200_000_000),
        ),
        (
            eclipse.penumbral_egress().instant(),
            (13, 57, 42, 900_000_000),
        ),
    ];
    for (actual, (hour, minute, second, nanosecond)) in expected {
        let reference = time.resolve(DateTime::<Gregorian, Tt>::from_components(
            2022, 11, 8, hour, minute, second, nanosecond,
        )?)?;
        let error = actual
            .duration_since(reference)?
            .checked_abs()?
            .as_seconds_f64();
        assert!(error < 180.0, "contact timing error {error:.3} s");
    }
    let position_angles = [
        (
            eclipse
                .penumbral_ingress()
                .geometry()
                .position_angle()
                .unwrap(),
            256.9,
        ),
        (
            eclipse
                .umbral_ingress()
                .unwrap()
                .geometry()
                .position_angle()
                .unwrap(),
            262.4,
        ),
        (
            eclipse
                .totality_ingress()
                .unwrap()
                .geometry()
                .position_angle()
                .unwrap(),
            281.8,
        ),
        (
            eclipse.maximum().geometry().position_angle().unwrap(),
            337.5,
        ),
        (
            eclipse
                .totality_egress()
                .unwrap()
                .geometry()
                .position_angle()
                .unwrap(),
            33.0,
        ),
        (
            eclipse
                .umbral_egress()
                .unwrap()
                .geometry()
                .position_angle()
                .unwrap(),
            52.6,
        ),
        (
            eclipse
                .penumbral_egress()
                .geometry()
                .position_angle()
                .unwrap(),
            58.2,
        ),
    ];
    for (actual, expected) in position_angles {
        assert_abs_diff_eq!(actual.as_degrees(), expected, epsilon = 0.6);
    }
    assert_abs_diff_eq!(
        eclipse.maximum().geometry().umbral_magnitude().as_ratio(),
        1.3589,
        epsilon = 0.015
    );
    assert_abs_diff_eq!(
        eclipse
            .maximum()
            .geometry()
            .penumbral_magnitude()
            .as_ratio(),
        2.4143,
        epsilon = 0.03
    );

    Ok(())
}
