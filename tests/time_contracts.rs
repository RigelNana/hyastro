use approx::assert_abs_diff_eq;
use hyastro::time::{
    CalendarMonths, CalendarSpan, CalendarYears, CivilDateTime, Date, DateTime, Duration, Error,
    FixedUtcOffset, Gps, Gregorian, Instant, InvalidDayPolicy, Julian, JulianDate, JulianEpoch,
    LeapKind, LeapSecond, LeapSeconds, Tai, TimeContext, TimeInterval, TimeOfDay, Tt, Utc, Weekday,
};
use proptest::prelude::*;

#[test]
fn gregorian_j2000_has_known_day_number_and_weekday() {
    let date = Date::<Gregorian>::new(2000, 1, 1).unwrap();
    assert_eq!(date.to_julian_day_number().value(), 2_451_545);
    assert_eq!(date.weekday(), Weekday::Saturday);
    assert_eq!(date.ordinal(), 1);
}

#[test]
fn gregorian_and_julian_dates_name_the_same_day_explicitly() {
    let gregorian = Date::<Gregorian>::new(1582, 10, 15).unwrap();
    let julian: Date<Julian> = gregorian.convert().unwrap();
    assert_eq!((julian.year(), julian.month(), julian.day()), (1582, 10, 5));
    assert_eq!(julian.convert::<Gregorian>().unwrap(), gregorian);
}

#[test]
fn calendar_validation_uses_astronomical_year_numbering() {
    assert!(Date::<Gregorian>::new(0, 2, 29).is_ok());
    assert!(Date::<Gregorian>::new(1900, 2, 29).is_err());
    assert!(Date::<Gregorian>::new(2000, 2, 29).is_ok());
    assert!(Date::<Julian>::new(1900, 2, 29).is_ok());
}

proptest! {
    #[test]
    fn gregorian_day_number_round_trips(year in -4_000_i32..4_000, month in 1_u8..=12) {
        let first = Date::<Gregorian>::new(year, month, 1).unwrap();
        let restored = Date::<Gregorian>::from_julian_day_number(first.to_julian_day_number()).unwrap();
        prop_assert_eq!(restored, first);
    }
}

#[test]
fn utc_leap_second_is_a_distinct_label_requiring_context() {
    let date = Date::<Gregorian>::new(2016, 12, 31).unwrap();
    let utc = DateTime::<Gregorian, Utc>::leap_second_label(date, 123).unwrap();
    assert!(utc.time().is_leap_second());
    assert_eq!(utc.time().nanoseconds_since_midnight(), 86_400_000_000_123);
    assert!(matches!(
        DateTime::<Gregorian, Tai>::new(date, utc.time()),
        Err(Error::LeapSecondNotRepresentable { .. })
    ));
    assert!(matches!(
        JulianDate::<Utc>::from_datetime(utc),
        Err(Error::ContextRequired { .. })
    ));
}

#[test]
fn bundled_leap_seconds_expose_version_coverage_and_exact_offsets() {
    let leaps = LeapSeconds::builtin();
    let (valid_from, expires) = leaps.coverage();
    assert_eq!(leaps.version(), "IERS Bulletin C 72");
    assert_eq!(valid_from, Date::<Gregorian>::new(1972, 1, 1).unwrap());
    assert_eq!(expires, Date::<Gregorian>::new(2027, 6, 28).unwrap());
    assert!(
        leaps
            .is_leap(Date::<Gregorian>::new(2016, 12, 31).unwrap())
            .unwrap()
    );

    let leap_start = Instant::<Tai>::from_tai_nanoseconds_since_1900(
        3_692_217_636_i128 * Duration::NANOSECONDS_PER_SECOND,
    );
    let next_midnight = leap_start
        .checked_add(Duration::from_seconds(1).unwrap())
        .unwrap();
    assert_eq!(
        leaps.offset(leap_start).unwrap(),
        Duration::from_seconds(36).unwrap()
    );
    assert_eq!(
        leaps.offset(next_midnight).unwrap(),
        Duration::from_seconds(37).unwrap()
    );
    let label =
        DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 23, 59, 60, 500_000_000).unwrap();
    let instant = leaps.resolve(label).unwrap();
    assert_eq!(leaps.represent::<Gregorian>(instant).unwrap(), label);

    assert!(matches!(
        leaps.is_leap(expires),
        Err(Error::LeapSecondsExpired { .. })
    ));
}

