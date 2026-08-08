#![cfg(feature = "anise")]

use approx::assert_abs_diff_eq;
use hyastro::{
    astro::{Astrometry, Error, ReceptionLightTimeOptions},
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{CelestialBody, Ephemeris, EphemerisQuery, KernelManifest},
    frame::{Bcrs, EquatorialDirection, Frames, Gcrs},
    math::Direction,
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, Hifitime,
        IersC04, ModifiedJulianDate, Tdb, TimeContext, Utc,
    },
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");

#[test]
fn reception_light_time_options_reject_non_positive_controls() {
    assert!(matches!(
        ReceptionLightTimeOptions::new(Duration::ZERO, 10),
        Err(Error::InvalidLightTimeTolerance { nanoseconds: 0 })
    ));
    assert!(matches!(
        ReceptionLightTimeOptions::new(Duration::from_nanoseconds(-1), 10),
        Err(Error::InvalidLightTimeTolerance { nanoseconds: -1 })
    ));
    assert!(matches!(
        ReceptionLightTimeOptions::new(Duration::from_nanoseconds(1), 0),
        Err(Error::InvalidLightTimeIterationLimit { max_iterations: 0 })
    ));

    let standard = ReceptionLightTimeOptions::standard();
    assert_eq!(standard.time_tolerance(), Duration::from_nanoseconds(1));
    assert_eq!(standard.max_iterations(), 10);
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local de440s.bsp"]
fn de440s_solar_apparent_longitude_has_strict_dual_epoch_semantics() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path.clone()]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let epoch = Hifitime::new()
        .resolve(DateTime::<Gregorian, Tdb>::from_components(2000, 1, 1, 12, 0, 0, 0).unwrap())
        .unwrap();
    let astrometry = Astrometry::new(&time, &ephemeris);
    let options = ReceptionLightTimeOptions::standard();

    let light_time = astrometry
        .reception_light_time(
            EphemerisQuery::<Bcrs, _>::new(CelestialBody::Sun, CelestialBody::Earth, epoch),
            options,
        )
        .unwrap();
    assert_eq!(light_time.target(), CelestialBody::Sun);
    assert_eq!(light_time.observer(), CelestialBody::Earth);
    assert_eq!(light_time.reception_epoch(), epoch);
    assert_eq!(
        light_time
            .reception_epoch()
            .duration_since(light_time.emission_epoch())
            .unwrap(),
        light_time.light_time()
    );
    assert!((490.0..510.0).contains(&light_time.light_time().as_seconds_f64()));
    assert!(light_time.iterations() <= options.max_iterations());
    assert!(light_time.residual() <= options.time_tolerance());
    assert_abs_diff_eq!(
        light_time.distance().as_light_seconds(),
        light_time.light_time().as_seconds_f64(),
        epsilon = 1.0e-9
    );
    assert_abs_diff_eq!(
        light_time
            .relative_position()
            .direction()
            .unwrap()
            .dot(light_time.direction()),
        1.0,
        epsilon = 2.0e-16
    );

    let apparent = astrometry.solar_apparent_ecliptic(epoch, options).unwrap();
    assert_eq!(apparent.reception_epoch(), epoch);
    assert_eq!(apparent.coordinates().epoch(), epoch);
    assert_eq!(
        apparent
            .reception_epoch()
            .duration_since(apparent.emission_epoch())
            .unwrap(),
        apparent.light_time()
    );
    assert!(apparent.light_time_residual() <= options.time_tolerance());
    assert_abs_diff_eq!(
        apparent.distance().as_metres(),
        light_time.distance().as_metres(),
        epsilon = 0.0
    );
    assert_abs_diff_eq!(
        apparent.longitude().as_radians(),
        4.893_347_601_971_767,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        apparent.latitude().as_radians(),
        3.969_001_799_0e-6,
        epsilon = 1.0e-14
    );

    let one_iteration = ReceptionLightTimeOptions::new(Duration::from_nanoseconds(1), 1).unwrap();
    assert!(matches!(
        astrometry.reception_light_time(
            EphemerisQuery::<Bcrs, _>::new(CelestialBody::Sun, CelestialBody::Earth, epoch,),
            one_iteration,
        ),
        Err(Error::LightTimeDidNotConverge { iterations: 1, .. })
    ));
    assert!(matches!(
        astrometry.reception_light_time(
            EphemerisQuery::<Bcrs, _>::new(CelestialBody::Sun, CelestialBody::Sun, epoch,),
            options,
        ),
        Err(Error::UndefinedIdentityObservation {
            body: CelestialBody::Sun
        })
    ));

    let coverage = ephemeris
        .coverage(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Sun,
            CelestialBody::Earth,
            epoch,
        ))
        .unwrap();
    assert!(matches!(
        astrometry.reception_light_time(
            EphemerisQuery::<Bcrs, _>::new(
                CelestialBody::Sun,
                CelestialBody::Earth,
                coverage.start(),
            ),
            options,
        ),
        Err(Error::Ephemeris(hyastro::ephem::Error::Coverage { .. }))
    ));

    let reference = anise::prelude::Almanac::default()
        .load(path.to_str().expect("DE440s path must be UTF-8"))
        .unwrap()
        .translate(
            anise::constants::frames::SUN_J2000,
            anise::constants::frames::EARTH_J2000,
            Hifitime::new().export(epoch),
            anise::astro::Aberration::CN_S,
        )
        .unwrap();
    assert_abs_diff_eq!(
        apparent.distance().as_kilometres(),
        reference.radius_km.norm(),
        epsilon = 1.0e-3
    );

    let reference_direction = Direction::<Gcrs>::try_from_components([
        reference.radius_km[0],
        reference.radius_km[1],
        reference.radius_km[2],
    ])
    .unwrap();
    let public_direction = Frames::new(&time)
        .celestial_orientation_at(epoch)
        .unwrap()
        .gcrs_from_true_ecliptic(apparent.coordinates())
        .unwrap()
        .to_direction()
        .unwrap();
    let natural_direction =
        Direction::<Gcrs>::try_from_components(light_time.direction().components()).unwrap();
    let aberration_arcseconds = natural_direction
        .angle_to(public_direction)
        .unwrap()
        .as_radians()
        .to_degrees()
        * 3_600.0;
    assert!((15.0..25.0).contains(&aberration_arcseconds));
    assert!(
        public_direction
            .angle_to(reference_direction)
            .unwrap()
            .as_radians()
            < 1.0e-11
    );

    let eop_data = IersC04::parse(C04).unwrap();
    let eop_samples = eop_data
        .try_samples_in(
            &time,
            ModifiedJulianDate::<Utc>::from_parts(51_543.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(51_546.0, 0.0).unwrap(),
            EarthOrientationAcceptance::FinalOnly,
        )
        .unwrap();
    let eop_expires = eop_samples[eop_samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let eop =
        EarthOrientationTable::new(&eop_samples, "C04 topocentric test", eop_expires).unwrap();
    let observed_time = time.with_earth_orientation(eop);
    let site = Earth::wgs84()
        .fixed_site(
            "topocentric test site",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(116.391).unwrap(),
                GeodeticLatitude::try_from_degrees(39.9075).unwrap(),
                EllipsoidalHeight::from_metres(43.5).unwrap(),
            ),
        )
        .unwrap();
    let observed_astrometry = Astrometry::new(&observed_time, &ephemeris);
    let observer = observed_astrometry.fixed_observer_at(&site, epoch).unwrap();
    let place = observer
        .vacuum_observed_place(CelestialBody::Sun, options)
        .unwrap();
    assert_eq!(observer.epoch(), epoch);
    assert_eq!(place.reception_epoch(), epoch);
    assert_eq!(place.intermediate_equatorial().epoch(), epoch);
    assert_eq!(
        place
            .reception_epoch()
            .duration_since(place.emission_epoch())
            .unwrap(),
        place.light_time()
    );
    assert!(place.light_time_residual() <= options.time_tolerance());
    assert!(place.horizontal().azimuth().is_some());
    assert!((-90.0..=90.0).contains(&place.horizontal().altitude().as_degrees()));
    assert!((place.distance().as_metres() - apparent.distance().as_metres()).abs() < 7_000_000.0);

    let earth_barycentric = ephemeris
        .state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Earth,
            CelestialBody::SolarSystemBarycenter,
            epoch,
        ))
        .unwrap();
    let site_velocity = observer
        .barycentric_velocity()
        .checked_sub(earth_barycentric.velocity())
        .unwrap()
        .magnitude()
        .unwrap()
        .as_metres_per_second();
    assert!((300.0..500.0).contains(&site_velocity));

    let orientation = Frames::new(&observed_time)
        .earth_orientation_at(epoch)
        .unwrap();
    let geocentric_intermediate = orientation
        .intermediate_equatorial(EquatorialDirection::from_direction(public_direction).unwrap())
        .unwrap();
    let topocentric_direction = place
        .intermediate_equatorial()
        .coordinates()
        .to_direction()
        .unwrap();
    let geocentric_direction = geocentric_intermediate
        .coordinates()
        .to_direction()
        .unwrap();
    let topocentric_shift_arcseconds = topocentric_direction
        .angle_to(geocentric_direction)
        .unwrap()
        .as_radians()
        .to_degrees()
        * 3_600.0;
    assert!((0.1..10.0).contains(&topocentric_shift_arcseconds));

    let one_iteration = ReceptionLightTimeOptions::new(Duration::from_nanoseconds(1), 1).unwrap();
    assert!(matches!(
        observer.vacuum_observed_place(CelestialBody::Sun, one_iteration),
        Err(Error::FixedSiteLightTimeDidNotConverge { iterations: 1, .. })
    ));

    println!(
        "light_time_s={:.9} residual_ns={} iterations={} longitude_rad={:.16} latitude_rad={:.16} anise_angle_rad={:.3e}",
        apparent.light_time().as_seconds_f64(),
        apparent.light_time_residual().as_nanoseconds(),
        apparent.iterations(),
        apparent.longitude().as_radians(),
        apparent.latitude().as_radians(),
        public_direction
            .angle_to(reference_direction)
            .unwrap()
            .as_radians(),
    );
}
