use approx::assert_abs_diff_eq;
use hyastro::{
    earth::{
        Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition,
        SiteVelocityModel,
    },
    frame::{Cirs, Frames, Gcrs, Itrs, State, Tirs},
    math::{Length, Longitude, Point3, Speed, Vector3},
    time::{
        DateTime, Duration, EarthAttitudeTable, EarthOrientationAcceptance,
        EarthOrientationProduct, EarthOrientationTable, EarthRotationTable, Error, Gregorian,
        IersC04, IersFinals2000A, JulianDate, ModifiedJulianDate, TimeContext, Tt, Ut1, Utc,
    },
    uncertainty::UncertaintyOrigin,
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");
const FINALS_2000_A: &str = include_str!("../data/eop/finals2000a-2026-08-09.all");

#[test]
fn c04_snapshot_preserves_complete_values_rates_and_coverage() {
    let data = IersC04::parse(C04).unwrap();
    assert_eq!(data.product(), EarthOrientationProduct::IersC04);
    assert_eq!(data.records().len(), 23_564);

    let first = data.records()[0];
    let last = data.records()[data.records().len() - 1];
    assert_abs_diff_eq!(
        first.modified_julian_date().as_f64_lossy(),
        37_665.0,
        epsilon = 0.0
    );
    assert_abs_diff_eq!(
        last.modified_julian_date().as_f64_lossy(),
        61_228.0,
        epsilon = 0.0
    );
    assert_abs_diff_eq!(
        first.polar_motion_x().unwrap().as_arcseconds(),
        -0.012_700,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        first.polar_motion_y().unwrap().as_arcseconds(),
        0.213_000,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        first.ut1_minus_utc().unwrap().as_seconds(),
        0.032_633_8,
        epsilon = 5.0e-10
    );
    assert_abs_diff_eq!(
        first.excess_length_of_day().unwrap().as_milliseconds(),
        1.723,
        epsilon = 5.0e-7
    );
    assert!(first.polar_motion_rate_x().is_some());
    assert!(first.polar_motion_rate_y().is_some());
    assert_abs_diff_eq!(
        first
            .polar_motion_x_standard_uncertainty()
            .unwrap()
            .value()
            .as_degrees()
            * 3_600.0,
        0.030,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        first
            .ut1_minus_utc_standard_uncertainty()
            .unwrap()
            .value()
            .as_seconds_f64(),
        0.002,
        epsilon = 5.0e-10
    );
    assert_abs_diff_eq!(
        first
            .celestial_pole_offset_x_standard_uncertainty()
            .unwrap()
            .value()
            .as_degrees()
            * 3_600.0,
        0.004_774,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        first
            .excess_length_of_day_standard_uncertainty()
            .unwrap()
            .value()
            .as_seconds_f64(),
        0.001_4,
        epsilon = 5.0e-10
    );
    assert!(
        first
            .polar_motion_rate_x_standard_uncertainty()
            .unwrap()
            .is_zero()
    );
    let time = TimeContext::builtin();
    let samples = data
        .try_samples_in(
            &time,
            ModifiedJulianDate::<Utc>::from_parts(41_317.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(61_228.0, 0.0).unwrap(),
            EarthOrientationAcceptance::FinalOnly,
        )
        .unwrap();
    assert_eq!(samples.len(), 19_912);
}

#[test]
fn finals_parser_prefers_bulletin_b_and_preserves_prediction_gaps() {
    let data = IersFinals2000A::parse(FINALS_2000_A).unwrap();
    assert_eq!(data.product(), EarthOrientationProduct::IersFinals2000A);
    assert_eq!(data.records().len(), 19_948);

    let first = data.records()[0];
    assert_abs_diff_eq!(
        first.polar_motion_x().unwrap().as_arcseconds(),
        0.143,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        first.ut1_minus_utc().unwrap().as_seconds(),
        0.807_5,
        epsilon = 5.0e-10
    );
    assert_abs_diff_eq!(
        first
            .celestial_pole_offset_x()
            .unwrap()
            .as_milliarcseconds(),
        -18.637,
        epsilon = 1.0e-12
    );
    assert!(first.polar_motion_x_standard_uncertainty().is_none());
    assert!(first.ut1_minus_utc_standard_uncertainty().is_none());
    assert!(
        first
            .celestial_pole_offset_x_standard_uncertainty()
            .is_none()
    );

    let last = data.records()[data.records().len() - 1];
    assert_abs_diff_eq!(
        last.modified_julian_date().as_f64_lossy(),
        61_631.0,
        epsilon = 0.0
    );
    assert!(last.polar_motion_x().is_some());
    assert!(last.polar_motion_y().is_some());
    assert!(last.ut1_minus_utc().is_some());
    assert!(last.excess_length_of_day().is_none());
    assert!(last.celestial_pole_offset_x().is_none());
    assert!(last.celestial_pole_offset_y().is_none());
    let time = TimeContext::builtin();
    let observed_mjd = ModifiedJulianDate::<Utc>::from_parts(61_249.0, 0.0).unwrap();
    assert!(matches!(
        data.try_samples_in(
            &time,
            observed_mjd,
            observed_mjd,
            EarthOrientationAcceptance::FinalOnly,
        ),
        Err(Error::EarthOrientationValueRejected {
            field: "polar motion xp/yp",
            provenance: "observed",
            acceptance: "FinalOnly",
            ..
        })
    ));
    assert_eq!(
        data.try_samples_in(
            &time,
            observed_mjd,
            observed_mjd,
            EarthOrientationAcceptance::ObservedOrFinal,
        )
        .unwrap()
        .len(),
        1
    );

    let predicted_mjd = ModifiedJulianDate::<Utc>::from_parts(61_250.0, 0.0).unwrap();
    assert!(matches!(
        data.try_samples_in(
            &time,
            predicted_mjd,
            predicted_mjd,
            EarthOrientationAcceptance::ObservedOrFinal,
        ),
        Err(Error::EarthOrientationValueRejected {
            field: "celestial-pole offsets dX/dY",
            provenance: "predicted",
            acceptance: "ObservedOrFinal",
            ..
        })
    ));
    assert_eq!(
        data.try_samples_in(
            &time,
            predicted_mjd,
            predicted_mjd,
            EarthOrientationAcceptance::IncludePredicted,
        )
        .unwrap()
        .len(),
        1
    );

    let incomplete_mjd = ModifiedJulianDate::<Utc>::from_parts(61_258.0, 0.0).unwrap();
    assert!(matches!(
        data.try_samples_in(
            &time,
            incomplete_mjd,
            incomplete_mjd,
            EarthOrientationAcceptance::IncludePredicted,
        ),
        Err(Error::MissingEarthOrientationValue {
            field: "length of day",
            ..
        })
    ));

    let attitude_samples = data
        .try_earth_attitude_samples_in(
            &time,
            incomplete_mjd,
            incomplete_mjd,
            EarthOrientationAcceptance::IncludePredicted,
        )
        .unwrap();
    let attitude_expires = attitude_samples[0]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let attitude_table = EarthAttitudeTable::new(
        &attitude_samples,
        "finals attitude without LOD",
        attitude_expires,
    )
    .unwrap();
    let attitude_time = time.with_earth_attitude(attitude_table);
    let attitude = attitude_time
        .earth_attitude_at(attitude_samples[0].epoch())
        .unwrap();
    assert_eq!(
        attitude.ut1_minus_utc(),
        attitude_samples[0].ut1_minus_utc()
    );
    let solution = Frames::new(&attitude_time)
        .earth_attitude_at(attitude_samples[0].epoch())
        .unwrap();
    let resolved_attitude = solution.earth_attitude();
    assert_eq!(resolved_attitude.epoch(), attitude.epoch());
    assert_eq!(
        resolved_attitude.ut1_minus_utc(),
        Some(attitude.ut1_minus_utc())
    );
    assert_eq!(
        resolved_attitude.polar_motion_x(),
        attitude.polar_motion_x()
    );
    assert_eq!(
        resolved_attitude.polar_motion_y(),
        attitude.polar_motion_y()
    );
    assert_eq!(
        resolved_attitude.celestial_pole_offset_x(),
        attitude.celestial_pole_offset_x()
    );
    assert_eq!(
        resolved_attitude.celestial_pole_offset_y(),
        attitude.celestial_pole_offset_y()
    );
    let site = Earth::wgs84()
        .fixed_site(
            "missing-LOD fallback",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(121.458_930).unwrap(),
                GeodeticLatitude::try_from_degrees(31.340_370).unwrap(),
                EllipsoidalHeight::from_metres(15.0).unwrap(),
            ),
        )
        .unwrap();
    let topocentric = site
        .topocentric_frame_with_nominal_rotation_at(
            attitude_samples[0].epoch(),
            &Frames::new(&attitude_time),
        )
        .unwrap();
    assert_eq!(
        topocentric.velocity_model(),
        SiteVelocityModel::IersNominalEarthRotation
    );
    let inertial_speed = topocentric
        .observer_state()
        .velocity()
        .magnitude()
        .unwrap()
        .as_metres_per_second();
    assert!((300.0..400.0).contains(&inertial_speed));
    let gcrs_state = topocentric.observer_state();
    let cirs_position = solution
        .gcrs_to_cirs()
        .apply_vector(gcrs_state.position().position())
        .unwrap()
        .components();
    let cirs_velocity = solution
        .gcrs_to_cirs()
        .apply_vector(gcrs_state.velocity())
        .unwrap()
        .components();
    let observations = solution.earth_attitude();
    let geodetic = site.geodetic_position();
    let mut sofa_pv = [[0.0; 3]; 2];
    sofars::astro::pvtob(
        geodetic.longitude().as_radians(),
        geodetic.latitude().as_radians(),
        geodetic.height().as_metres(),
        observations.polar_motion_x().as_angle().as_radians(),
        observations.polar_motion_y().as_angle().as_radians(),
        solution.tio_locator().as_radians(),
        solution.earth_rotation_angle().as_radians(),
        &mut sofa_pv,
    );
    for axis in 0..3 {
        assert_abs_diff_eq!(
            cirs_position[axis].as_metres(),
            sofa_pv[0][axis],
            epsilon = 2.0e-8
        );
        assert_abs_diff_eq!(
            cirs_velocity[axis].as_metres_per_second(),
            sofa_pv[1][axis],
            epsilon = 1.0e-5
        );
    }
}
#[test]
fn finals_prediction_uncertainties_survive_attitude_interpolation() {
    let data = IersFinals2000A::parse(FINALS_2000_A).unwrap();
    let time = TimeContext::builtin();
    let start = ModifiedJulianDate::<Utc>::from_parts(61_261.0, 0.0).unwrap();
    let end = ModifiedJulianDate::<Utc>::from_parts(61_262.0, 0.0).unwrap();
    let samples = data
        .try_earth_attitude_samples_in(
            &time,
            start,
            end,
            EarthOrientationAcceptance::IncludePredicted,
        )
        .unwrap();
    assert_eq!(samples.len(), 2);
    let expires = samples[1]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let table =
        EarthAttitudeTable::new(&samples, "finals prediction uncertainty", expires).unwrap();
    let time = time.with_earth_attitude(table);
    let midpoint = samples[0]
        .epoch()
        .checked_add(Duration::from_seconds(43_200).unwrap())
        .unwrap();
    let attitude = time.earth_attitude_at(midpoint).unwrap();
    let uncertainties = attitude.standard_uncertainties();

    assert_eq!(
        attitude.standard_uncertainty_origin(),
        Some(UncertaintyOrigin::CorrelationAgnosticLinearInterpolation)
    );
    assert_abs_diff_eq!(
        uncertainties
            .ut1_minus_utc()
            .unwrap()
            .value()
            .as_seconds_f64(),
        0.000_352_45,
        epsilon = 5.0e-10
    );
    assert_abs_diff_eq!(
        uncertainties.polar_motion_x().unwrap().value().as_degrees() * 3_600.0,
        0.001_247,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        uncertainties.polar_motion_y().unwrap().value().as_degrees() * 3_600.0,
        0.001_062_5,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        uncertainties
            .celestial_pole_offset_x()
            .unwrap()
            .value()
            .as_degrees()
            * 3_600_000.0,
        0.128,
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        uncertainties
            .celestial_pole_offset_y()
            .unwrap()
            .value()
            .as_degrees()
            * 3_600_000.0,
        0.160,
        epsilon = 1.0e-12
    );
}

#[test]
fn sidereal_time_uses_current_ut1_without_requiring_length_of_day() {
    let base = TimeContext::builtin();
    let data = IersFinals2000A::parse(FINALS_2000_A).unwrap();
    let samples = data
        .try_earth_rotation_samples_in(
            &base,
            ModifiedJulianDate::<Utc>::from_parts(61_258.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(61_259.0, 0.0).unwrap(),
            EarthOrientationAcceptance::IncludePredicted,
        )
        .unwrap();
    assert_eq!(samples.len(), 2);

    let expires = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2026, 8, 8, 0, 0, 0, 0).unwrap())
        .unwrap();
    let table = EarthRotationTable::new(&samples, "finals2000A current UT1", expires).unwrap();
    let time = base.with_earth_rotation(table);
    let epoch = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2026, 8, 6, 12, 0, 0, 0).unwrap())
        .unwrap();
    let solution = Frames::new(&time).sidereal_time_at(epoch).unwrap();
    let rotation = time.earth_rotation_at(epoch).unwrap();
    assert_abs_diff_eq!(
        rotation.ut1_minus_utc().as_seconds(),
        0.010_615_35,
        epsilon = 5.0e-10
    );

    let tt = JulianDate::<Tt>::from_instant(epoch, &time).unwrap();
    let ut1 = JulianDate::<Ut1>::from_instant(epoch, &time).unwrap();
    let (tt_first, tt_second) = tt.parts();
    let (ut1_first, ut1_second) = ut1.parts();
    assert_abs_diff_eq!(
        solution.earth_rotation_angle().as_radians(),
        sofars::erst::era00(ut1_first, ut1_second),
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        solution.greenwich_mean_sidereal_time().as_radians(),
        sofars::erst::gmst06(ut1_first, ut1_second, tt_first, tt_second),
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        solution.greenwich_apparent_sidereal_time().as_radians(),
        sofars::erst::gst06a(ut1_first, ut1_second, tt_first, tt_second),
        epsilon = 1.0e-15
    );

    let longitude = Longitude::try_from_degrees(116.391).unwrap();
    let expected_local = (solution.greenwich_apparent_sidereal_time().as_radians()
        + longitude.as_radians())
    .rem_euclid(core::f64::consts::TAU);
    assert_abs_diff_eq!(
        solution
            .local_apparent_sidereal_time(longitude)
            .unwrap()
            .as_radians(),
        expected_local,
        epsilon = 1.0e-15
    );
}

