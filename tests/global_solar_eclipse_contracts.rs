use hyastro::{
    astro::Astrometry,
    earth::Earth,
    ephem::SofaAnalyticEphemeris,
    event::{
        AngularEventSearchOptions, BesselianPolynomialOptions, CentralSolarEclipseCharacter,
        Events, GlobalSolarEclipseKind, SolarEclipseSearchOptions,
    },
    math::Angle,
    time::{DateTime, Gregorian, TimeContext, TimeInterval, Tt},
};

#[cfg(feature = "anise")]
use hyastro::ephem::{Ephemeris, KernelManifest};
#[cfg(feature = "anise")]
use hyastro::time::Utc;

#[test]
fn global_shadow_geometry_distinguishes_partial_annular_total_and_hybrid_eclipses() {
    let time = TimeContext::builtin();
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let standard = AngularEventSearchOptions::standard();
    let search = AngularEventSearchOptions::new(
        standard.scan_step(),
        standard.time_tolerance(),
        Angle::from_radians(1.0e-10).unwrap(),
        standard.max_refinement_iterations(),
        standard.max_evaluations(),
        standard.light_time(),
    )
    .unwrap();
    let options =
        SolarEclipseSearchOptions::new(search, SolarEclipseSearchOptions::standard().model());
    let scenarios = [
        (
            (2023, 4, 20),
            (2023, 4, 21),
            (4, 17, 56),
            GlobalSolarEclipseKind::Hybrid,
            -0.3952,
        ),
        (
            (2023, 10, 14),
            (2023, 10, 15),
            (18, 0, 41),
            GlobalSolarEclipseKind::Annular,
            0.3753,
        ),
        (
            (2024, 4, 8),
            (2024, 4, 9),
            (18, 18, 29),
            GlobalSolarEclipseKind::Total,
            0.3431,
        ),
        (
            (2025, 3, 29),
            (2025, 3, 30),
            (10, 48, 36),
            GlobalSolarEclipseKind::Partial,
            1.0405,
        ),
        (
            (2043, 4, 9),
            (2043, 4, 10),
            (18, 57, 49),
            GlobalSolarEclipseKind::Total,
            1.0031,
        ),
        (
            (2043, 10, 3),
            (2043, 10, 4),
            (3, 1, 49),
            GlobalSolarEclipseKind::Annular,
            -1.0102,
        ),
    ];

    for (start_date, end_date, greatest_clock, expected_kind, expected_gamma) in scenarios {
        let start = time
            .resolve(
                DateTime::<Gregorian, Tt>::from_components(
                    start_date.0,
                    start_date.1,
                    start_date.2,
                    0,
                    0,
                    0,
                    0,
                )
                .unwrap(),
            )
            .unwrap();
        let end = time
            .resolve(
                DateTime::<Gregorian, Tt>::from_components(
                    end_date.0, end_date.1, end_date.2, 0, 0, 0, 0,
                )
                .unwrap(),
            )
            .unwrap();
        let eclipses = events
            .global_solar_eclipses_in(
                &Earth::wgs84(),
                TimeInterval::new(start, end).unwrap(),
                options,
            )
            .unwrap();
        assert_eq!(eclipses.len(), 1, "{start_date:?}");
        let eclipse = &eclipses[0];
        assert_eq!(eclipse.kind(), expected_kind, "{start_date:?}");
        assert!(
            (eclipse.maximum().gamma().as_equatorial_radii() - expected_gamma).abs() < 0.002,
            "{start_date:?}: gamma {} vs NASA {expected_gamma}",
            eclipse.maximum().gamma().as_equatorial_radii(),
        );
        let expected_greatest = time
            .resolve(
                DateTime::<Gregorian, Tt>::from_components(
                    start_date.0,
                    start_date.1,
                    start_date.2,
                    greatest_clock.0,
                    greatest_clock.1,
                    greatest_clock.2,
                    0,
                )
                .unwrap(),
            )
            .unwrap();
        let greatest_error_nanoseconds = eclipse
            .maximum()
            .instant()
            .tai_nanoseconds_since_1900()
            .abs_diff(expected_greatest.tai_nanoseconds_since_1900());
        assert!(
            greatest_error_nanoseconds < 30_000_000_000,
            "{start_date:?}: greatest-eclipse error {greatest_error_nanoseconds} ns",
        );

        match expected_kind {
            GlobalSolarEclipseKind::Hybrid => {
                let path = eclipse.central_path().unwrap();
                assert_eq!(
                    path.start_character(),
                    CentralSolarEclipseCharacter::Annular
                );
                assert_eq!(
                    path.greatest_character(),
                    CentralSolarEclipseCharacter::Total
                );
                assert_eq!(path.end_character(), CentralSolarEclipseCharacter::Annular);
                assert_eq!(path.transitions().len(), 2);
                assert!(path.transitions().iter().all(|transition| {
                    transition.shadow_radius().as_metres().abs() < 1.0
                        && transition.time_uncertainty().as_seconds_f64() <= 0.001
                }));
            }
            GlobalSolarEclipseKind::Annular => {
                if eclipse.is_non_central() {
                    assert!(eclipse.central_path().is_none());
                    assert!(eclipse.maximum().antumbra_intersects_earth());
                } else {
                    let path = eclipse.central_path().unwrap();
                    assert_eq!(
                        path.greatest_character(),
                        CentralSolarEclipseCharacter::Annular
                    );
                    assert!(path.transitions().is_empty());
                }
            }
            GlobalSolarEclipseKind::Total => {
                if eclipse.is_non_central() {
                    assert!(eclipse.central_path().is_none());
                    assert!(eclipse.maximum().umbra_intersects_earth());
                } else {
                    let path = eclipse.central_path().unwrap();
                    assert_eq!(
                        path.greatest_character(),
                        CentralSolarEclipseCharacter::Total
                    );
                    assert!(path.transitions().is_empty());
                }
            }
            GlobalSolarEclipseKind::Partial => {
                assert!(eclipse.central_path().is_none());
                assert!(!eclipse.maximum().umbra_intersects_earth());
                assert!(!eclipse.maximum().antumbra_intersects_earth());
            }
            _ => unreachable!("test scenarios use the four stable eclipse kinds"),
        }
    }
}

