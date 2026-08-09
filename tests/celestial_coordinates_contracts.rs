use core::f64::consts::FRAC_PI_4;

use approx::assert_abs_diff_eq;
use hyastro::{
    frame::{
        EclipticDirection, EclipticDirectionAt, EclipticLatitude, EclipticLongitude,
        EquatorialDirection, EquatorialDirectionAt, Error as FrameError, Frames, GalacticDirection,
        GalacticLatitude, GalacticLongitude, Gcrs, Icrs, MeanEclipticEquinoxJ2000,
        MeanEclipticEquinoxOfDate, MeanEquatorEquinoxJ2000, MeanEquatorEquinoxOfDate,
        TrueEclipticEquinoxOfDate,
    },
    math::{Declination, RightAscension},
    time::{
        CelestialPoleOffsetX, CelestialPoleOffsetY, DateTime, Duration, EarthOrientationSample,
        EarthOrientationTable, ExcessLengthOfDay, Gregorian, PolarMotionX, PolarMotionY,
        TimeContext, Ut1MinusUtc, Utc,
    },
};

#[test]
fn coordinate_angles_enforce_distinct_semantics_and_ranges() {
    let ecliptic_longitude = EclipticLongitude::wrap_degrees(-30.0).unwrap();
    let galactic_longitude = GalacticLongitude::wrap_degrees(390.0).unwrap();
    let ecliptic_latitude = EclipticLatitude::try_from_degrees(-45.0).unwrap();
    let galactic_latitude = GalacticLatitude::try_from_degrees(45.0).unwrap();

    assert_abs_diff_eq!(ecliptic_longitude.as_degrees(), 330.0, epsilon = 1.0e-12);
    assert_abs_diff_eq!(galactic_longitude.as_degrees(), 30.0, epsilon = 1.0e-12);
    assert_abs_diff_eq!(ecliptic_latitude.as_degrees(), -45.0, epsilon = 1.0e-12);
    assert_abs_diff_eq!(galactic_latitude.as_degrees(), 45.0, epsilon = 1.0e-12);
    assert!(EclipticLongitude::try_from_degrees(360.0).is_err());
    assert!(GalacticLatitude::try_from_degrees(90.000_001).is_err());
}

#[test]
fn fixed_j2000_ecliptic_equatorial_and_galactic_conversions_match_references() {
    let icrs = EquatorialDirection::<Icrs>::new(
        RightAscension::try_from_radians(5.933_807_430_222_719).unwrap(),
        Declination::try_from_radians(-1.178_487_061_357_994_5).unwrap(),
    );

    let galactic = GalacticDirection::from_icrs(icrs).unwrap();
    assert_abs_diff_eq!(
        galactic.longitude().as_radians(),
        5.585_053_606_381_854,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        galactic.latitude().as_radians(),
        -FRAC_PI_4,
        epsilon = 1.0e-14
    );
    assert!(
        galactic
            .to_icrs()
            .unwrap()
            .separation_to(icrs)
            .unwrap()
            .as_radians()
            < 1.0e-14
    );

    let ecliptic = EclipticDirection::<MeanEclipticEquinoxJ2000>::from_icrs(icrs).unwrap();
    assert_abs_diff_eq!(
        ecliptic.longitude().as_radians(),
        5.347_306_084_305_337,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        ecliptic.latitude().as_radians(),
        -0.920_197_070_093_526_1,
        epsilon = 1.0e-14
    );
    assert!(
        ecliptic
            .to_icrs()
            .unwrap()
            .separation_to(icrs)
            .unwrap()
            .as_radians()
            < 1.0e-14
    );
    let gcrs = EquatorialDirection::<Gcrs>::new(icrs.right_ascension(), icrs.declination());
    let mean_j2000 = EquatorialDirection::<MeanEquatorEquinoxJ2000>::from_gcrs(gcrs).unwrap();
    assert!(
        mean_j2000
            .to_gcrs()
            .unwrap()
            .separation_to(gcrs)
            .unwrap()
            .as_radians()
            < 1.0e-14
    );
}

