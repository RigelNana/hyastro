use approx::assert_abs_diff_eq;
use hyastro::{
    astro::Astrometry,
    earth::Earth,
    ephem::SofaAnalyticEphemeris,
    event::{
        AngularEventSearchOptions, BesselianPolynomialOptions, Events,
        GlobalSolarEclipsePathOptions, SolarEclipseSearchOptions,
    },
    math::Angle,
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, EarthRotationTable,
        Gregorian, IersC04, IersFinals2000A, ModifiedJulianDate, TimeContext, TimeInterval, Tt,
        Utc,
    },
};

#[cfg(feature = "anise")]
use hyastro::ephem::{Ephemeris, KernelManifest};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");
const FINALS_2000_A: &str = include_str!("../data/eop/finals2000a-2026-08-09.all");

fn search_options() -> SolarEclipseSearchOptions {
    let standard = AngularEventSearchOptions::standard();
    let angular = AngularEventSearchOptions::new(
        standard.scan_step(),
        standard.time_tolerance(),
        Angle::from_radians(1.0e-10).unwrap(),
        standard.max_refinement_iterations(),
        standard.max_evaluations(),
        standard.light_time(),
    )
    .unwrap();
    SolarEclipseSearchOptions::new(angular, SolarEclipseSearchOptions::standard().model())
}

fn interval(time: &TimeContext<'_, EarthOrientationTable<'_>>) -> TimeInterval<Tt> {
    let start = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 4, 8, 0, 0, 0, 0).unwrap())
        .unwrap();
    let end = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 4, 9, 0, 0, 0, 0).unwrap())
        .unwrap();
    TimeInterval::new(start, end).unwrap()
}

#[test]
fn analytic_path_retains_centre_limits_width_duration_and_sun_direction() {
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
    let eop = EarthOrientationTable::new(&samples, "C04 solar-eclipse path test", expires).unwrap();
    let time = base.with_earth_orientation(eop);
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let earth = Earth::wgs84();
    let eclipses = events
        .global_solar_eclipses_in(&earth, interval(&time), search_options())
        .unwrap();
    let eclipse = &eclipses[0];
    let reference_epoch = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 4, 8, 18, 0, 0, 0).unwrap())
        .unwrap();
    let polynomial = events
        .solar_eclipse_besselian_polynomial(
            &earth,
            reference_epoch,
            hyastro::event::BesselianPolynomialOptions::nasa_six_hour(),
        )
        .unwrap();
    let delta_t = time.delta_t_at(reference_epoch).unwrap();
    let path = events
        .solar_eclipse_path(
            eclipse,
            &polynomial,
            delta_t,
            GlobalSolarEclipsePathOptions::standard(),
        )
        .unwrap();

    assert_eq!(path.earth(), earth);
    assert_eq!(path.delta_t(), delta_t);
    assert_eq!(path.limb_model(), polynomial.limb_model());
    assert!(path.points().len() > 80);
    assert!(
        path.points()
            .windows(2)
            .all(|pair| pair[0].instant() < pair[1].instant())
    );

    let greatest = path
        .points()
        .iter()
        .copied()
        .find(|point| point.instant() == eclipse.maximum().instant())
        .unwrap();
    let centre = greatest.centre_line();
    assert_abs_diff_eq!(centre.latitude().as_degrees(), 25.3, epsilon = 0.3);
    assert_abs_diff_eq!(centre.longitude().as_degrees(), -104.1, epsilon = 0.4);
    assert!(greatest.northern_limit().latitude() > centre.latitude());
    assert!(greatest.southern_limit().latitude() < centre.latitude());
    assert_abs_diff_eq!(
        greatest.boundary_geodesic_span().as_kilometres(),
        198.0,
        epsilon = 15.0
    );
    assert_abs_diff_eq!(greatest.path_width().as_kilometres(), 198.0, epsilon = 15.0);
    assert_abs_diff_eq!(
        greatest.central_duration().as_seconds_f64(),
        268.0,
        epsilon = 12.0
    );
    assert!(greatest.central_phase().second_contact() < greatest.instant());
    assert!(greatest.central_phase().third_contact() > greatest.instant());
    assert_abs_diff_eq!(
        greatest.sun_direction().altitude().as_degrees(),
        69.8,
        epsilon = 1.0
    );
    assert_abs_diff_eq!(
        greatest.sun_direction().azimuth().unwrap().as_degrees(),
        149.4,
        epsilon = 2.0
    );
}