#[test]
fn core_context_resolves_utc_tai_tt_and_gps_exactly() {
    let context = TimeContext::builtin();
    let utc_label =
        DateTime::<Gregorian, Utc>::from_components(2017, 1, 1, 0, 0, 0, 123_456_789).unwrap();
    let utc = context.resolve(utc_label).unwrap();

    let tai = Instant::<Tai>::from_instant(utc, &context).unwrap();
    let tai_label = context.represent::<Gregorian, Tai>(tai).unwrap();
    assert_eq!(
        (
            tai_label.time().hour(),
            tai_label.time().minute(),
            tai_label.time().second()
        ),
        (0, 0, 37)
    );
    assert_eq!(tai_label.time().nanosecond(), 123_456_789);

    let tt = Instant::<Tt>::from_instant(utc, &context).unwrap();
    let tt_label = context.represent::<Gregorian, Tt>(tt).unwrap();
    assert_eq!(
        (
            tt_label.time().hour(),
            tt_label.time().minute(),
            tt_label.time().second()
        ),
        (0, 1, 9)
    );
    assert_eq!(tt_label.time().nanosecond(), 307_456_789);

    let gps = Instant::<Gps>::from_instant(utc, &context).unwrap();
    let gps_label = context.represent::<Gregorian, Gps>(gps).unwrap();
    assert_eq!(
        (
            gps_label.time().hour(),
            gps_label.time().minute(),
            gps_label.time().second()
        ),
        (0, 0, 18)
    );
    assert_eq!(gps_label.time().nanosecond(), 123_456_789);

    assert_eq!(context.resolve(tai_label).unwrap(), tai);
    assert_eq!(context.resolve(tt_label).unwrap(), tt);
    assert_eq!(context.resolve(gps_label).unwrap(), gps);
    assert_eq!(
        utc.tai_nanoseconds_since_1900(),
        gps.tai_nanoseconds_since_1900()
    );
}

#[test]
fn core_tt_conversion_matches_sofars_two_part_julian_date() {
    let context = TimeContext::builtin();
    let tai_label =
        DateTime::<Gregorian, Tai>::from_components(2006, 1, 15, 12, 34, 56, 789_123_456).unwrap();
    let tai = context.resolve(tai_label).unwrap();
    let tai_julian = JulianDate::<Tai>::from_instant(tai, &context).unwrap();
    let (expected_first, expected_second) =
        sofars::ts::taitt(tai_julian.parts().0, tai_julian.parts().1).unwrap();

    let tt = Instant::<Tt>::from_instant(tai, &context).unwrap();
    let actual = JulianDate::<Tt>::from_instant(tt, &context).unwrap();
    assert_abs_diff_eq!(actual.parts().0, expected_first, epsilon = 0.0);
    assert_abs_diff_eq!(actual.parts().1, expected_second, epsilon = 1.0e-15);

    let (round_trip_first, round_trip_second) =
        sofars::ts::tttai(actual.parts().0, actual.parts().1).unwrap();
    assert_abs_diff_eq!(round_trip_first, tai_julian.parts().0, epsilon = 0.0);
    assert_abs_diff_eq!(round_trip_second, tai_julian.parts().1, epsilon = 1.0e-15);
}
#[test]
fn leap_seconds_reject_unordered_events() {
    let entries = [
        LeapSecond::new(
            Date::<Gregorian>::new(2017, 1, 1).unwrap(),
            LeapKind::Positive,
        ),
        LeapSecond::new(
            Date::<Gregorian>::new(2015, 7, 1).unwrap(),
            LeapKind::Positive,
        ),
    ];
    let result = LeapSeconds::new(
        "invalid",
        Date::<Gregorian>::new(1972, 1, 1).unwrap(),
        Date::<Gregorian>::new(2027, 1, 1).unwrap(),
        10,
        &entries,
    );
    assert!(matches!(
        result,
        Err(Error::InvalidLeapSecond { index: 1, .. })
    ));
}

