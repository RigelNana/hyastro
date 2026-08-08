use std::io::{Error, ErrorKind};

use hyastro::{
    astro::{Astrometry, ReceptionLightTimeOptions},
    ephem::{Ephemeris, KernelManifest},
    frame::Frames,
    time::{DateTime, Gregorian, TimeContext, Utc},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kernel_path = std::env::args_os().nth(1).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "usage: cargo run --features anise --example solar_apparent_position -- /path/to/de440s.bsp",
        )
    })?;

    let time = TimeContext::builtin();
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel_path])?)?;
    let epoch = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2024, 3, 20, 3, 6, 0, 0,
    )?)?;
    let astrometry = Astrometry::new(&time, &ephemeris);
    let apparent =
        astrometry.solar_apparent_ecliptic(epoch, ReceptionLightTimeOptions::standard())?;

    let celestial = Frames::new(&time).celestial_orientation_at(epoch)?;
    let gcrs = celestial.gcrs_from_true_ecliptic(apparent.coordinates())?;
    let equatorial = celestial.true_equatorial(gcrs)?.coordinates();

    println!("epoch                      = 2024-03-20T03:06:00 UTC");
    println!(
        "apparent ecliptic longitude = {:.12}°",
        apparent.longitude().as_degrees()
    );
    println!(
        "apparent ecliptic latitude  = {:+.12}°",
        apparent.latitude().as_degrees()
    );
    println!(
        "true-of-date right ascension = {:.12} h",
        equatorial.right_ascension().as_hours()
    );
    println!(
        "true-of-date declination     = {:+.12}°",
        equatorial.declination().as_degrees()
    );
    println!(
        "Sun–Earth distance           = {:.12} au",
        apparent.distance().as_astronomical_units()
    );
    println!(
        "one-way light time           = {:.9} s ({} iterations, {} ns residual)",
        apparent.light_time().as_seconds_f64(),
        apparent.iterations(),
        apparent.light_time_residual().as_nanoseconds(),
    );
    println!("model excludes              = station parallax and atmospheric refraction");

    Ok(())
}
