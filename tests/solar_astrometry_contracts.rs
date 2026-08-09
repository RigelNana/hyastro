#![cfg(feature = "anise")]

use approx::assert_abs_diff_eq;
use hyastro::{
    astro::{
        AirTemperature, ApparentDiskRelationship, Astrometry, AtmosphericConditions,
        AtmosphericPressure, Error, ObservingWavelength, ReceptionLightTimeOptions,
        RefractionAccuracy, RelativeHumidity, SolarDeflectionDisposition,
    },
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{CelestialBody, Ephemeris, EphemerisQuery, KernelManifest, SphericalBodyFigure},
    frame::{Bcrs, EquatorialDirection, Frames, Gcrs},
    math::{Direction, Longitude},
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
fn atmospheric_conditions_enforce_the_explicit_sofa_domain() {
    assert!(matches!(
        AtmosphericPressure::from_hectopascals(f64::NAN),
        Err(Error::NonFiniteAtmosphericValue {
            field: "atmospheric pressure",
            ..
        })
    ));
    assert!(matches!(
        AtmosphericPressure::from_hectopascals(-0.1),
        Err(Error::AtmosphericValueOutOfRange {
            field: "atmospheric pressure",
            ..
        })
    ));
    assert!(AtmosphericPressure::from_hectopascals(10_000.0).is_ok());
    assert!(AtmosphericPressure::from_hectopascals(10_000.1).is_err());

    assert!(AirTemperature::from_degrees_celsius(-150.0).is_ok());
    assert!(AirTemperature::from_degrees_celsius(200.0).is_ok());
    assert!(AirTemperature::from_degrees_celsius(-150.1).is_err());
    assert!(AirTemperature::from_degrees_celsius(200.1).is_err());

    assert!(RelativeHumidity::from_fraction(0.0).is_ok());
    assert!(RelativeHumidity::from_fraction(1.0).is_ok());
    assert!(RelativeHumidity::from_fraction(-0.001).is_err());
    assert!(RelativeHumidity::from_fraction(1.001).is_err());

    assert!(ObservingWavelength::from_micrometres(0.1).is_ok());
    assert!(ObservingWavelength::from_micrometres(1.0e6).is_ok());
    assert!(ObservingWavelength::from_micrometres(0.099).is_err());
    assert!(ObservingWavelength::from_micrometres(1.0e6 + 1.0).is_err());
    assert!(
        ObservingWavelength::from_micrometres(100.0)
            .unwrap()
            .is_optical_or_infrared()
    );
    assert!(
        !ObservingWavelength::from_micrometres(100.001)
            .unwrap()
            .is_optical_or_infrared()
    );
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

    let apparent = astrometry.solar_apparent_place(epoch, options).unwrap();
    assert_eq!(apparent.reception_epoch(), epoch);
    assert_eq!(apparent.true_ecliptic().epoch(), epoch);
    assert_eq!(apparent.geocentric().target(), CelestialBody::Sun);
    assert_eq!(
        apparent.reception_light_time().observer(),
        CelestialBody::Earth
    );
    assert_eq!(
        apparent.solar_light_deflection().disposition(),
        SolarDeflectionDisposition::NotAppliedToSun
    );
    assert_abs_diff_eq!(
        apparent.solar_light_deflection().correction().as_radians(),
        0.0,
        epsilon = 0.0
    );
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
    let public_direction = apparent
        .gcrs_direction()
        .coordinates()
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
    let solar_deflection = place.solar_light_deflection();
    assert_eq!(
        solar_deflection.disposition(),
        SolarDeflectionDisposition::NotAppliedToSun
    );
    assert_eq!(solar_deflection.deflector_epoch(), place.emission_epoch());
    assert_eq!(solar_deflection.correction().as_radians(), 0.0);
    let solar_disk = place
        .apparent_disk(SphericalBodyFigure::IAU_2015_NOMINAL_SUN)
        .unwrap();
    let solar_semidiameter_arcminutes = solar_disk.semidiameter().as_degrees() * 60.0;
    assert!((15.0..17.0).contains(&solar_semidiameter_arcminutes));
    assert_abs_diff_eq!(
        solar_disk.diameter().as_radians(),
        2.0 * (SphericalBodyFigure::IAU_2015_NOMINAL_SUN
            .radius()
            .as_metres()
            / place.distance().as_metres())
        .asin(),
        epsilon = 2.0e-18
    );

    let moon_place = observer
        .vacuum_observed_place(CelestialBody::Moon, options)
        .unwrap();
    assert_eq!(
        moon_place.solar_light_deflection().disposition(),
        SolarDeflectionDisposition::Applied
    );
    assert!(
        moon_place
            .solar_light_deflection()
            .correction()
            .as_radians()
            > 0.0
    );
    let moon_disk = moon_place
        .apparent_disk(SphericalBodyFigure::IAU_WGCCRE_2015_MOON)
        .unwrap();
    let disk_separation = solar_disk.separation_from(moon_disk).unwrap();
    assert_eq!(
        disk_separation.relationship(),
        ApparentDiskRelationship::Separate
    );
    assert!(disk_separation.limb_clearance().as_radians() > 0.0);

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

    let greenwich_site = Earth::wgs84()
        .fixed_site(
            "Greenwich refraction test site",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(0.0).unwrap(),
                GeodeticLatitude::try_from_degrees(0.0).unwrap(),
                EllipsoidalHeight::from_metres(0.0).unwrap(),
            ),
        )
        .unwrap();
    let greenwich_observer = observed_astrometry
        .fixed_observer_at(&greenwich_site, epoch)
        .unwrap();
    let greenwich_vacuum = greenwich_observer
        .vacuum_observed_place(CelestialBody::Sun, options)
        .unwrap();
    let atmosphere = |pressure_hectopascals, wavelength_micrometres| {
        AtmosphericConditions::new(
            AtmosphericPressure::from_hectopascals(pressure_hectopascals).unwrap(),
            AirTemperature::from_degrees_celsius(10.0).unwrap(),
            RelativeHumidity::from_fraction(0.8).unwrap(),
            ObservingWavelength::from_micrometres(wavelength_micrometres).unwrap(),
        )
    };
    let vacuum_altitude = greenwich_vacuum.horizontal().altitude().as_radians();
    assert!((60.0..70.0).contains(&greenwich_vacuum.horizontal().altitude().as_degrees()));

    let zero_pressure = greenwich_vacuum
        .apply_refraction(atmosphere(0.0, 0.55))
        .unwrap();
    assert_abs_diff_eq!(
        zero_pressure.horizontal().altitude().as_radians(),
        vacuum_altitude,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        zero_pressure.horizontal().azimuth().unwrap().as_radians(),
        greenwich_vacuum
            .horizontal()
            .azimuth()
            .unwrap()
            .as_radians(),
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        zero_pressure.refraction().amount().as_radians(),
        0.0,
        epsilon = 1.0e-14
    );

    let nominal = greenwich_vacuum
        .apply_refraction(atmosphere(1_013.25, 0.55))
        .unwrap();
    assert!(nominal.horizontal().altitude().as_radians() > vacuum_altitude);
    assert_eq!(nominal.refraction().accuracy(), RefractionAccuracy::Nominal);
    assert_eq!(nominal.vacuum(), greenwich_vacuum);
    assert_abs_diff_eq!(
        nominal.refraction().amount().as_radians(),
        nominal.horizontal().altitude().as_radians() - vacuum_altitude,
        epsilon = 1.0e-15
    );

    let low_pressure = greenwich_vacuum
        .apply_refraction(atmosphere(800.0, 0.55))
        .unwrap();
    let high_pressure = greenwich_vacuum
        .apply_refraction(atmosphere(1_050.0, 0.55))
        .unwrap();
    assert!(
        high_pressure.refraction().amount().as_radians()
            > low_pressure.refraction().amount().as_radians()
    );

    let blue = greenwich_vacuum
        .apply_refraction(atmosphere(1_013.25, 0.4))
        .unwrap();
    let infrared = greenwich_vacuum
        .apply_refraction(atmosphere(1_013.25, 2.0))
        .unwrap();
    assert!(blue.refraction().amount().as_radians() > infrared.refraction().amount().as_radians());

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

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local de440s.bsp"]
fn de440s_moon_geocentric_apparent_place_preserves_target_and_dual_epochs() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path.clone()]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let epoch = Hifitime::new()
        .resolve(DateTime::<Gregorian, Tdb>::from_components(2000, 1, 1, 12, 0, 0, 0).unwrap())
        .unwrap();
    let astrometry = Astrometry::new(&time, &ephemeris);
    let options = ReceptionLightTimeOptions::standard();

    let apparent = astrometry
        .geocentric_apparent_place(CelestialBody::Moon, epoch, options)
        .unwrap();
    let reception = apparent.reception_light_time();
    assert_eq!(apparent.target(), CelestialBody::Moon);
    assert_eq!(reception.observer(), CelestialBody::Earth);
    assert_eq!(apparent.reception_epoch(), epoch);
    assert_eq!(apparent.gcrs_direction().epoch(), epoch);
    assert_eq!(apparent.true_equatorial().epoch(), epoch);
    assert_eq!(apparent.true_ecliptic().epoch(), epoch);
    assert_eq!(
        apparent
            .reception_epoch()
            .duration_since(apparent.emission_epoch())
            .unwrap(),
        apparent.light_time()
    );
    assert!((1.0..1.5).contains(&apparent.light_time().as_seconds_f64()));
    assert!(apparent.iterations() <= options.max_iterations());
    assert!(apparent.light_time_residual() <= options.time_tolerance());
    assert_eq!(
        apparent.solar_light_deflection().disposition(),
        SolarDeflectionDisposition::Applied
    );
    assert!((0.0..1.0e-6).contains(&apparent.solar_light_deflection().correction().as_radians()));

    let reference = anise::prelude::Almanac::default()
        .load(path.to_str().expect("DE440s path must be UTF-8"))
        .unwrap()
        .translate(
            anise::constants::frames::MOON_J2000,
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
    let apparent_direction = apparent
        .gcrs_direction()
        .coordinates()
        .to_direction()
        .unwrap();
    assert!(
        apparent_direction
            .angle_to(reference_direction)
            .unwrap()
            .as_radians()
            < 1.0e-7
    );

    assert!(matches!(
        astrometry.geocentric_apparent_place(CelestialBody::Earth, epoch, options),
        Err(Error::UndefinedIdentityObservation {
            body: CelestialBody::Earth
        })
    ));
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local de440s.bsp"]
fn de440s_equation_of_time_matches_usno_and_is_longitude_invariant() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let base = TimeContext::builtin();
    let eop_data = IersC04::parse(C04).unwrap();
    let samples = eop_data
        .try_samples_in(
            &base,
            ModifiedJulianDate::<Utc>::from_parts(60_350.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(60_619.0, 0.0).unwrap(),
            EarthOrientationAcceptance::FinalOnly,
        )
        .unwrap();
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let eop = EarthOrientationTable::new(&samples, "C04 solar-time test", expires).unwrap();
    let time = base.with_earth_orientation(eop);
    let astrometry = Astrometry::new(&time, &ephemeris);

    // USNO Apparent Geocentric Positions, true equator and equinox of date:
    // 2024-02-11 12:00 UT1 -> -14m11.6s
    // 2024-11-03 12:00 UT1 -> +16m27.0s
    // https://aa.usno.navy.mil/data/geocentric
    for (month, day, expected_seconds) in [(2, 11, -851.6), (11, 3, 987.0)] {
        let epoch = base
            .resolve(
                DateTime::<Gregorian, Utc>::from_components(2024, month, day, 12, 0, 0, 0).unwrap(),
            )
            .unwrap();
        let solution = astrometry
            .solar_time(epoch, ReceptionLightTimeOptions::standard())
            .unwrap();
        assert_eq!(solution.epoch(), epoch);
        assert_eq!(solution.apparent_sun().true_equatorial().epoch(), epoch);
        assert_abs_diff_eq!(
            solution.equation_of_time().as_seconds(),
            expected_seconds,
            epsilon = 0.15
        );

        let greenwich = solution.greenwich();
        let local = solution
            .at_longitude(Longitude::try_from_degrees(30.0).unwrap())
            .unwrap();
        assert_eq!(local.equation_of_time(), solution.equation_of_time());
        assert_abs_diff_eq!(
            local.mean_solar_time().as_decimal_hours()
                - greenwich.mean_solar_time().as_decimal_hours(),
            2.0,
            epsilon = 1.0e-14
        );
        assert_abs_diff_eq!(
            local.apparent_solar_time().as_decimal_hours()
                - greenwich.apparent_solar_time().as_decimal_hours(),
            2.0,
            epsilon = 1.0e-14
        );

        let mean_nanoseconds = i128::from(
            greenwich
                .mean_solar_time()
                .as_time_of_day()
                .nanoseconds_since_midnight(),
        );
        let apparent_nanoseconds = i128::from(
            greenwich
                .apparent_solar_time()
                .as_time_of_day()
                .nanoseconds_since_midnight(),
        );
        let half_day = Duration::NANOSECONDS_PER_DAY / 2;
        let clock_difference = (apparent_nanoseconds - mean_nanoseconds + half_day)
            .rem_euclid(Duration::NANOSECONDS_PER_DAY)
            - half_day;
        assert!(
            (clock_difference - solution.equation_of_time().duration().as_nanoseconds()).abs() <= 1
        );
    }
}
