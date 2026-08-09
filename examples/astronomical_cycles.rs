use std::{
    env,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use hyastro::{
    astro::{Astrometry, MoonPhaseAngle},
    ephem::{Ephemeris, KernelManifest},
    event::{AngularEventSearchOptions, CycleStatistics, EquinoxKind, Events},
    time::{DateTime, Gregorian, TimeContext, TimeInterval, Utc},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kernel_path = env::args_os().nth(1).map(PathBuf::from).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "usage: cargo run --release --features anise --example astronomical_cycles -- /path/to/de440.bsp",
        )
    })?;
    let time = TimeContext::builtin();
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel_path])?)?;
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let start = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2022, 1, 1, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2025, 1, 1, 0, 0, 0, 0,
    )?)?;
    let interval = TimeInterval::new(start, end)?;
    let options = AngularEventSearchOptions::standard();

    let equinox_years = events.equinox_years_in(interval, EquinoxKind::March, options)?;
    let synodic_months =
        events.synodic_months_in(interval, MoonPhaseAngle::try_from_degrees(0.0)?, options)?;
    let equinox_statistics = CycleStatistics::from_cycles(&equinox_years)?;
    let synodic_statistics = CycleStatistics::from_cycles(&synodic_months)?;
    let days = |duration: hyastro::time::Duration| duration.as_seconds_f64() / 86_400.0;

    println!(
        "{} complete March-equinox years: mean {:.9} d, range {:.9}..{:.9} d",
        equinox_statistics.count(),
        days(equinox_statistics.mean()),
        days(equinox_statistics.minimum()),
        days(equinox_statistics.maximum()),
    );
    println!(
        "{} complete new-Moon synodic months: mean {:.9} d, σ {:.6} d",
        synodic_statistics.count(),
        days(synodic_statistics.mean()),
        days(synodic_statistics.standard_deviation()),
    );
    Ok(())
}