// NASA GSFC path table for the 2024-04-08 total eclipse:
// https://eclipse.gsfc.nasa.gov/SEpath/SEpath2001/SE2024Apr08Tpath.html
#[cfg(feature = "anise")]
#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-series BSP"]
fn de440_greatest_path_circumstances_match_nasa() {
    let kernel = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel]).unwrap()).unwrap();
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
    let eop = EarthOrientationTable::new(&samples, "C04 solar-eclipse path test", expires).unwrap();
    let time = base.with_earth_orientation(eop);
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let earth = Earth::wgs84();
    let eclipses = events
        .global_solar_eclipses_in(&earth, interval(&time), search_options())
        .unwrap();
    let eclipse = &eclipses[0];
    let reference_epoch = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 4, 8, 18, 0, 0, 0).unwrap())
        .unwrap();
    let polynomial = events
        .solar_eclipse_besselian_polynomial(
            &earth,
            reference_epoch,
            hyastro::event::BesselianPolynomialOptions::nasa_six_hour(),
        )
        .unwrap();
    let path = events
        .solar_eclipse_path(
            eclipse,
            &polynomial,
            time.delta_t_at(reference_epoch).unwrap(),
            GlobalSolarEclipsePathOptions::standard(),
        )
        .unwrap();
    let greatest = path
        .points()
        .iter()
        .copied()
        .find(|point| point.instant() == eclipse.maximum().instant())
        .unwrap();

    assert_abs_diff_eq!(
        greatest.centre_line().latitude().as_degrees(),
        25.2867,
        epsilon = 0.05
    );
    assert_abs_diff_eq!(
        greatest.centre_line().longitude().as_degrees(),
        -104.1383,
        epsilon = 0.05
    );
    assert_abs_diff_eq!(
        greatest.boundary_geodesic_span().as_kilometres(),
        197.5,
        epsilon = 3.0
    );
    assert_abs_diff_eq!(greatest.path_width().as_kilometres(), 197.5, epsilon = 3.0);
    assert_abs_diff_eq!(
        greatest.central_duration().as_seconds_f64(),
        268.1,
        epsilon = 3.0
    );
    assert_abs_diff_eq!(
        greatest.sun_direction().altitude().as_degrees(),
        69.8,
        epsilon = 0.2
    );
    assert_abs_diff_eq!(
        greatest.sun_direction().azimuth().unwrap().as_degrees(),
        149.4,
        epsilon = 0.5
    );
}

// NASA GSFC central-path tables:
// https://eclipse.gsfc.nasa.gov/SEpath/SEpath2001/SE2026Feb17Apath.html
// https://eclipse.gsfc.nasa.gov/SEpath/SEpath2001/SE2026Aug12Tpath.html
#[cfg(feature = "anise")]
#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-series BSP"]
fn de440_2026_path_widths_and_contact_cores_match_nasa_conventions() {
    let kernel = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel]).unwrap()).unwrap();
    let base = TimeContext::builtin();
    let data = IersFinals2000A::parse(FINALS_2000_A).unwrap();
    let samples = data
        .try_earth_rotation_samples_in(
            &base,
            ModifiedJulianDate::<Utc>::from_parts(61_040.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(61_407.0, 0.0).unwrap(),
            EarthOrientationAcceptance::IncludePredicted,
        )
        .unwrap();
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let rotation =
        EarthRotationTable::new(&samples, "IERS finals2000A snapshot 2026-08-09", expires).unwrap();
    let time = base.with_earth_rotation(rotation);
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let earth = Earth::wgs84();
    let start = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2026, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let end = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2027, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let eclipses = events
        .global_solar_eclipses_in(
            &earth,
            TimeInterval::new(start, end).unwrap(),
            search_options(),
        )
        .unwrap();
    assert_eq!(eclipses.len(), 2);

    let expected = [(616.6, 650.2, -73.37), (294.0, 297.3, 52.03)];
    for (eclipse, (expected_width, expected_span, expected_contact_core)) in
        eclipses.iter().zip(expected)
    {
        let maximum = eclipse.maximum();
        let polynomial = events
            .solar_eclipse_besselian_polynomial(
                &earth,
                maximum.instant(),
                BesselianPolynomialOptions::nasa_six_hour(),
            )
            .unwrap();
        let elements = polynomial.elements_at(maximum.instant()).unwrap();
        let path = events
            .solar_eclipse_path(
                eclipse,
                &polynomial,
                time.delta_t_at(maximum.instant()).unwrap(),
                GlobalSolarEclipsePathOptions::standard(),
            )
            .unwrap();
        let greatest = path
            .points()
            .iter()
            .copied()
            .find(|point| point.instant() == maximum.instant())
            .unwrap();

        assert_abs_diff_eq!(
            greatest.path_width().as_kilometres(),
            expected_width,
            epsilon = 1.0
        );
        assert_abs_diff_eq!(
            greatest.boundary_geodesic_span().as_kilometres(),
            expected_span,
            epsilon = 0.2
        );
        assert_abs_diff_eq!(
            elements
                .contact_core_shadow_radius_at_fundamental_plane()
                .as_metres()
                / 1_000.0,
            expected_contact_core,
            epsilon = 0.05
        );
    }
}