#[test]
fn latest_finals_snapshot_supplies_delta_t_through_2026() {
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
    let table =
        EarthRotationTable::new(&samples, "finals2000A full-year 2026 UT1", expires).unwrap();
    let time = base.with_earth_rotation(table);

    for (month, day) in [(2, 17), (8, 12)] {
        let epoch = time
            .resolve(
                DateTime::<Gregorian, Utc>::from_components(2026, month, day, 12, 0, 0, 0).unwrap(),
            )
            .unwrap();
        let delta_t = time.delta_t_at(epoch).unwrap();
        assert!((60.0..80.0).contains(&delta_t.as_seconds()));
    }
}

#[test]
fn c04_parser_rejects_a_calendar_and_mjd_disagreement() {
    let invalid = "1962   1   1   0  37666.00   -0.012700    0.213000   0.0326338    0.000000    0.000000    0.000000    0.000000   0.0017230    0.030000    0.030000   0.0020000    0.004774    0.002000    0.000000    0.000000   0.0014000";
    assert!(matches!(
        IersC04::parse(invalid),
        Err(Error::EarthOrientationMjdMismatch { line: 1, .. })
    ));
}

#[test]
fn complete_gcrs_itrs_chain_matches_sofa_and_round_trips_state() {
    let base = TimeContext::builtin();
    let data = IersC04::parse(C04).unwrap();
    let samples = data
        .try_samples_in(
            &base,
            ModifiedJulianDate::<Utc>::from_parts(41_317.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(61_228.0, 0.0).unwrap(),
            EarthOrientationAcceptance::FinalOnly,
        )
        .unwrap();
    let expires = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2026, 8, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let table =
        EarthOrientationTable::new(&samples, "IERS EOP 20u24 C04 2026-08-06", expires).unwrap();
    let time = base.with_earth_orientation(table);
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2007, 10, 15, 12, 0, 0, 0).unwrap())
        .unwrap();
    let orientation = time.earth_orientation_at(epoch).unwrap();
    let tt = JulianDate::<Tt>::from_instant(epoch, &time).unwrap();
    let ut1 = JulianDate::<Ut1>::from_instant(epoch, &time).unwrap();
    let (tt_first, tt_second) = tt.parts();
    let (ut1_first, ut1_second) = ut1.parts();
    let (x, y, modeled_cio_locator) = sofars::pnp::xys06a(tt_first, tt_second);
    let corrected_x = x + orientation
        .celestial_pole_offset_x()
        .as_angle()
        .as_radians();
    let corrected_y = y + orientation
        .celestial_pole_offset_y()
        .as_angle()
        .as_radians();
    let cio_locator = sofars::pnp::s06(tt_first, tt_second, corrected_x, corrected_y);
    let expected_cirs = sofars::pnp::c2ixys(corrected_x, corrected_y, cio_locator);
    let era = sofars::erst::era00(ut1_first, ut1_second);
    let polar_motion = sofars::pnp::pom00(
        orientation.polar_motion_x().as_angle().as_radians(),
        orientation.polar_motion_y().as_angle().as_radians(),
        sofars::pnp::sp00(tt_first, tt_second),
    );
    let expected_itrs = sofars::pnp::c2tcio(&expected_cirs, era, &polar_motion);

    let frames = Frames::new(&time);
    let gcrs_to_cirs = frames.at::<Gcrs, Cirs, Utc>(epoch).unwrap();
    let tirs_to_itrs = frames.at::<Tirs, Itrs, Utc>(epoch).unwrap();
    let gcrs_to_itrs = frames.at::<Gcrs, Itrs, Utc>(epoch).unwrap();
    let cirs_to_gcrs = frames.at::<Cirs, Gcrs, Utc>(epoch).unwrap();
    let cirs_to_tirs = frames.at::<Cirs, Tirs, Utc>(epoch).unwrap();
    let tirs_to_cirs = frames.at::<Tirs, Cirs, Utc>(epoch).unwrap();
    let itrs_to_tirs = frames.at::<Itrs, Tirs, Utc>(epoch).unwrap();
    let gcrs_to_tirs = frames.at::<Gcrs, Tirs, Utc>(epoch).unwrap();
    let tirs_to_gcrs = frames.at::<Tirs, Gcrs, Utc>(epoch).unwrap();
    let cirs_to_itrs = frames.at::<Cirs, Itrs, Utc>(epoch).unwrap();
    let itrs_to_cirs = frames.at::<Itrs, Cirs, Utc>(epoch).unwrap();
    let itrs_to_gcrs = frames.at::<Itrs, Gcrs, Utc>(epoch).unwrap();
    let assert_inverse = |forward: hyastro::math::Matrix3, inverse: hyastro::math::Matrix3| {
        for row in 0..3 {
            for column in 0..3 {
                assert_abs_diff_eq!(
                    forward.element(row, column).unwrap(),
                    inverse.element(column, row).unwrap(),
                    epsilon = 3.0e-14
                );
            }
        }
    };
    assert_inverse(
        gcrs_to_cirs.rotation().matrix(),
        cirs_to_gcrs.rotation().matrix(),
    );
    assert_inverse(
        cirs_to_tirs.rotation().matrix(),
        tirs_to_cirs.rotation().matrix(),
    );
    assert_inverse(
        tirs_to_itrs.rotation().matrix(),
        itrs_to_tirs.rotation().matrix(),
    );
    assert_inverse(
        gcrs_to_tirs.rotation().matrix(),
        tirs_to_gcrs.rotation().matrix(),
    );
    assert_inverse(
        cirs_to_itrs.rotation().matrix(),
        itrs_to_cirs.rotation().matrix(),
    );
    assert_inverse(
        gcrs_to_itrs.rotation().matrix(),
        itrs_to_gcrs.rotation().matrix(),
    );
    for row in 0..3 {
        for column in 0..3 {
            assert_abs_diff_eq!(
                gcrs_to_cirs
                    .rotation()
                    .matrix()
                    .element(row, column)
                    .unwrap(),
                expected_cirs[row][column],
                epsilon = 2.0e-14
            );
            assert_abs_diff_eq!(
                tirs_to_itrs
                    .rotation()
                    .matrix()
                    .element(row, column)
                    .unwrap(),
                polar_motion[row][column],
                epsilon = 2.0e-14
            );
            assert_abs_diff_eq!(
                gcrs_to_itrs
                    .rotation()
                    .matrix()
                    .element(row, column)
                    .unwrap(),
                expected_itrs[row][column],
                epsilon = 3.0e-14
            );
        }
    }

    let uncorrected = sofars::pnp::c2ixys(x, y, modeled_cio_locator);
    assert!(
        (gcrs_to_cirs.rotation().matrix().element(0, 2).unwrap() - uncorrected[0][2]).abs()
            > 1.0e-13
    );
    let celestial_rate = gcrs_to_cirs.angular_velocity().components();
    assert!(
        celestial_rate
            .iter()
            .any(|value| value.as_radians_per_second().abs() > 0.0)
    );
    let polar_rate = tirs_to_itrs.angular_velocity().components();
    assert!(
        polar_rate
            .iter()
            .any(|value| value.as_radians_per_second().abs() > 0.0)
    );

    let zero_speed = Speed::from_metres_per_second(0.0).unwrap();
    let state = State::<Gcrs, Utc>::new(
        Point3::new(
            Length::from_metres(6_378_137.0).unwrap(),
            Length::from_metres(-1_200_000.0).unwrap(),
            Length::from_metres(2_400_000.0).unwrap(),
        ),
        Vector3::new(zero_speed, zero_speed, zero_speed),
        epoch,
    );
    let itrs: State<Itrs, Utc> = frames.transform(state).unwrap();
    let recovered: State<Gcrs, Utc> = frames.transform(itrs).unwrap();
    let original_position = state.position().position().components();
    let recovered_position = recovered.position().position().components();
    let recovered_velocity = recovered.velocity().components();
    for index in 0..3 {
        assert_abs_diff_eq!(
            recovered_position[index].as_metres(),
            original_position[index].as_metres(),
            epsilon = 2.0e-9
        );
        assert_abs_diff_eq!(
            recovered_velocity[index].as_metres_per_second(),
            0.0,
            epsilon = 2.0e-12
        );
    }

    let one_second = Duration::from_seconds(1).unwrap();
    let before_epoch = epoch.checked_sub(one_second).unwrap();
    let after_epoch = epoch.checked_add(one_second).unwrap();
    let before_state = State::<Gcrs, Utc>::new(state.position(), state.velocity(), before_epoch);
    let after_state = State::<Gcrs, Utc>::new(state.position(), state.velocity(), after_epoch);
    let before: State<Itrs, Utc> = frames.transform(before_state).unwrap();
    let after: State<Itrs, Utc> = frames.transform(after_state).unwrap();
    let before_position = before.position().position().components();
    let after_position = after.position().position().components();
    let velocity = itrs.velocity().components();
    for index in 0..3 {
        let finite_difference =
            (after_position[index].as_metres() - before_position[index].as_metres()) / 2.0;
        assert_abs_diff_eq!(
            velocity[index].as_metres_per_second(),
            finite_difference,
            epsilon = 2.0e-3
        );
    }
}
