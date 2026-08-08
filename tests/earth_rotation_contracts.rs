use approx::assert_abs_diff_eq;
use hyastro::{
    frame::{Cirs, Frames, State, Tirs},
    math::{Length, Point3, Speed, Vector3},
    time::{
        CelestialPoleOffsetX, CelestialPoleOffsetY, DateTime, EarthOrientationSample,
        EarthOrientationTable, EarthRotationSample, EarthRotationTable, Error, ExcessLengthOfDay,
        Gregorian, JulianDate, PolarMotionX, PolarMotionY, Tai, TimeContext, Ut1, Ut1MinusUtc, Utc,
    },
};

#[test]
fn earth_rotation_table_keeps_ut1_continuous_across_a_utc_leap_second() {
    let base = TimeContext::builtin();
    let left = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 0, 0, 0, 0).unwrap())
        .unwrap();
    let right = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2017, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let midpoint = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 12, 0, 0, 0).unwrap())
        .unwrap();
    let uncovered = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2017, 1, 2, 0, 0, 0, 0).unwrap())
        .unwrap();
    let expires = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2017, 1, 3, 0, 0, 0, 0).unwrap())
        .unwrap();
    let samples = [
        EarthRotationSample::new(left, Ut1MinusUtc::from_seconds(-0.4).unwrap()),
        EarthRotationSample::new(right, Ut1MinusUtc::from_seconds(0.6).unwrap()),
    ];
    let table = EarthRotationTable::new(&samples, "synthetic leap", expires).unwrap();
    let time = base.with_earth_rotation(table);

    let rotation = time.earth_rotation_at(midpoint).unwrap();
    assert_abs_diff_eq!(
        rotation.ut1_minus_utc().as_seconds(),
        -0.4,
        epsilon = 1.0e-12
    );
    let tai = JulianDate::<Tai>::from_instant(midpoint, &time).unwrap();
    let ut1 = JulianDate::<Ut1>::from_instant(midpoint, &time).unwrap();
    let ut1_minus_tai_seconds =
        ((ut1.parts().0 - tai.parts().0) + (ut1.parts().1 - tai.parts().1)) * 86_400.0;
    assert_abs_diff_eq!(ut1_minus_tai_seconds, -36.4, epsilon = 2.0e-11);
    assert!(matches!(
        time.earth_rotation_at(uncovered),
        Err(Error::EarthOrientationUnavailable { .. })
    ));
    assert!(matches!(
        time.earth_rotation_at(expires),
        Err(Error::EarthOrientationExpired { .. })
    ));
}

#[test]
fn eop_interpolation_keeps_ut1_continuous_across_a_utc_leap_second() {
    let base = TimeContext::builtin();
    let left = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 0, 0, 0, 0).unwrap())
        .unwrap();
    let right = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2017, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let midpoint = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 12, 0, 0, 0).unwrap())
        .unwrap();
    let uncovered = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2017, 1, 2, 0, 0, 0, 0).unwrap())
        .unwrap();
    let expires = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2017, 1, 3, 0, 0, 0, 0).unwrap())
        .unwrap();

    let zero_lod = ExcessLengthOfDay::from_milliseconds(0.0).unwrap();
    let zero_xp = PolarMotionX::from_arcseconds(0.0).unwrap();
    let zero_yp = PolarMotionY::from_arcseconds(0.0).unwrap();
    let zero_dx = CelestialPoleOffsetX::from_milliarcseconds(0.0).unwrap();
    let zero_dy = CelestialPoleOffsetY::from_milliarcseconds(0.0).unwrap();
    let samples = [
        EarthOrientationSample::new(
            left,
            Ut1MinusUtc::from_seconds(-0.4).unwrap(),
            zero_lod,
            zero_xp,
            zero_yp,
            zero_dx,
            zero_dy,
        ),
        EarthOrientationSample::new(
            right,
            Ut1MinusUtc::from_seconds(0.6).unwrap(),
            zero_lod,
            zero_xp,
            zero_yp,
            zero_dx,
            zero_dy,
        ),
    ];
    let table = EarthOrientationTable::new(&samples, "synthetic leap", expires).unwrap();
    let time = base.with_earth_orientation(table);

    let orientation = time.earth_orientation_at(midpoint).unwrap();
    assert_abs_diff_eq!(
        orientation.ut1_minus_utc().as_seconds(),
        -0.4,
        epsilon = 1.0e-12
    );

    let tai = JulianDate::<Tai>::from_instant(midpoint, &time).unwrap();
    let ut1 = JulianDate::<Ut1>::from_instant(midpoint, &time).unwrap();
    let ut1_minus_tai_seconds =
        ((ut1.parts().0 - tai.parts().0) + (ut1.parts().1 - tai.parts().1)) * 86_400.0;
    assert_abs_diff_eq!(ut1_minus_tai_seconds, -36.4, epsilon = 2.0e-11);

    assert!(matches!(
        time.earth_orientation_at(uncovered),
        Err(Error::EarthOrientationUnavailable { .. })
    ));
    assert!(matches!(
        time.earth_orientation_at(expires),
        Err(Error::EarthOrientationExpired { .. })
    ));
}