#[test]
fn celestial_orientation_binds_of_date_coordinates_to_its_epoch() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let solution = Frames::new(&time).celestial_orientation_at(epoch).unwrap();
    let icrs = EquatorialDirection::<Icrs>::new(
        RightAscension::try_from_degrees(201.25).unwrap(),
        Declination::try_from_degrees(-47.5).unwrap(),
    );
    let gcrs = EquatorialDirection::<Gcrs>::new(icrs.right_ascension(), icrs.declination());

    let mean = solution.mean_equatorial(gcrs).unwrap();
    let true_equatorial = solution.true_equatorial(gcrs).unwrap();
    let ecliptic = solution.mean_ecliptic(icrs).unwrap();
    let gcrs_ecliptic = solution.mean_ecliptic_from_gcrs(gcrs).unwrap();
    assert_eq!(mean.epoch(), epoch);
    assert_eq!(true_equatorial.epoch(), epoch);
    assert_eq!(ecliptic.epoch(), epoch);
    assert_eq!(gcrs_ecliptic.epoch(), epoch);
    assert!(
        solution
            .gcrs_from_mean_equatorial(mean)
            .unwrap()
            .separation_to(gcrs)
            .unwrap()
            .as_radians()
            < 1.0e-14
    );
    assert!(
        solution
            .gcrs_from_true_equatorial(true_equatorial)
            .unwrap()
            .separation_to(gcrs)
            .unwrap()
            .as_radians()
            < 1.0e-14
    );
    assert!(
        solution
            .icrs_from_mean_ecliptic(ecliptic)
            .unwrap()
            .separation_to(icrs)
            .unwrap()
            .as_radians()
            < 1.0e-14
    );
    assert!(
        solution
            .gcrs_from_mean_ecliptic(gcrs_ecliptic)
            .unwrap()
            .separation_to(gcrs)
            .unwrap()
            .as_radians()
            < 1.0e-14
    );

    let wrong_epoch = epoch
        .checked_add(Duration::from_seconds(1).unwrap())
        .unwrap();
    let wrong_mean = EquatorialDirectionAt::<MeanEquatorEquinoxOfDate, Utc>::new(
        wrong_epoch,
        mean.coordinates(),
    );
    assert!(matches!(
        solution.gcrs_from_mean_equatorial(wrong_mean),
        Err(FrameError::EpochMismatch { .. })
    ));
    let wrong_ecliptic = EclipticDirectionAt::<MeanEclipticEquinoxOfDate, Utc>::new(
        wrong_epoch,
        ecliptic.coordinates(),
    );
    assert!(matches!(
        solution.icrs_from_mean_ecliptic(wrong_ecliptic),
        Err(FrameError::EpochMismatch { .. })
    ));
    let wrong_gcrs_ecliptic = EclipticDirectionAt::<MeanEclipticEquinoxOfDate, Utc>::new(
        wrong_epoch,
        gcrs_ecliptic.coordinates(),
    );
    assert!(matches!(
        solution.gcrs_from_mean_ecliptic(wrong_gcrs_ecliptic),
        Err(FrameError::EpochMismatch { .. })
    ));
}

#[test]
fn true_ecliptic_of_date_matches_erfa_convention_and_round_trips() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let solution = Frames::new(&time).celestial_orientation_at(epoch).unwrap();
    let icrs = EquatorialDirection::<Icrs>::new(
        RightAscension::try_from_degrees(201.25).unwrap(),
        Declination::try_from_degrees(-47.5).unwrap(),
    );

    let true_ecliptic = solution.true_ecliptic(icrs).unwrap();

    assert_eq!(true_ecliptic.epoch(), epoch);
    assert_abs_diff_eq!(
        true_ecliptic.coordinates().longitude().as_radians(),
        3.835_733_818_748_319_7,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        true_ecliptic.coordinates().latitude().as_radians(),
        -0.617_588_143_743_217_1,
        epsilon = 1.0e-14
    );
    assert!(
        solution
            .icrs_from_true_ecliptic(true_ecliptic)
            .unwrap()
            .separation_to(icrs)
            .unwrap()
            .as_radians()
            < 1.0e-14
    );

    let wrong_epoch = epoch
        .checked_add(Duration::from_seconds(1).unwrap())
        .unwrap();
    let wrong_ecliptic = EclipticDirectionAt::<TrueEclipticEquinoxOfDate, Utc>::new(
        wrong_epoch,
        true_ecliptic.coordinates(),
    );
    assert!(matches!(
        solution.icrs_from_true_ecliptic(wrong_ecliptic),
        Err(FrameError::EpochMismatch { .. })
    ));
}

#[test]
fn observed_cirs_coordinates_round_trip_through_one_eop_solution() {
    let base = TimeContext::builtin();
    let left = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2007, 10, 14, 0, 0, 0, 0).unwrap())
        .unwrap();
    let epoch = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2007, 10, 15, 0, 0, 0, 0).unwrap())
        .unwrap();
    let right = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2007, 10, 16, 0, 0, 0, 0).unwrap())
        .unwrap();
    let expires = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2007, 10, 17, 0, 0, 0, 0).unwrap())
        .unwrap();
    let sample = |epoch| {
        EarthOrientationSample::new(
            epoch,
            Ut1MinusUtc::from_seconds(0.0).unwrap(),
            ExcessLengthOfDay::from_milliseconds(1.0).unwrap(),
            PolarMotionX::from_arcseconds(0.0).unwrap(),
            PolarMotionY::from_arcseconds(0.0).unwrap(),
            CelestialPoleOffsetX::from_milliarcseconds(0.1).unwrap(),
            CelestialPoleOffsetY::from_milliarcseconds(-0.2).unwrap(),
        )
    };
    let samples = [sample(left), sample(right)];
    let table = EarthOrientationTable::new(&samples, "CIRS coordinate test", expires).unwrap();
    let time = base.with_earth_orientation(table);
    let solution = Frames::new(&time).earth_orientation_at(epoch).unwrap();
    let gcrs = EquatorialDirection::<Gcrs>::new(
        RightAscension::try_from_degrees(120.0).unwrap(),
        Declination::try_from_degrees(-35.0).unwrap(),
    );

    let cirs = solution.intermediate_equatorial(gcrs).unwrap();
    assert_eq!(cirs.epoch(), epoch);
    assert!(
        solution
            .gcrs_from_intermediate_equatorial(cirs)
            .unwrap()
            .separation_to(gcrs)
            .unwrap()
            .as_radians()
            < 1.0e-14
    );
}
