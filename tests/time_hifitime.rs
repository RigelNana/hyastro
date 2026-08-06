#![cfg(feature = "hifitime")]

use approx::assert_abs_diff_eq;

use hyastro::time::{
    Date, DateTime, Duration, Error, Gregorian, Hifitime, Instant, JulianDate, LeapKind,
    LeapSecond, LeapSeconds, Tai, Tcb, Tcg, Tdb, TimeContext, Tt, UnixTimestamp, Utc,
};

#[test]
fn hifitime_resolves_utc_and_represents_tai_and_tt() {
    let adapter = TimeContext::builtin();
    let utc_label =
        DateTime::<Gregorian, Utc>::from_components(2017, 1, 1, 0, 0, 0, 123_456_789).unwrap();
    let utc = adapter.resolve(utc_label).unwrap();

    let tai = Instant::<Tai>::from_instant(utc, &adapter).unwrap();
    let tai_label = adapter.represent::<Gregorian, Tai>(tai).unwrap();
    assert_eq!(tai_label.time().second(), 37);
    assert_eq!(tai_label.time().nanosecond(), 123_456_789);

    let tt = Instant::<Tt>::from_instant(utc, &adapter).unwrap();
    let tt_label = adapter.represent::<Gregorian, Tt>(tt).unwrap();
    assert_eq!(tt_label.time().minute(), 1);
    assert_eq!(tt_label.time().second(), 9);
    assert_eq!(tt_label.time().nanosecond(), 307_456_789);
    assert_eq!(
        utc.tai_nanoseconds_since_1900(),
        tt.tai_nanoseconds_since_1900()
    );
}

#[test]
fn hifitime_round_trips_exact_nanoseconds() {
    let adapter = TimeContext::builtin();
    let hifitime = Hifitime::new();
    let source =
        DateTime::<Gregorian, Tai>::from_components(2026, 8, 6, 12, 34, 56, 987_654_321).unwrap();
    let instant = adapter.resolve(source).unwrap();
    assert_eq!(
        adapter.represent::<Gregorian, Tai>(instant).unwrap(),
        source
    );
    assert_eq!(hifitime.import::<Tai>(hifitime.export(instant)), instant);
}

#[test]
fn hifitime_tcg_and_tcb_transformations_match_sofars() {
    let context = TimeContext::builtin();
    let hifitime = Hifitime::new();
    let source =
        DateTime::<Gregorian, Tai>::from_components(2006, 1, 15, 12, 34, 56, 789_123_456).unwrap();
    let tai = context.resolve(source).unwrap();

    let tt = Instant::<Tt>::from_instant(tai, &hifitime).unwrap();
    let tcg = Instant::<Tcg>::from_instant(tai, &hifitime).unwrap();
    let tt_julian = JulianDate::<Tt>::from_instant(tt, &hifitime).unwrap();
    let tcg_julian = JulianDate::<Tcg>::from_instant(tcg, &hifitime).unwrap();
    let (tcg_first, tcg_second) =
        sofars::ts::tttcg(tt_julian.parts().0, tt_julian.parts().1).unwrap();
    let tcg_error = (tcg_julian.parts().0 - tcg_first) + (tcg_julian.parts().1 - tcg_second);
    assert_abs_diff_eq!(tcg_error, 0.0, epsilon = 2.0e-14);
    let tcg_label = hifitime.represent::<Gregorian, Tcg>(tcg).unwrap();
    assert_eq!(hifitime.resolve(tcg_label).unwrap(), tcg);

    let tdb = Instant::<Tdb>::from_instant(tai, &hifitime).unwrap();
    let tcb = Instant::<Tcb>::from_instant(tai, &hifitime).unwrap();
    let tdb_julian = JulianDate::<Tdb>::from_instant(tdb, &hifitime).unwrap();
    let tcb_julian = JulianDate::<Tcb>::from_instant(tcb, &hifitime).unwrap();
    let (tcb_first, tcb_second) =
        sofars::ts::tdbtcb(tdb_julian.parts().0, tdb_julian.parts().1).unwrap();
    let tcb_error = (tcb_julian.parts().0 - tcb_first) + (tcb_julian.parts().1 - tcb_second);
    assert_abs_diff_eq!(tcb_error, 0.0, epsilon = 2.0e-14);
    let tcb_label = hifitime.represent::<Gregorian, Tcb>(tcb).unwrap();
    assert_eq!(hifitime.resolve(tcb_label).unwrap(), tcb);
}

