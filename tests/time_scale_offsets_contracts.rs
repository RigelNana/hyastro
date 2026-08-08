use approx::assert_abs_diff_eq;
use hyastro::time::{
    CelestialPoleOffsetX, CelestialPoleOffsetY, DateTime, Duration, EarthOrientationSample,
    EarthOrientationTable, EarthRotationSample, EarthRotationTable, ExcessLengthOfDay,
    GeocentricTdb, Gregorian, Instant, JulianDate, PolarMotionX, PolarMotionY, Tdb, TimeContext,
    Tt, Ut1MinusUtc, Utc,
};

#[test]
fn delta_t_stays_continuous_across_a_positive_utc_leap_second() {
    let base = TimeContext::builtin();
    let left = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 0, 0, 0, 0).unwrap())
        .unwrap();
    let midpoint = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 12, 0, 0, 0).unwrap())
        .unwrap();
    let right = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2017, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let expires = right.checked_add(Duration::from_days(1).unwrap()).unwrap();
    let samples = [
        EarthRotationSample::new(left, Ut1MinusUtc::from_seconds(-0.4).unwrap()),
        EarthRotationSample::new(right, Ut1MinusUtc::from_seconds(0.6).unwrap()),
    ];
    let table = EarthRotationTable::new(&samples, "synthetic leap transition", expires).unwrap();
    let time = base.with_earth_rotation(table);

    assert_abs_diff_eq!(time.delta_t_at(left).unwrap().as_seconds(), 68.584);
    assert_abs_diff_eq!(time.delta_t_at(midpoint).unwrap().as_seconds(), 68.584);
    assert_abs_diff_eq!(time.delta_t_at(right).unwrap().as_seconds(), 68.584);
}

#[test]
fn complete_eop_context_exposes_the_same_delta_t_capability() {
    let base = TimeContext::builtin();
    let start = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let end = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 2, 0, 0, 0, 0).unwrap())
        .unwrap();
    let epoch = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 12, 0, 0, 0).unwrap())
        .unwrap();
    let expires = end.checked_add(Duration::from_days(1).unwrap()).unwrap();
    let dut1 = Ut1MinusUtc::from_seconds(0.1).unwrap();
    let lod = ExcessLengthOfDay::from_milliseconds(0.0).unwrap();
    let xp = PolarMotionX::from_arcseconds(0.0).unwrap();
    let yp = PolarMotionY::from_arcseconds(0.0).unwrap();
    let dx = CelestialPoleOffsetX::from_milliarcseconds(0.0).unwrap();
    let dy = CelestialPoleOffsetY::from_milliarcseconds(0.0).unwrap();
    let samples = [
        EarthOrientationSample::new(start, dut1, lod, xp, yp, dx, dy),
        EarthOrientationSample::new(end, dut1, lod, xp, yp, dx, dy),
    ];
    let table = EarthOrientationTable::new(&samples, "synthetic complete EOP", expires).unwrap();
    let time = base.with_earth_orientation(table);
    let result = time.delta_t_at(epoch).unwrap();

    assert_eq!(result.epoch(), epoch);
    assert_abs_diff_eq!(result.as_seconds(), 69.084);
    assert_eq!(result.tt_minus_ut1().as_nanoseconds(), 69_084_000_000);
}

#[test]
fn geocentric_tdb_matches_the_full_sofa_analytical_model() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 12, 0, 0, 0).unwrap())
        .unwrap();
    let model = GeocentricTdb::new();
    let solution = model.at(epoch).unwrap();
    let tdb = JulianDate::<Tdb>::from_instant(epoch, &model).unwrap();
    let tt = JulianDate::<Tt>::from_instant(epoch, &time).unwrap();
    let tagged = Instant::<Tdb>::from_instant(epoch, &model).unwrap();

    assert_eq!(
        GeocentricTdb::MODEL,
        "Fairhead-Bretagnon 1990 via SOFA 2023-10-11"
    );
    assert_eq!(solution.epoch(), epoch);
    assert_eq!(solution.terrestrial_time(), tt);
    assert_eq!(solution.barycentric_dynamical_time(), tdb);
    assert_eq!(solution.tdb_minus_tt().as_nanoseconds(), -104_919);
    assert_eq!(
        tagged.tai_nanoseconds_since_1900(),
        epoch.tai_nanoseconds_since_1900()
    );

    let (tdb_first, tdb_second) = tdb.parts();
    let (tt_first, tt_second) = tt.parts();
    let coordinate_difference_seconds =
        ((tdb_first - tt_first) + (tdb_second - tt_second)) * 86_400.0;
    assert_abs_diff_eq!(
        coordinate_difference_seconds,
        solution.tdb_minus_tt_seconds(),
        epsilon = 5.0e-12
    );
}