#[test]
fn cirs_to_tirs_matches_the_sofa_era_reference_and_rotating_state_derivative() {
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

    let zero_dut1 = Ut1MinusUtc::from_seconds(0.0).unwrap();
    let excess_lod = ExcessLengthOfDay::from_milliseconds(1.0).unwrap();
    let zero_xp = PolarMotionX::from_arcseconds(0.0).unwrap();
    let zero_yp = PolarMotionY::from_arcseconds(0.0).unwrap();
    let zero_dx = CelestialPoleOffsetX::from_milliarcseconds(0.0).unwrap();
    let zero_dy = CelestialPoleOffsetY::from_milliarcseconds(0.0).unwrap();
    let samples = [
        EarthOrientationSample::new(
            left, zero_dut1, excess_lod, zero_xp, zero_yp, zero_dx, zero_dy,
        ),
        EarthOrientationSample::new(
            right, zero_dut1, excess_lod, zero_xp, zero_yp, zero_dx, zero_dy,
        ),
    ];
    let table = EarthOrientationTable::new(&samples, "SOFA era00 reference", expires).unwrap();
    let time = base.with_earth_orientation(table);
    let frames = Frames::new(&time);

    let ut1 = JulianDate::<Ut1>::from_instant(epoch, &time).unwrap();
    assert_abs_diff_eq!(ut1.as_f64_lossy(), 2_454_388.5, epsilon = 0.0);

    let transform = frames.at::<Cirs, Tirs, Utc>(epoch).unwrap();
    let reference_era: f64 = 0.402_283_724_002_815_8;
    let cosine = reference_era.cos();
    let sine = reference_era.sin();
    let matrix = transform.rotation().matrix();
    assert_abs_diff_eq!(matrix.element(0, 0).unwrap(), cosine, epsilon = 1.0e-12);
    assert_abs_diff_eq!(matrix.element(0, 1).unwrap(), sine, epsilon = 1.0e-12);
    assert_abs_diff_eq!(matrix.element(1, 0).unwrap(), -sine, epsilon = 1.0e-12);
    assert_abs_diff_eq!(matrix.element(1, 1).unwrap(), cosine, epsilon = 1.0e-12);
    assert_abs_diff_eq!(matrix.element(2, 2).unwrap(), 1.0, epsilon = 0.0);

    let expected_angular_speed = 7.292_115_0e-5 * 86_400.0 / 86_400.001;
    let angular_velocity = transform.angular_velocity().components();
    assert_abs_diff_eq!(
        angular_velocity[2].as_radians_per_second(),
        -expected_angular_speed,
        epsilon = 1.0e-16
    );

    let state = State::new(
        Point3::<Cirs>::new(
            Length::from_metres(1.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(0.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
        ),
        epoch,
    );
    let transformed: State<Tirs, Utc> = frames.transform(state).unwrap();
    let position = transformed.position().position().components();
    let velocity = transformed.velocity().components();
    assert_abs_diff_eq!(position[0].as_metres(), cosine, epsilon = 1.0e-12);
    assert_abs_diff_eq!(position[1].as_metres(), -sine, epsilon = 1.0e-12);
    assert_abs_diff_eq!(
        velocity[0].as_metres_per_second(),
        -expected_angular_speed * sine,
        epsilon = 1.0e-16
    );
    assert_abs_diff_eq!(
        velocity[1].as_metres_per_second(),
        -expected_angular_speed * cosine,
        epsilon = 1.0e-16
    );
    assert_eq!(transformed.epoch(), epoch);
}