#[test]
fn geometric_and_besselian_contact_core_radii_are_explicit() {
    let time = TimeContext::builtin();
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let standard = AngularEventSearchOptions::standard();
    let search = AngularEventSearchOptions::new(
        standard.scan_step(),
        standard.time_tolerance(),
        Angle::from_radians(1.0e-10).unwrap(),
        standard.max_refinement_iterations(),
        standard.max_evaluations(),
        standard.light_time(),
    )
    .unwrap();
    let options =
        SolarEclipseSearchOptions::new(search, SolarEclipseSearchOptions::standard().model());
    let start = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2026, 2, 17, 0, 0, 0, 0).unwrap())
        .unwrap();
    let end = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2026, 2, 18, 0, 0, 0, 0).unwrap())
        .unwrap();
    let eclipse = events
        .global_solar_eclipses_in(
            &Earth::wgs84(),
            TimeInterval::new(start, end).unwrap(),
            options,
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let maximum = eclipse.maximum();
    let polynomial = events
        .solar_eclipse_besselian_polynomial(
            &Earth::wgs84(),
            maximum.instant(),
            BesselianPolynomialOptions::nasa_six_hour(),
        )
        .unwrap();
    let elements = polynomial.elements_at(maximum.instant()).unwrap();
    let expected_contact_core_metres = -elements.l2().as_equatorial_radii()
        * Earth::wgs84()
            .reference_ellipsoid()
            .semi_major_axis()
            .as_metres();
    let geometric_core = maximum.geometric_core_shadow_radius_at_axis_plane();
    let contact_core = elements.contact_core_shadow_radius_at_fundamental_plane();

    assert!(
        (contact_core.as_metres() - expected_contact_core_metres).abs() < 1.0e-9,
        "contact core must equal -l2*a"
    );
    assert!(
        (geometric_core.as_metres() - contact_core.as_metres()).abs() > 1_000.0,
        "the physical and NASA contact conventions must remain distinguishable"
    );
    assert!(geometric_core.is_antumbral());
    assert!(contact_core.is_antumbral());
}

