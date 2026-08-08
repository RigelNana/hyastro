#![cfg(feature = "anise")]

use hyastro::{
    astro::{Astrometry, ReceptionLightTimeOptions},
    ephem::{Ephemeris, KernelManifest},
    event::{Error, Events, SolarTerm, SolarTermSearchOptions},
    math::Angle,
    time::{
        CivilDateTime, Date, Duration, FixedUtcOffset, Gregorian, TimeContext, TimeInterval,
        TimeOfDay,
    },
};

#[test]
fn solar_term_search_options_reject_invalid_controls() {
    let angle = Angle::from_radians(1.0e-12).unwrap();
    let light_time = ReceptionLightTimeOptions::standard();
    assert!(matches!(
        SolarTermSearchOptions::new(
            Duration::ZERO,
            Duration::from_milliseconds(1).unwrap(),
            angle,
            64,
            1_024,
            light_time,
        ),
        Err(Error::InvalidSearchDuration { .. })
    ));
    assert!(matches!(
        SolarTermSearchOptions::new(
            Duration::from_days(1).unwrap(),
            Duration::ZERO,
            angle,
            64,
            1_024,
            light_time,
        ),
        Err(Error::InvalidSearchDuration { .. })
    ));
    assert!(matches!(
        SolarTermSearchOptions::new(
            Duration::from_days(1).unwrap(),
            Duration::from_milliseconds(1).unwrap(),
            Angle::from_radians(-1.0).unwrap(),
            64,
            1_024,
            light_time,
        ),
        Err(Error::InvalidLongitudeTolerance { .. })
    ));
    assert!(matches!(
        SolarTermSearchOptions::new(
            Duration::from_days(1).unwrap(),
            Duration::from_milliseconds(1).unwrap(),
            angle,
            0,
            1_024,
            light_time,
        ),
        Err(Error::InvalidSearchLimit { .. })
    ));
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local de440s.bsp"]
fn de440s_2024_solar_terms_match_hong_kong_observatory_minutes() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let astrometry = Astrometry::new(&time, &ephemeris);
    let events = Events::new(astrometry);
    let offset = FixedUtcOffset::east_hours(8).unwrap();
    let options = SolarTermSearchOptions::standard();
    let year = events.solar_term_year(2024, offset, options).unwrap();

    // Hong Kong Observatory 24SolarTerms_2024.xml. Values are Hong Kong
    // Time (UTC+08:00), published to the nearest minute.
    let published = [
        (1, 6, 4, 49),
        (1, 20, 22, 7),
        (2, 4, 16, 27),
        (2, 19, 12, 13),
        (3, 5, 10, 23),
        (3, 20, 11, 6),
        (4, 4, 15, 2),
        (4, 19, 22, 0),
        (5, 5, 8, 10),
        (5, 20, 21, 0),
        (6, 5, 12, 10),
        (6, 21, 4, 51),
        (7, 6, 22, 20),
        (7, 22, 15, 44),
        (8, 7, 8, 9),
        (8, 22, 22, 55),
        (9, 7, 11, 11),
        (9, 22, 20, 44),
        (10, 8, 3, 0),
        (10, 23, 6, 15),
        (11, 7, 6, 20),
        (11, 22, 3, 56),
        (12, 6, 23, 17),
        (12, 21, 17, 21),
    ];

    assert_eq!(year.year(), 2024);
    assert_eq!(year.offset(), offset);
    assert_eq!(year.entries().len(), 24);
    let mut maximum_published_difference = 0.0_f64;
    for ((entry, expected_term), &(month, day, hour, minute)) in year
        .entries()
        .iter()
        .zip(SolarTerm::ALL.iter())
        .zip(published.iter())
    {
        let event = entry.event();
        assert_eq!(event.term(), *expected_term);
        assert_eq!(entry.local_time().date().year(), 2024);
        assert!(
            event.evidence().residual().as_radians().abs()
                <= options.longitude_tolerance().as_radians()
        );
        assert!(event.evidence().time_uncertainty() <= Duration::from_microseconds(500).unwrap());
        assert!(event.evidence().iterations() <= options.max_refinement_iterations());
        assert!(event.evidence().evaluations() > 0);

        let published_label = CivilDateTime::<Gregorian>::new(
            Date::new(2024, month, day).unwrap(),
            TimeOfDay::new(hour, minute, 0, 0).unwrap(),
            offset,
        )
        .unwrap();
        let published_epoch = time.resolve_fixed(published_label).unwrap();
        let difference = event
            .instant()
            .duration_since(published_epoch)
            .unwrap()
            .checked_abs()
            .unwrap()
            .as_seconds_f64();
        maximum_published_difference = maximum_published_difference.max(difference);
        assert!(difference <= 30.5);
    }

    let first = year.entries()[0].event();
    let endpoint_interval = TimeInterval::new(
        first.instant(),
        first
            .instant()
            .checked_add(Duration::from_days(1).unwrap())
            .unwrap(),
    )
    .unwrap();
    let endpoint_events = events.solar_terms_in(endpoint_interval, options).unwrap();
    assert_eq!(endpoint_events.len(), 1);
    assert_eq!(endpoint_events[0].term(), SolarTerm::MinorCold);
    assert_eq!(endpoint_events[0].instant(), first.instant());

    let limited = SolarTermSearchOptions::new(
        Duration::from_days(1).unwrap(),
        Duration::from_milliseconds(1).unwrap(),
        Angle::from_radians(1.0e-12).unwrap(),
        64,
        1,
        ReceptionLightTimeOptions::standard(),
    )
    .unwrap();
    assert!(matches!(
        events.solar_terms_in(endpoint_interval, limited),
        Err(Error::EvaluationLimitExceeded { maximum: 1 })
    ));

    println!(
        "24 HKO minute-rounded terms matched; maximum difference={maximum_published_difference:.6} s"
    );
}
