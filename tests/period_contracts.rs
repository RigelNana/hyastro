#![cfg(feature = "anise")]

use hyastro::{
    astro::{Astrometry, MoonPhaseAngle, ReceptionLightTimeOptions},
    ephem::{Ephemeris, KernelManifest},
    event::{
        AngularEventSearchOptions, CycleStatistics, EquinoxKind, Events, LunarNode, ModeledCycle,
        TropicalYear,
    },
    frame::EclipticLongitude,
    math::Angle,
    time::{DateTime, Duration, Gregorian, Instant, Tai, TimeContext, TimeInterval, Utc},
};

fn utc(
    time: &TimeContext<'_, hyastro::time::NoEarthOrientation>,
    year: i32,
    month: u8,
    day: u8,
) -> hyastro::time::Instant<Utc> {
    time.resolve(DateTime::<Gregorian, Utc>::from_components(year, month, day, 0, 0, 0, 0).unwrap())
        .unwrap()
}

fn days(seconds: f64) -> f64 {
    seconds / 86_400.0
}

#[test]
fn mean_tropical_year_is_an_epoch_bound_model_not_an_equinox_interval() {
    let time = TimeContext::builtin();
    let epoch = utc(&time, 2000, 1, 1);
    let modeled =
        ModeledCycle::<TropicalYear, _>::from_meeus_mean_solar_longitude(epoch, &time).unwrap();
    assert_eq!(
        modeled.model_identifier(),
        "Meeus J2000 geometric mean solar longitude derivative"
    );
    assert!((days(modeled.duration().as_seconds_f64()) - 365.242_189_6).abs() < 1.0e-7);
    assert!(
        modeled
            .validity()
            .contains(hyastro::time::JulianEpoch::J2000)
    );

    let ancient = Instant::<Tai>::from_tai_nanoseconds_since_1900(
        -2_000 * Duration::NANOSECONDS_PER_JULIAN_YEAR,
    );
    assert!(
        ModeledCycle::<TropicalYear, _>::from_meeus_mean_solar_longitude(ancient, &time).is_err()
    );
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-family BSP"]
fn de440_measures_all_year_and_month_cycle_definitions() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let options = AngularEventSearchOptions::standard();
    let zero = EclipticLongitude::try_from_degrees(0.0).unwrap();

    let years = TimeInterval::new(utc(&time, 2021, 1, 1), utc(&time, 2025, 7, 1)).unwrap();
    let equinox = events
        .equinox_years_in(years, EquinoxKind::March, options)
        .unwrap();
    let sidereal_years = events.sidereal_years_in(years, zero, options).unwrap();
    let anomalistic_years = events.anomalistic_years_in(years, options).unwrap();
    let draconic_years = events
        .draconic_years_in(years, LunarNode::Ascending, options)
        .unwrap();

    assert!(equinox.len() >= 3);
    assert!(sidereal_years.len() >= 3);
    assert!(anomalistic_years.len() >= 3);
    assert!(draconic_years.len() >= 3);
    for cycle in &equinox {
        assert!((365.20..365.30).contains(&days(cycle.duration().as_seconds_f64())));
        assert_eq!(cycle.model().ephemeris().kernel_count(), 1);
        assert!(cycle.start().instant() < cycle.end().instant());
    }
    for cycle in &sidereal_years {
        assert!((365.20..365.32).contains(&days(cycle.duration().as_seconds_f64())));
    }
    for cycle in &anomalistic_years {
        let value = days(cycle.duration().as_seconds_f64());
        assert!((362.0..368.0).contains(&value), "{value}");
    }
    for cycle in &draconic_years {
        let value = days(cycle.duration().as_seconds_f64());
        assert!((345.0..348.0).contains(&value), "{value}");
    }
    let statistics = CycleStatistics::from_cycles(&equinox).unwrap();
    assert_eq!(statistics.count(), equinox.len());
    assert!(statistics.minimum() <= statistics.mean());
    assert!(statistics.mean() <= statistics.maximum());

    let months = TimeInterval::new(utc(&time, 2024, 1, 1), utc(&time, 2024, 8, 1)).unwrap();
    let synodic = events
        .synodic_months_in(
            months,
            MoonPhaseAngle::try_from_degrees(0.0).unwrap(),
            options,
        )
        .unwrap();
    let sidereal = events.sidereal_months_in(months, zero, options).unwrap();
    let tropical = events.tropical_months_in(months, zero, options).unwrap();
    let anomalistic = events.anomalistic_months_in(months, options).unwrap();
    let draconic = events
        .draconic_months_in(months, LunarNode::Ascending, options)
        .unwrap();

    assert!(synodic.len() >= 5);
    assert!(sidereal.len() >= 5);
    assert!(tropical.len() >= 5);
    assert!(anomalistic.len() >= 5);
    assert!(draconic.len() >= 5);
    for cycle in &synodic {
        assert!((29.2..29.9).contains(&days(cycle.duration().as_seconds_f64())));
    }
    for cycle in &sidereal {
        assert!((27.1..27.6).contains(&days(cycle.duration().as_seconds_f64())));
    }
    for cycle in &tropical {
        assert!((27.1..27.6).contains(&days(cycle.duration().as_seconds_f64())));
    }
    for cycle in &anomalistic {
        let value = days(cycle.duration().as_seconds_f64());
        assert!((24.0..30.0).contains(&value), "{value}");
    }
    for cycle in &draconic {
        assert!((27.0..27.5).contains(&days(cycle.duration().as_seconds_f64())));
    }
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-family BSP"]
fn equinox_year_search_does_not_refine_unrelated_solar_terms() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let interval = TimeInterval::new(utc(&time, 2023, 1, 1), utc(&time, 2025, 7, 1)).unwrap();
    let options = AngularEventSearchOptions::new(
        Duration::from_days(7).unwrap(),
        Duration::from_milliseconds(1).unwrap(),
        Angle::from_radians(1.0e-11).unwrap(),
        64,
        250,
        ReceptionLightTimeOptions::standard(),
    )
    .unwrap();

    let years = events
        .equinox_years_in(interval, EquinoxKind::March, options)
        .unwrap();
    assert_eq!(years.len(), 2);
    assert!(years[1].end().evidence().evaluations() <= 250);
}
