use approx::assert_abs_diff_eq;
use hyastro::{
    frame::{Cirs, Frames, Gcrs, Itrs, State, Tirs},
    math::{Length, Point3, Speed, Vector3},
    time::{
        DateTime, Duration, EarthOrientationProduct, EarthOrientationTable,
        EarthOrientationValueKind, Error, Gregorian, IersC04, IersFinals2000A, JulianDate,
        ModifiedJulianDate, TimeContext, Tt, Ut1, Utc,
    },
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");
const FINALS_2000_A: &str = include_str!("../data/eop/finals2000a-2026-08-06.all");

#[test]
fn c04_snapshot_preserves_complete_values_rates_uncertainties_and_coverage() {
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
    assert_eq!(
        first.quality().polar_motion(),
        Some(EarthOrientationValueKind::Final)
    );
    assert_eq!(
        first.quality().celestial_pole(),
        Some(EarthOrientationValueKind::Final)
    );
    assert!(first.polar_motion_rate_x().is_some());
    assert!(first.polar_motion_rate_y().is_some());
    assert!(first.uncertainty().polar_motion_x().is_some());
    assert!(first.uncertainty().ut1_minus_utc().is_some());

    let time = TimeContext::builtin();
    assert!(matches!(
        data.try_samples(&time),
        Err(Error::LeapSecondsUnavailable { .. })
    ));
    let samples = data
        .try_samples_in(
            &time,
            ModifiedJulianDate::<Utc>::from_parts(41_317.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(61_228.0, 0.0).unwrap(),
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
    assert_eq!(
        first.quality().polar_motion(),
        Some(EarthOrientationValueKind::Final)
    );
    assert_eq!(
        first.quality().ut1(),
        Some(EarthOrientationValueKind::Final)
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
    assert_eq!(
        last.quality().polar_motion(),
        Some(EarthOrientationValueKind::Predicted)
    );
    assert_eq!(
        last.quality().ut1(),
        Some(EarthOrientationValueKind::Predicted)
    );
    assert_eq!(last.quality().length_of_day(), None);
    assert_eq!(last.quality().celestial_pole(), None);
    let first_without_lod = data
        .records()
        .iter()
        .copied()
        .find(|record| record.modified_julian_date().as_f64_lossy() == 61_258.0)
        .unwrap();
    assert!(matches!(
        first_without_lod.try_into_sample(&TimeContext::builtin()),
        Err(Error::MissingEarthOrientationValue {
            field: "length of day",
            ..
        })
    ));
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
    let (x, y, s) = sofars::pnp::xys06a(tt_first, tt_second);
    let corrected_x = x + orientation
        .celestial_pole_offset_x()
        .as_angle()
        .as_radians();
    let corrected_y = y + orientation
        .celestial_pole_offset_y()
        .as_angle()
        .as_radians();
    let expected_cirs = sofars::pnp::c2ixys(corrected_x, corrected_y, s);
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

    let uncorrected = sofars::pnp::c2ixys(x, y, s);
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
