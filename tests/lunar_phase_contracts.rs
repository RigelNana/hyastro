#![cfg(feature = "anise")]

use hyastro::{
    astro::{Astrometry, MoonPhaseAngle, MoonPhaseBranch, ReceptionLightTimeOptions},
    ephem::{CelestialBody, Ephemeris, KernelManifest},
    event::{AngularEventSearchOptions, Error, Events, MoonPhase},
    math::Angle,
    time::{
        CivilDateTime, Date, DateTime, Duration, FixedUtcOffset, Gregorian, TimeContext,
        TimeInterval, TimeOfDay, Utc,
    },
};

#[test]
fn primary_moon_phases_have_stable_longitude_definitions() {
    let expected = [0.0, 90.0, 180.0, 270.0];
    for (phase, expected_degrees) in MoonPhase::ALL.iter().zip(expected) {
        assert_eq!(
            phase.target_longitude_difference().as_degrees(),
            expected_degrees
        );
        assert!(!phase.english_name().is_empty());
    }
}

#[test]
fn directed_moon_phase_angles_enforce_one_cycle() {
    assert_eq!(
        MoonPhaseAngle::try_from_degrees(45.0).unwrap().as_degrees(),
        45.0
    );
    assert!(MoonPhaseAngle::try_from_degrees(-1.0).is_err());
    assert!(MoonPhaseAngle::try_from_degrees(360.0).is_err());
    assert!(MoonPhaseAngle::try_from_radians(f64::NAN).is_err());
    assert!(
        (MoonPhaseAngle::wrap_degrees(-45.0).unwrap().as_degrees() - 315.0).abs()
            <= f64::EPSILON * 315.0
    );
    assert_eq!(
        MoonPhaseAngle::wrap_degrees(720.0).unwrap().as_degrees(),
        0.0
    );
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-family BSP"]
fn de440_lunar_illumination_matches_horizons() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 25, 7, 0, 0, 0).unwrap())
        .unwrap();
    let illumination = Astrometry::new(&time, &ephemeris)
        .lunar_illumination_at(epoch, ReceptionLightTimeOptions::standard())
        .unwrap();
    let moon = illumination.apparent_moon();
    let sun = illumination.apparent_sun();
    let sunlight = illumination.sunlight_at_moon();

    assert_eq!(illumination.reception_epoch(), epoch);
    assert_eq!(moon.target(), CelestialBody::Moon);
    assert_eq!(moon.reception_light_time().observer(), CelestialBody::Earth);
    assert_eq!(sun.geocentric().target(), CelestialBody::Sun);
    assert_eq!(
        sun.geocentric().reception_light_time().observer(),
        CelestialBody::Earth
    );
    assert_eq!(sunlight.target(), CelestialBody::Sun);
    assert_eq!(sunlight.observer(), CelestialBody::Moon);
    assert_eq!(sunlight.reception_epoch(), moon.emission_epoch());
    assert!(sunlight.emission_epoch() < sunlight.reception_epoch());
    assert_eq!(illumination.branch(), MoonPhaseBranch::Waxing);

    // JPL Horizons, DE441, geocentric observer table at 2024-03-25 07:00 UTC,
    // quantities 10 and 24: Illu%=99.99300 and S-T-O=0.9587 degrees.
    // S-T-O includes down-leg aberration, so it differs from this physical phase
    // angle by the documented few-arcsecond scale.
    assert!((illumination.illuminated_fraction().as_percent() - 99.99300).abs() <= 0.00001);
    assert!((illumination.phase_angle().as_degrees() - 0.9587).abs() <= 0.001);
    assert!((illumination.directed_elongation().as_degrees() - 180.0).abs() <= 0.01);
    assert!(illumination.apparent_separation().as_degrees() > 179.0);
    assert_eq!(
        illumination.illuminated_fraction().as_percent(),
        illumination.illuminated_fraction().as_ratio() * 100.0
    );
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-family BSP"]
fn de440_arbitrary_phase_angle_matches_horizons_and_primary_wrapper() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let astrometry = Astrometry::new(&time, &ephemeris);
    let events = Events::new(astrometry);
    let options = AngularEventSearchOptions::standard();
    let interval = TimeInterval::new(
        time.resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 10, 0, 0, 0, 0).unwrap())
            .unwrap(),
        time.resolve(DateTime::<Gregorian, Utc>::from_components(2024, 4, 10, 0, 0, 0, 0).unwrap())
            .unwrap(),
    )
    .unwrap();
    let target = MoonPhaseAngle::try_from_degrees(45.0).unwrap();
    let found = events
        .moon_phase_angle_in(interval, target, options)
        .unwrap();

    assert_eq!(found.len(), 1);
    let event = found[0];
    assert_eq!(event.target(), target);
    assert_eq!(event.apparent_moon().target(), CelestialBody::Moon);
    assert_eq!(
        event.apparent_sun().geocentric().target(),
        CelestialBody::Sun
    );
    assert_eq!(event.apparent_moon().reception_epoch(), event.instant());
    assert_eq!(event.apparent_sun().reception_epoch(), event.instant());
    assert!(
        event.evidence().residual().as_radians().abs() <= options.angular_tolerance().as_radians()
    );
    assert!(event.evidence().time_uncertainty() <= Duration::from_microseconds(500).unwrap());
    assert!(event.evidence().evaluations() >= 4);
    assert_eq!(event.evidence().evaluations() % 2, 0);

    // JPL Horizons DE441 geocentric observer tables, quantity 31, give apparent
    // ecliptic-of-date longitudes 38.5055006° for the Moon and 353.5054999° for
    // the Sun at 2024-03-13 14:32:28.766 UTC: a directed difference of 45.0000007°.
    let horizons_epoch = time
        .resolve(
            DateTime::<Gregorian, Utc>::from_components(2024, 3, 13, 14, 32, 28, 766_000_000)
                .unwrap(),
        )
        .unwrap();
    assert!(
        event
            .instant()
            .duration_since(horizons_epoch)
            .unwrap()
            .checked_abs()
            .unwrap()
            <= Duration::from_milliseconds(1).unwrap()
    );
    assert!((event.longitude_difference().as_degrees() - 45.0).abs() <= 1.0e-8);

    let quarter_interval = TimeInterval::new(
        time.resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 16, 0, 0, 0, 0).unwrap())
            .unwrap(),
        time.resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 18, 0, 0, 0, 0).unwrap())
            .unwrap(),
    )
    .unwrap();
    let primary = events.moon_phases_in(quarter_interval, options).unwrap();
    let arbitrary = events
        .moon_phase_angle_in(
            quarter_interval,
            MoonPhaseAngle::try_from_degrees(90.0).unwrap(),
            options,
        )
        .unwrap();
    assert_eq!(primary.len(), 1);
    assert_eq!(primary[0].phase(), MoonPhase::FirstQuarter);
    assert_eq!(arbitrary.len(), 1);
    assert_eq!(primary[0].angle_event(), arbitrary[0]);

    let endpoint_interval = TimeInterval::new(
        event.instant(),
        event
            .instant()
            .checked_add(Duration::from_days(1).unwrap())
            .unwrap(),
    )
    .unwrap();
    let endpoint_events = events
        .moon_phase_angle_in(endpoint_interval, target, options)
        .unwrap();
    assert_eq!(endpoint_events.len(), 1);
    assert_eq!(endpoint_events[0].instant(), event.instant());

    let limited = AngularEventSearchOptions::new(
        Duration::from_days(1).unwrap(),
        Duration::from_milliseconds(1).unwrap(),
        Angle::from_radians(1.0e-11).unwrap(),
        64,
        1,
        ReceptionLightTimeOptions::standard(),
    )
    .unwrap();
    assert!(matches!(
        events.moon_phase_angle_in(endpoint_interval, target, limited),
        Err(Error::EvaluationLimitExceeded { maximum: 1 })
    ));
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-family BSP"]
fn de440_2024_primary_moon_phases_match_usno_minutes() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let offset = FixedUtcOffset::UTC;
    let options = AngularEventSearchOptions::standard();
    let year = events.moon_phase_year(2024, offset, options).unwrap();

    // USNO Astronomical Applications API v4.0.1, moon/phases/year?year=2024.
    // Values are Universal Time published to the nearest minute.
    let published = [
        (MoonPhase::LastQuarter, 1, 4, 3, 30),
        (MoonPhase::NewMoon, 1, 11, 11, 57),
        (MoonPhase::FirstQuarter, 1, 18, 3, 52),
        (MoonPhase::FullMoon, 1, 25, 17, 54),
        (MoonPhase::LastQuarter, 2, 2, 23, 18),
        (MoonPhase::NewMoon, 2, 9, 22, 59),
        (MoonPhase::FirstQuarter, 2, 16, 15, 1),
        (MoonPhase::FullMoon, 2, 24, 12, 30),
        (MoonPhase::LastQuarter, 3, 3, 15, 23),
        (MoonPhase::NewMoon, 3, 10, 9, 0),
        (MoonPhase::FirstQuarter, 3, 17, 4, 11),
        (MoonPhase::FullMoon, 3, 25, 7, 0),
        (MoonPhase::LastQuarter, 4, 2, 3, 15),
        (MoonPhase::NewMoon, 4, 8, 18, 21),
        (MoonPhase::FirstQuarter, 4, 15, 19, 13),
        (MoonPhase::FullMoon, 4, 23, 23, 49),
        (MoonPhase::LastQuarter, 5, 1, 11, 27),
        (MoonPhase::NewMoon, 5, 8, 3, 22),
        (MoonPhase::FirstQuarter, 5, 15, 11, 48),
        (MoonPhase::FullMoon, 5, 23, 13, 53),
        (MoonPhase::LastQuarter, 5, 30, 17, 13),
        (MoonPhase::NewMoon, 6, 6, 12, 38),
        (MoonPhase::FirstQuarter, 6, 14, 5, 18),
        (MoonPhase::FullMoon, 6, 22, 1, 8),
        (MoonPhase::LastQuarter, 6, 28, 21, 53),
        (MoonPhase::NewMoon, 7, 5, 22, 57),
        (MoonPhase::FirstQuarter, 7, 13, 22, 49),
        (MoonPhase::FullMoon, 7, 21, 10, 17),
        (MoonPhase::LastQuarter, 7, 28, 2, 51),
        (MoonPhase::NewMoon, 8, 4, 11, 13),
        (MoonPhase::FirstQuarter, 8, 12, 15, 19),
        (MoonPhase::FullMoon, 8, 19, 18, 26),
        (MoonPhase::LastQuarter, 8, 26, 9, 26),
        (MoonPhase::NewMoon, 9, 3, 1, 55),
        (MoonPhase::FirstQuarter, 9, 11, 6, 5),
        (MoonPhase::FullMoon, 9, 18, 2, 34),
        (MoonPhase::LastQuarter, 9, 24, 18, 50),
        (MoonPhase::NewMoon, 10, 2, 18, 49),
        (MoonPhase::FirstQuarter, 10, 10, 18, 55),
        (MoonPhase::FullMoon, 10, 17, 11, 26),
        (MoonPhase::LastQuarter, 10, 24, 8, 3),
        (MoonPhase::NewMoon, 11, 1, 12, 47),
        (MoonPhase::FirstQuarter, 11, 9, 5, 55),
        (MoonPhase::FullMoon, 11, 15, 21, 28),
        (MoonPhase::LastQuarter, 11, 23, 1, 28),
        (MoonPhase::NewMoon, 12, 1, 6, 21),
        (MoonPhase::FirstQuarter, 12, 8, 15, 26),
        (MoonPhase::FullMoon, 12, 15, 9, 2),
        (MoonPhase::LastQuarter, 12, 22, 22, 18),
        (MoonPhase::NewMoon, 12, 30, 22, 27),
    ];

    assert_eq!(year.year(), 2024);
    assert_eq!(year.offset(), offset);
    assert_eq!(year.entries().len(), published.len());
    let mut maximum_published_difference = 0.0_f64;
    for (entry, &(expected_phase, month, day, hour, minute)) in
        year.entries().iter().zip(published.iter())
    {
        let event = entry.event();
        assert_eq!(event.phase(), expected_phase);
        assert_eq!(event.apparent_moon().target(), CelestialBody::Moon);
        assert_eq!(
            event.apparent_sun().geocentric().target(),
            CelestialBody::Sun
        );
        assert_eq!(event.apparent_moon().reception_epoch(), event.instant());
        assert_eq!(event.apparent_sun().reception_epoch(), event.instant());
        assert_eq!(entry.local_time().date().year(), 2024);
        assert!(
            event.evidence().residual().as_radians().abs()
                <= options.angular_tolerance().as_radians()
        );
        assert!(event.evidence().time_uncertainty() <= Duration::from_microseconds(500).unwrap());
        assert!(event.evidence().iterations() <= options.max_refinement_iterations());
        assert!(event.evidence().evaluations() >= 4);
        assert_eq!(event.evidence().evaluations() % 2, 0);

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
        assert!(difference <= 120.5);
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
    let endpoint_events = events.moon_phases_in(endpoint_interval, options).unwrap();
    assert_eq!(endpoint_events.len(), 1);
    assert_eq!(endpoint_events[0].phase(), first.phase());
    assert_eq!(endpoint_events[0].instant(), first.instant());

    let limited = AngularEventSearchOptions::new(
        Duration::from_days(1).unwrap(),
        Duration::from_milliseconds(1).unwrap(),
        Angle::from_radians(1.0e-12).unwrap(),
        64,
        1,
        ReceptionLightTimeOptions::standard(),
    )
    .unwrap();
    assert!(matches!(
        events.moon_phases_in(endpoint_interval, limited),
        Err(Error::EvaluationLimitExceeded { maximum: 1 })
    ));

    println!(
        "50 USNO minute-rounded primary phases matched; maximum difference={maximum_published_difference:.6} s"
    );
}
