#![cfg(feature = "anise")]

use hyastro::{
    astro::Astrometry,
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{CelestialBody, Ephemeris, KernelManifest},
    event::{
        AngularEventSearchOptions, AngularSeparationExtremumQuery, AstrometricMode,
        ConfigurationCoordinate, ConfigurationKind, ConfigurationQuery, CoordinateCrossingKind,
        CoordinateCrossingQuery, CoordinateExtremumQuery, DistanceExtremumQuery, ElongationSide,
        EventCoordinate, Events, ExtremumKind, ExtremumSearchOptions, RelativeBodyQuery,
        SolarConjunctionKind, StationKind, StationQuery,
    },
    math::Angle,
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian,
        IersFinals2000A, JulianDate, TimeContext, TimeInterval, Utc,
    },
};

fn utc<E>(
    time: &TimeContext<'_, E>,
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
) -> hyastro::time::Instant<Utc> {
    time.resolve(
        DateTime::<Gregorian, Utc>::from_components(year, month, day, hour, minute, 0, 0).unwrap(),
    )
    .unwrap()
}

#[test]
fn relative_queries_reject_identical_bodies_and_preserve_semantics() {
    assert!(
        RelativeBodyQuery::new(
            CelestialBody::Mars,
            CelestialBody::Mars,
            AstrometricMode::Apparent,
        )
        .is_err()
    );
    assert!(
        DistanceExtremumQuery::new(
            CelestialBody::Earth,
            CelestialBody::Earth,
            ExtremumKind::Minimum,
        )
        .is_err()
    );
    let bodies = RelativeBodyQuery::new(
        CelestialBody::Mercury,
        CelestialBody::Sun,
        AstrometricMode::Geometric,
    )
    .unwrap();
    let query = ConfigurationQuery::new(
        bodies,
        ConfigurationKind::WesternQuadrature,
        ConfigurationCoordinate::RightAscension,
    );
    assert_eq!(query.bodies(), bodies);
    assert_eq!(query.kind().target_angle().as_degrees(), 270.0);
    assert_eq!(query.coordinate(), ConfigurationCoordinate::RightAscension);
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-family BSP"]
fn de440_planetary_configurations_and_extrema_match_horizons_and_physical_counts() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let interval =
        TimeInterval::new(utc(&time, 2024, 1, 1, 0, 0), utc(&time, 2025, 1, 1, 0, 0)).unwrap();
    let angular = AngularEventSearchOptions::standard();
    let extrema = ExtremumSearchOptions::standard();
    let mercury_sun = RelativeBodyQuery::new(
        CelestialBody::Mercury,
        CelestialBody::Sun,
        AstrometricMode::Apparent,
    )
    .unwrap();

    let conjunctions = events
        .configurations_in(
            interval,
            ConfigurationQuery::new(
                mercury_sun,
                ConfigurationKind::Conjunction,
                ConfigurationCoordinate::EclipticLongitude,
            ),
            angular,
        )
        .unwrap();
    assert_eq!(conjunctions.len(), 6);
    assert!(conjunctions.iter().all(|event| {
        event.evidence().residual().as_radians().abs() <= angular.angular_tolerance().as_radians()
            && event.solar_conjunction_kind().is_some()
    }));
    assert!(
        conjunctions
            .iter()
            .any(|event| event.solar_conjunction_kind() == Some(SolarConjunctionKind::Inferior))
    );
    assert!(
        conjunctions
            .iter()
            .any(|event| event.solar_conjunction_kind() == Some(SolarConjunctionKind::Superior))
    );

    let elongations = events
        .greatest_elongations_in(interval, mercury_sun, extrema)
        .unwrap();
    assert_eq!(elongations.len(), 7);
    assert_eq!(elongations[0].side(), ElongationSide::Western);
    assert_eq!(elongations[1].side(), ElongationSide::Eastern);
    let horizons_epoch = utc(&time, 2024, 3, 24, 22, 34);
    assert!(
        elongations[1]
            .instant()
            .duration_since(horizons_epoch)
            .unwrap()
            .checked_abs()
            .unwrap()
            < Duration::from_seconds(120).unwrap()
    );
    assert!((elongations[1].separation().as_degrees() - 18.7016).abs() < 5.0e-5);
    assert!(elongations.iter().all(
        |event| event.evidence().time_uncertainty() <= Duration::from_milliseconds(2).unwrap()
    ));

    let separation_maxima = events
        .angular_separation_extrema_in(
            interval,
            AngularSeparationExtremumQuery::new(mercury_sun, ExtremumKind::Maximum),
            extrema,
        )
        .unwrap();
    assert_eq!(separation_maxima.len(), elongations.len());
    for (separation, elongation) in separation_maxima.iter().zip(&elongations) {
        assert!(
            separation
                .instant()
                .duration_since(elongation.instant())
                .unwrap()
                .checked_abs()
                .unwrap()
                <= Duration::from_milliseconds(2).unwrap()
        );
    }

    let stations = events
        .stations_in(
            interval,
            StationQuery::new(CelestialBody::Mercury, AstrometricMode::Apparent),
            angular,
        )
        .unwrap();
    assert_eq!(stations.len(), 7);
    assert_eq!(stations[0].kind(), StationKind::Direct);
    for pair in stations.windows(2) {
        assert_ne!(pair[0].kind(), pair[1].kind());
    }
    assert!(stations.iter().all(|event| {
        event
            .evidence()
            .residual_rate()
            .as_radians_per_second()
            .abs()
            < 1.0e-13
    }));

    let lunar_perigees = events
        .distance_extrema_in(
            interval,
            DistanceExtremumQuery::new(
                CelestialBody::Moon,
                CelestialBody::Earth,
                ExtremumKind::Minimum,
            )
            .unwrap(),
            extrema,
        )
        .unwrap();
    assert_eq!(lunar_perigees.len(), 13);
    assert!(lunar_perigees.iter().all(|event| {
        (350_000.0..380_000.0).contains(&event.distance().as_kilometres())
            && event.state().target() == CelestialBody::Moon
            && event.state().center() == CelestialBody::Earth
    }));

    let nodes = events
        .coordinate_crossings_in(
            interval,
            CoordinateCrossingQuery::new(
                CelestialBody::Moon,
                AstrometricMode::Apparent,
                EventCoordinate::EclipticLatitude,
            ),
            angular,
        )
        .unwrap();
    assert_eq!(nodes.len(), 27);
    for pair in nodes.windows(2) {
        assert_ne!(pair[0].kind(), pair[1].kind());
    }
    assert!(
        nodes
            .iter()
            .any(|event| event.kind() == CoordinateCrossingKind::Ascending)
    );
    assert!(nodes.iter().all(|event| {
        event.value().as_radians().abs() <= angular.angular_tolerance().as_radians()
    }));

    let declination_maxima = events
        .coordinate_extrema_in(
            interval,
            CoordinateExtremumQuery::new(
                CelestialBody::Moon,
                AstrometricMode::Apparent,
                EventCoordinate::Declination,
                ExtremumKind::Maximum,
            ),
            extrema,
        )
        .unwrap();
    assert_eq!(declination_maxima.len(), 13);
    assert!(
        declination_maxima
            .iter()
            .all(|event| event.value().as_radians()
                > Angle::from_degrees(20.0).unwrap().as_radians())
    );
}