#[test]
fn hifitime_validates_real_utc_leap_second_dates() {
    let adapter = TimeContext::builtin();
    let leap =
        DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 23, 59, 60, 500_000_000).unwrap();
    let instant = adapter.resolve(leap).unwrap();
    assert_eq!(adapter.represent::<Gregorian, Utc>(instant).unwrap(), leap);

    let before = adapter
        .resolve(
            DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 23, 59, 59, 500_000_000)
                .unwrap(),
        )
        .unwrap();
    let after = adapter
        .resolve(
            DateTime::<Gregorian, Utc>::from_components(2017, 1, 1, 0, 0, 0, 500_000_000).unwrap(),
        )
        .unwrap();
    assert_eq!(
        instant.duration_since(before).unwrap(),
        Duration::from_seconds(1).unwrap()
    );
    assert_eq!(
        after.duration_since(instant).unwrap(),
        Duration::from_seconds(1).unwrap()
    );

    let invalid = DateTime::<Gregorian, Utc>::from_components(2016, 12, 30, 23, 59, 60, 0).unwrap();
    assert!(matches!(
        adapter.resolve(invalid),
        Err(Error::InvalidLeapSecondDate { .. })
    ));
}

#[test]
fn custom_leap_seconds_drive_utc_resolution() {
    let entries = [LeapSecond::new(
        Date::<Gregorian>::new(2030, 7, 1).unwrap(),
        LeapKind::Positive,
    )];
    let leaps = LeapSeconds::new(
        "synthetic positive",
        Date::<Gregorian>::new(2030, 1, 1).unwrap(),
        Date::<Gregorian>::new(2031, 1, 1).unwrap(),
        37,
        &entries,
    )
    .unwrap();
    let context = TimeContext::new(leaps);
    let label = DateTime::<Gregorian, Utc>::from_components(2030, 6, 30, 23, 59, 60, 123).unwrap();
    let instant = context.resolve(label).unwrap();
    assert_eq!(context.represent::<Gregorian, Utc>(instant).unwrap(), label);
    assert_eq!(context.leap_seconds().version(), "synthetic positive");
}

#[test]
fn negative_leap_second_removes_the_last_utc_second() {
    let entries = [LeapSecond::new(
        Date::<Gregorian>::new(2030, 7, 1).unwrap(),
        LeapKind::Negative,
    )];
    let context = TimeContext::new(
        LeapSeconds::new(
            "synthetic negative",
            Date::<Gregorian>::new(2030, 1, 1).unwrap(),
            Date::<Gregorian>::new(2031, 1, 1).unwrap(),
            37,
            &entries,
        )
        .unwrap(),
    );
    let before = DateTime::<Gregorian, Utc>::from_components(2030, 6, 30, 23, 59, 58, 0).unwrap();
    let missing = DateTime::<Gregorian, Utc>::from_components(2030, 6, 30, 23, 59, 59, 0).unwrap();
    let after = DateTime::<Gregorian, Utc>::from_components(2030, 7, 1, 0, 0, 0, 0).unwrap();

    let before_instant = context.resolve(before).unwrap();
    let after_instant = context.resolve(after).unwrap();
    assert_eq!(
        after_instant.duration_since(before_instant).unwrap(),
        Duration::from_seconds(1).unwrap()
    );
    assert!(matches!(
        context.resolve(missing),
        Err(Error::NonexistentUtcLabel { .. })
    ));
    assert_eq!(
        context.represent::<Gregorian, Utc>(before_instant).unwrap(),
        before
    );
    assert_eq!(
        context.represent::<Gregorian, Utc>(after_instant).unwrap(),
        after
    );
}

#[test]
fn unix_mapping_is_exact_at_nanosecond_precision() {
    let adapter = Hifitime::new();
    let unix = UnixTimestamp::from_nanoseconds(-1_234_567_890_123_456_789);
    let utc = adapter.resolve_unix(unix);
    assert_eq!(adapter.unix_timestamp(utc), unix);
}