// NASA Five Millennium Catalog, 2001-2100:
// https://eclipse.gsfc.nasa.gov/SEcat5/SE2001-2100.html
#[cfg(feature = "anise")]
#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-series BSP"]
fn de440_global_kinds_gamma_and_greatest_instants_match_nasa_catalog() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let standard = AngularEventSearchOptions::standard();
    let search = AngularEventSearchOptions::new(
        standard.scan_step(),
        standard.time_tolerance(),
        Angle::from_radians(1.0e-10).unwrap(),
        standard.max_refinement_iterations(),
        standard.max_evaluations(),
        standard.light_time(),
    )
    .unwrap();
    let options =
        SolarEclipseSearchOptions::new(search, SolarEclipseSearchOptions::standard().model());
    let scenarios = [
        (
            (2023, 4, 20),
            (2023, 4, 21),
            (4, 16, 47),
            GlobalSolarEclipseKind::Hybrid,
            -0.3952,
        ),
        (
            (2023, 10, 14),
            (2023, 10, 15),
            (17, 59, 32),
            GlobalSolarEclipseKind::Annular,
            0.3753,
        ),
        (
            (2024, 4, 8),
            (2024, 4, 9),
            (18, 17, 20),
            GlobalSolarEclipseKind::Total,
            0.3431,
        ),
        (
            (2025, 3, 29),
            (2025, 3, 30),
            (10, 47, 27),
            GlobalSolarEclipseKind::Partial,
            1.0405,
        ),
    ];

    for (start_date, end_date, greatest_clock, expected_kind, expected_gamma) in scenarios {
        let start = time
            .resolve(
                DateTime::<Gregorian, Utc>::from_components(
                    start_date.0,
                    start_date.1,
                    start_date.2,
                    0,
                    0,
                    0,
                    0,
                )
                .unwrap(),
            )
            .unwrap();
        let end = time
            .resolve(
                DateTime::<Gregorian, Utc>::from_components(
                    end_date.0, end_date.1, end_date.2, 0, 0, 0, 0,
                )
                .unwrap(),
            )
            .unwrap();
        let eclipse = Events::new(Astrometry::new(&time, &ephemeris))
            .global_solar_eclipses_in(
                &Earth::wgs84(),
                TimeInterval::new(start, end).unwrap(),
                options,
            )
            .unwrap()
            .into_iter()
            .next()
            .expect("NASA catalog eclipse must be found");
        assert_eq!(eclipse.kind(), expected_kind, "{start_date:?}");
        assert!(
            (eclipse.maximum().gamma().as_equatorial_radii() - expected_gamma).abs() < 0.0005,
            "{start_date:?}: gamma {} vs NASA {expected_gamma}",
            eclipse.maximum().gamma().as_equatorial_radii(),
        );
        let expected_greatest = time
            .resolve(
                DateTime::<Gregorian, Utc>::from_components(
                    start_date.0,
                    start_date.1,
                    start_date.2,
                    greatest_clock.0,
                    greatest_clock.1,
                    greatest_clock.2,
                    0,
                )
                .unwrap(),
            )
            .unwrap();
        let greatest_error_nanoseconds = eclipse
            .maximum()
            .instant()
            .tai_nanoseconds_since_1900()
            .abs_diff(expected_greatest.tai_nanoseconds_since_1900());
        assert!(
            greatest_error_nanoseconds < 10_000_000_000,
            "{start_date:?}: greatest-eclipse error {greatest_error_nanoseconds} ns",
        );
    }
}