#[test]
#[ignore = "requires HYASTRO_DE440S and HYASTRO_EOP_FINALS local data files"]
fn fixed_site_extrema_apply_topocentric_parallax_with_complete_eop() {
    let kernel = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let eop_path = std::env::var_os("HYASTRO_EOP_FINALS").expect("HYASTRO_EOP_FINALS must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel]).unwrap()).unwrap();
    let base = TimeContext::builtin();
    let start = utc(&base, 2024, 3, 1, 0, 0);
    let end = utc(&base, 2024, 5, 1, 0, 0);
    let eop_data = IersFinals2000A::parse(&std::fs::read_to_string(eop_path).unwrap()).unwrap();
    let samples = eop_data
        .try_samples_in(
            &base,
            JulianDate::<Utc>::from_instant(start, &base)
                .unwrap()
                .to_modified()
                .unwrap(),
            JulianDate::<Utc>::from_instant(end, &base)
                .unwrap()
                .to_modified()
                .unwrap(),
            EarthOrientationAcceptance::IncludePredicted,
        )
        .unwrap();
    let eop = EarthOrientationTable::new(
        &samples,
        "contract IERS finals2000A",
        end.checked_add(Duration::from_days(1).unwrap()).unwrap(),
    )
    .unwrap();
    let time = base.with_earth_orientation(eop);
    let site = Earth::wgs84()
        .fixed_site(
            "Beijing",
            GeodeticPosition::new(
                GeodeticLongitude::wrap_degrees(116.391).unwrap(),
                GeodeticLatitude::try_from_degrees(39.9075).unwrap(),
                EllipsoidalHeight::from_metres(43.5).unwrap(),
            ),
        )
        .unwrap();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let interval = TimeInterval::new(start, end).unwrap();
    let query = AngularSeparationExtremumQuery::new(
        RelativeBodyQuery::new(
            CelestialBody::Mercury,
            CelestialBody::Sun,
            AstrometricMode::Apparent,
        )
        .unwrap(),
        ExtremumKind::Maximum,
    );
    let geocentric = events
        .angular_separation_extrema_in(interval, query, ExtremumSearchOptions::standard())
        .unwrap();
    let topocentric = events
        .fixed_site_angular_separation_extrema_in(
            &site,
            interval,
            query,
            ExtremumSearchOptions::standard(),
        )
        .unwrap();
    assert_eq!(geocentric.len(), 1);
    assert_eq!(topocentric.len(), 1);
    assert_ne!(geocentric[0].instant(), topocentric[0].instant());
    assert_ne!(geocentric[0].separation(), topocentric[0].separation());
}