#[test]
fn j2000_datetime_round_trips_through_two_part_julian_date() {
    let datetime = DateTime::<Gregorian, Tt>::from_components(2000, 1, 1, 12, 0, 0, 0).unwrap();
    let julian = JulianDate::from_datetime(datetime).unwrap();
    assert_abs_diff_eq!(julian.as_f64_lossy(), 2_451_545.0, epsilon = 0.0);
    assert_eq!(julian.to_datetime::<Gregorian>().unwrap(), datetime);

    let modified = julian.to_modified().unwrap();
    assert_abs_diff_eq!(modified.as_f64_lossy(), 51_544.5, epsilon = 0.0);
    assert_abs_diff_eq!(
        modified.to_julian().unwrap().as_f64_lossy(),
        2_451_545.0,
        epsilon = 0.0
    );
}

#[test]
fn two_part_julian_date_preserves_nanosecond_increment() {
    let start = JulianDate::<Tt>::from_j2000_offset_days(0.0).unwrap();
    let later = start
        .checked_add_duration(Duration::from_nanoseconds(1))
        .unwrap();
    assert_eq!(
        later.duration_since_rounded(start).unwrap(),
        Duration::from_nanoseconds(1)
    );
}

#[test]
fn julian_epoch_has_typed_tt_conversion() {
    let j2016 = JulianEpoch::J2016;
    let date = j2016.to_tt().unwrap();
    assert_abs_diff_eq!(
        date.as_f64_lossy(),
        2_451_545.0 + 16.0 * 365.25,
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        JulianEpoch::from_tt(date).unwrap().value(),
        2016.0,
        epsilon = 1.0e-12
    );
}

#[test]
fn duration_and_instant_arithmetic_remain_integer_exact() {
    let duration = Duration::from_nanoseconds(-1);
    assert_eq!(duration.split_seconds(), (-1, 999_999_999));

    let start = Instant::<Tai>::from_tai_nanoseconds_since_1900(10);
    let later = start.checked_add(Duration::from_nanoseconds(7)).unwrap();
    assert_eq!(later.tai_nanoseconds_since_1900(), 17);
    assert_eq!(
        later.duration_since(start).unwrap(),
        Duration::from_nanoseconds(7)
    );
}

#[test]
fn time_of_day_rejects_silent_component_coercion() {
    assert!(TimeOfDay::new(24, 0, 0, 0).is_err());
    assert!(TimeOfDay::new(12, 30, 60, 0).is_err());
    assert!(DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 12, 30, 60, 0).is_err());
}

#[test]
fn typed_time_interval_is_closed_nonempty_and_exact() {
    let start = Instant::<Tai>::from_tai_nanoseconds_since_1900(10);
    let end = Instant::<Tai>::from_tai_nanoseconds_since_1900(25);
    let interval = TimeInterval::new(start, end).unwrap();
    assert_eq!(interval.start(), start);
    assert_eq!(interval.end(), end);
    assert_eq!(interval.duration(), Duration::from_nanoseconds(15));
    assert!(interval.contains(start));
    assert!(interval.contains(end));
    assert!(!interval.contains(Instant::from_tai_nanoseconds_since_1900(26)));
    assert!(matches!(
        TimeInterval::new(start, start),
        Err(Error::InvalidTimeInterval { .. })
    ));
    assert!(matches!(
        TimeInterval::new(end, start),
        Err(Error::InvalidTimeInterval { .. })
    ));
}

