#![cfg(feature = "jiff")]

use hyastro::time::{Date, DateTime, Error, Gregorian, Jiff, UnixTimestamp, Utc};

#[test]
fn jiff_gregorian_date_round_trips() {
    let adapter = Jiff::new();
    let source = jiff::civil::Date::new(2024, 2, 29).unwrap();
    let date = adapter.import_date(source).unwrap();
    assert_eq!((date.year(), date.month(), date.day()), (2024, 2, 29));
    assert_eq!(adapter.export_date(date).unwrap(), source);
}

#[test]
fn jiff_civil_datetime_requires_explicit_utc_label_method() {
    let adapter = Jiff::new();
    let source = jiff::civil::DateTime::new(2026, 8, 6, 12, 34, 56, 789_123_456).unwrap();
    let label = adapter.import_utc_label(source).unwrap();
    assert_eq!(label.date(), Date::<Gregorian>::new(2026, 8, 6).unwrap());
    assert_eq!(label.time().nanosecond(), 789_123_456);
    assert_eq!(adapter.export_utc_label(label).unwrap(), source);
}

#[test]
fn jiff_rejects_utc_leap_second_without_clamping() {
    let adapter = Jiff::new();
    let leap = DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 23, 59, 60, 0).unwrap();
    assert!(matches!(
        adapter.export_utc_label(leap),
        Err(Error::LeapSecondNotRepresentable { target: "jiff" })
    ));
}

#[test]
fn jiff_timestamp_round_trips_exact_negative_nanoseconds() {
    let adapter = Jiff::new();
    let source = jiff::Timestamp::from_nanosecond(-1_234_567_890_123_456).unwrap();
    let timestamp = adapter.import_timestamp(source);
    assert_eq!(
        timestamp,
        UnixTimestamp::from_nanoseconds(-1_234_567_890_123_456)
    );
    assert_eq!(adapter.export_timestamp(timestamp).unwrap(), source);
}

#[cfg(feature = "hifitime")]
#[test]
fn jiff_and_hifitime_share_an_exact_unix_seam() {
    use hyastro::time::Hifitime;

    let jiff = Jiff::new();
    let hifitime = Hifitime::new();
    let source = jiff::Timestamp::from_nanosecond(1_700_000_000_123_456_789).unwrap();
    let unix = jiff.import_timestamp(source);
    let utc = hifitime.resolve_unix(unix);
    assert_eq!(
        jiff.export_timestamp(hifitime.unix_timestamp(utc)).unwrap(),
        source
    );
}
