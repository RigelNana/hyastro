use std::{
    env,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use hyastro::{
    astro::Astrometry,
    ephem::{CelestialBody, Ephemeris, KernelManifest},
    event::{
        AngularEventSearchOptions, AstrometricMode, ConfigurationCoordinate, ConfigurationKind,
        ConfigurationQuery, DistanceExtremumQuery, Events, ExtremumKind, ExtremumSearchOptions,
        RelativeBodyQuery, StationQuery,
    },
    time::{DateTime, Gregorian, TimeContext, TimeInterval, Utc},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kernel_path = env::args_os().nth(1).map(PathBuf::from).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "usage: cargo run --release --features anise --example planetary_events -- /path/to/de440.bsp",
        )
    })?;
    let time = TimeContext::builtin();
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel_path])?)?;
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let start = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2024, 1, 1, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2025, 1, 1, 0, 0, 0, 0,
    )?)?;
    let interval = TimeInterval::new(start, end)?;
    let mercury_sun = RelativeBodyQuery::new(
        CelestialBody::Mercury,
        CelestialBody::Sun,
        AstrometricMode::Apparent,
    )?;
    let conjunctions = events.configurations_in(
        interval,
        ConfigurationQuery::new(
            mercury_sun,
            ConfigurationKind::Conjunction,
            ConfigurationCoordinate::EclipticLongitude,
        ),
        AngularEventSearchOptions::standard(),
    )?;
    let elongations =
        events.greatest_elongations_in(interval, mercury_sun, ExtremumSearchOptions::standard())?;
    let stations = events.stations_in(
        interval,
        StationQuery::new(CelestialBody::Mercury, AstrometricMode::Apparent),
        AngularEventSearchOptions::standard(),
    )?;
    let lunar_perigees = events.distance_extrema_in(
        interval,
        DistanceExtremumQuery::new(
            CelestialBody::Moon,
            CelestialBody::Earth,
            ExtremumKind::Minimum,
        )?,
        ExtremumSearchOptions::standard(),
    )?;

    println!("2024 Mercury-Sun apparent geocentric events");
    println!("conjunctions: {}", conjunctions.len());
    for event in &elongations {
        let utc = time.represent::<Gregorian, Utc>(event.instant())?;
        let date = utc.date();
        let clock = utc.time();
        println!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z  greatest {:?} elongation  {:>12.9} deg  ±{} ns",
            date.year(),
            date.month(),
            date.day(),
            clock.hour(),
            clock.minute(),
            clock.second(),
            clock.nanosecond(),
            event.side(),
            event.separation().as_degrees(),
            event.evidence().time_uncertainty().as_nanoseconds(),
        );
    }
    for event in &stations {
        let utc = time.represent::<Gregorian, Utc>(event.instant())?;
        let date = utc.date();
        let clock = utc.time();
        println!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z  {:?} station  residual={:.3e} rad/s",
            date.year(),
            date.month(),
            date.day(),
            clock.hour(),
            clock.minute(),
            clock.second(),
            clock.nanosecond(),
            event.kind(),
            event.evidence().residual_rate().as_radians_per_second(),
        );
    }
    println!("2024 geometric lunar perigees: {}", lunar_perigees.len());
    for event in &lunar_perigees {
        let utc = time.represent::<Gregorian, Utc>(event.instant())?;
        let date = utc.date();
        let clock = utc.time();
        println!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z  {:>10.3} km",
            date.year(),
            date.month(),
            date.day(),
            clock.hour(),
            clock.minute(),
            clock.second(),
            clock.nanosecond(),
            event.distance().as_kilometres(),
        );
    }
    Ok(())
}