#[test]
fn fixed_offset_civil_labels_round_trip_without_time_zone_rules() {
    let context = TimeContext::builtin();
    let offset = FixedUtcOffset::east_hours(8).unwrap();
    let local = CivilDateTime::<Gregorian>::new(
        Date::new(2024, 1, 1).unwrap(),
        TimeOfDay::new(0, 15, 30, 123_456_789).unwrap(),
        offset,
    )
    .unwrap();
    let instant = context.resolve_fixed(local).unwrap();
    let utc = context.represent::<Gregorian, Utc>(instant).unwrap();
    assert_eq!(
        (utc.date().year(), utc.date().month(), utc.date().day()),
        (2023, 12, 31)
    );
    assert_eq!(
        (
            utc.time().hour(),
            utc.time().minute(),
            utc.time().second(),
            utc.time().nanosecond(),
        ),
        (16, 15, 30, 123_456_789)
    );
    assert_eq!(
        context
            .represent_fixed::<Gregorian, _>(instant, offset)
            .unwrap(),
        local
    );
    assert!(FixedUtcOffset::east_hours(24).is_err());
    assert!(FixedUtcOffset::from_seconds(86_400).is_err());
}

#[test]
fn fixed_offset_labels_reject_the_utc_leap_second_itself() {
    let context = TimeContext::builtin();
    let leap = context
        .resolve(DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 23, 59, 60, 0).unwrap())
        .unwrap();
    assert!(matches!(
        context.represent_fixed::<Gregorian, _>(leap, FixedUtcOffset::east_hours(8).unwrap()),
        Err(Error::FixedOffsetLeapSecondUnsupported)
    ));
}

#[test]
fn calendar_displacements_keep_months_distinct_from_durations() {
    let leap_day = Date::<Gregorian>::new(2024, 2, 29).unwrap();
    assert!(
        leap_day
            .checked_add_years(CalendarYears::new(1), InvalidDayPolicy::Reject)
            .is_err()
    );
    assert_eq!(
        leap_day
            .checked_add_years(CalendarYears::new(1), InvalidDayPolicy::Constrain)
            .unwrap(),
        Date::new(2025, 2, 28).unwrap()
    );

    let january_end = Date::<Gregorian>::new(2024, 1, 31).unwrap();
    assert!(
        january_end
            .checked_add_months(CalendarMonths::new(1), InvalidDayPolicy::Reject)
            .is_err()
    );
    assert_eq!(
        january_end
            .checked_add_months(CalendarMonths::new(1), InvalidDayPolicy::Constrain)
            .unwrap(),
        Date::new(2024, 2, 29).unwrap()
    );
    assert_eq!(
        january_end
            .checked_add_calendar_span(CalendarSpan::new(0, 1, 1), InvalidDayPolicy::Constrain,)
            .unwrap(),
        Date::new(2024, 3, 1).unwrap()
    );
}

#[test]
fn calendar_month_differences_distinguish_boundaries_whole_months_and_remainders() {
    let january_end = Date::<Gregorian>::new(2024, 1, 31).unwrap();
    let february_start = Date::<Gregorian>::new(2024, 2, 1).unwrap();
    assert_eq!(
        february_start.month_boundaries_since(january_end).unwrap(),
        1
    );
    assert_eq!(
        february_start
            .whole_months_since(january_end, InvalidDayPolicy::Constrain)
            .unwrap(),
        0
    );

    let march_end = Date::<Gregorian>::new(2024, 3, 31).unwrap();
    let difference = march_end
        .calendar_difference_since(january_end, InvalidDayPolicy::Constrain)
        .unwrap();
    assert_eq!(difference.whole_months(), 2);
    assert_eq!(difference.remaining_days(), 0);
}
