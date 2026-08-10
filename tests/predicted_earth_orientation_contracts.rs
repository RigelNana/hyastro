use approx::assert_abs_diff_eq;
use hyastro::{
    frame::Frames,
    time::{
        DateTime, DeltaTEstimate, DeltaTModel, Duration, EarthAttitudeOffsetModel, Error,
        Gregorian, Instant, JulianDate, PredictedEarthOrientation, PredictionDisposition,
        TimeContext, TimeInterval, Tt, Ut1, Utc,
    },
    uncertainty::StandardUncertainty,
};

fn tt(time: &TimeContext<'_>, year: i32, month: u8, day: u8) -> hyastro::time::Instant<Tt> {
    time.resolve(DateTime::<Gregorian, Tt>::from_components(year, month, day, 0, 0, 0, 0).unwrap())
        .unwrap()
}

#[test]
fn constant_prediction_derives_ut1_without_a_future_utc_scenario() {
    let base = TimeContext::builtin();
    let validity = TimeInterval::new(tt(&base, 2035, 9, 1), tt(&base, 2035, 9, 3)).unwrap();
    let uncertainty = StandardUncertainty::new(Duration::from_seconds_f64(0.8).unwrap()).unwrap();
    let delta_t = DeltaTModel::constant(
        "NASA 2035 path-table Delta T 80.6 s",
        validity,
        DeltaTEstimate::new(Duration::from_seconds_f64(80.6).unwrap(), Some(uncertainty)),
    )
    .unwrap();
    let prediction = PredictedEarthOrientation::new(
        "2035 eclipse prediction scenario",
        delta_t,
        EarthAttitudeOffsetModel::assumed_zero(),
    )
    .unwrap();
    let time = base.with_predicted_earth_orientation(prediction);
    let epoch = tt(&base, 2035, 9, 2);

    let resolved = time.earth_attitude_state_at(epoch).unwrap();
    assert_abs_diff_eq!(resolved.delta_t().as_seconds(), 80.6, epsilon = 1.0e-9);
    assert_eq!(resolved.delta_t_standard_uncertainty(), Some(uncertainty));
    assert_eq!(resolved.ut1_minus_utc(), None);
    assert_eq!(resolved.polar_motion_x().as_arcseconds(), 0.0);
    assert_eq!(resolved.polar_motion_y().as_arcseconds(), 0.0);
    assert_eq!(resolved.celestial_pole_offset_x().as_milliarcseconds(), 0.0);
    assert_eq!(resolved.celestial_pole_offset_y().as_milliarcseconds(), 0.0);

    let terrestrial = JulianDate::<Tt>::from_instant(epoch, &time).unwrap();
    let universal = JulianDate::<Ut1>::from_instant(epoch, &time).unwrap();
    let expected = terrestrial
        .checked_add_duration(Duration::from_seconds_f64(-80.6).unwrap())
        .unwrap();
    let (actual_first, actual_second) = universal.parts();
    let (expected_first, expected_second) = expected.parts();
    assert_eq!(actual_first, expected_first);
    assert_abs_diff_eq!(actual_second, expected_second, epsilon = 1.0e-15);

    assert!(matches!(
        Instant::<Utc>::from_instant(epoch, &time),
        Err(Error::LeapSecondsExpired { .. })
    ));

    let solution = Frames::new(&time).earth_attitude_at(epoch).unwrap();
    assert_eq!(solution.earth_attitude(), resolved);
    assert_abs_diff_eq!(
        solution.universal_time().as_f64_lossy(),
        universal.as_f64_lossy(),
        epsilon = 1.0e-15
    );

    let provenance = time.earth_attitude_provenance();
    assert_eq!(provenance.source(), "2035 eclipse prediction scenario");
    assert!(provenance.is_predicted());
    assert_eq!(
        provenance.delta_t_model(),
        Some("NASA 2035 path-table Delta T 80.6 s")
    );
    assert_eq!(
        provenance.delta_t_disposition(),
        Some(PredictionDisposition::Assumed)
    );
    assert_eq!(
        provenance.offset_disposition(),
        Some(PredictionDisposition::Assumed)
    );
}

#[test]
fn espenak_meeus_prediction_is_bounded_and_does_not_invent_uncertainty() {
    let base = TimeContext::builtin();
    let model = DeltaTModel::espenak_meeus_2006().unwrap();
    let prediction = PredictedEarthOrientation::new(
        "Espenak-Meeus scenario",
        model,
        EarthAttitudeOffsetModel::assumed_zero(),
    )
    .unwrap();
    let time = base.with_predicted_earth_orientation(prediction);
    let epoch = tt(&base, 2035, 9, 2);

    let state = time.earth_attitude_state_at(epoch).unwrap();
    assert_abs_diff_eq!(
        state.delta_t().as_seconds(),
        81.550_604_203,
        epsilon = 1.0e-9
    );
    assert_eq!(state.delta_t_standard_uncertainty(), None);

    let outside = tt(&base, 3001, 1, 1);
    assert!(matches!(
        time.earth_attitude_state_at(outside),
        Err(Error::EarthOrientationPredictionOutsideValidity { .. })
    ));
}
