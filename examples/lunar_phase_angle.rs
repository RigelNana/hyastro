use std::io::{Error, ErrorKind};

use hyastro::{
    astro::{Astrometry, MoonPhaseAngle, ReceptionLightTimeOptions},
    ephem::{Ephemeris, KernelManifest},
    event::{AngularEventSearchOptions, Events},
    time::{DateTime, FixedUtcOffset, Gregorian, TimeContext, TimeInterval, Utc},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kernel_path = std::env::args_os().nth(1).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "usage: cargo run --features anise --example lunar_phase_angle -- /path/to/de440s.bsp",
        )
    })?;

    let time = TimeContext::builtin();
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel_path])?)?;
    let astrometry = Astrometry::new(&time, &ephemeris);
    let interval = TimeInterval::new(
        time.resolve(DateTime::<Gregorian, Utc>::from_components(
            2024, 3, 10, 0, 0, 0, 0,
        )?)?,
        time.resolve(DateTime::<Gregorian, Utc>::from_components(
            2024, 4, 10, 0, 0, 0, 0,
        )?)?,
    )?;
    let target = MoonPhaseAngle::try_from_degrees(45.0)?;
    let events = Events::new(astrometry).moon_phase_angle_in(
        interval,
        target,
        AngularEventSearchOptions::standard(),
    )?;

    println!(
        "{} crossing(s) of directed Moon phase angle {:.1} deg",
        events.len(),
        target.as_degrees()
    );
    for event in events {
        let label = time.represent_fixed::<Gregorian, _>(event.instant(), FixedUtcOffset::UTC)?;
        let date = label.date();
        let clock = label.time();
        let illumination = astrometry
            .lunar_illumination_at(event.instant(), ReceptionLightTimeOptions::standard())?;
        println!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09} UTC  actual={:.12} deg  branch={:?}  illuminated={:.6}%  residual={:+.3e} rad  +/-{:.9} s",
            date.year(),
            date.month(),
            date.day(),
            clock.hour(),
            clock.minute(),
            clock.second(),
            clock.nanosecond(),
            event.longitude_difference().as_degrees(),
            illumination.branch(),
            illumination.illuminated_fraction().as_percent(),
            event.evidence().residual().as_radians(),
            event.evidence().time_uncertainty().as_seconds_f64(),
        );
    }

    Ok(())
}
