use std::io::{Error, ErrorKind};

use hyastro::{
    astro::{Astrometry, HorizonsCompatibleLunarV, ReceptionLightTimeOptions},
    ephem::{Ephemeris, KernelManifest},
    time::{DateTime, Gregorian, TimeContext, Utc},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kernel_path = std::env::args_os().nth(1).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "usage: cargo run --features anise --example lunar_illumination -- /path/to/de440s.bsp",
        )
    })?;

    let time = TimeContext::builtin();
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel_path])?)?;
    let epoch = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2026, 8, 9, 0, 0, 0, 0,
    )?)?;

    let astrometry = Astrometry::new(&time, &ephemeris);
    let illumination =
        astrometry.lunar_illumination_at(epoch, ReceptionLightTimeOptions::standard())?;
    let visual_magnitude = HorizonsCompatibleLunarV::evaluate(illumination)?;
    let moon = illumination.apparent_moon();
    let sun = illumination.apparent_sun();
    let sunlight = illumination.sunlight_at_moon();

    println!("epoch                         = 2026-08-09T00:00:00 UTC");
    println!(
        "phase branch                  = {:?}",
        illumination.branch()
    );
    println!(
        "directed Moon-Sun longitude  = {:.12} deg",
        illumination.directed_elongation().as_degrees()
    );
    println!(
        "apparent Moon-Sun separation = {:.12} deg",
        illumination.apparent_separation().as_degrees()
    );
    println!(
        "physical phase angle         = {:.12} deg",
        illumination.phase_angle().as_degrees()
    );
    println!(
        "illuminated fraction          = {:.9}%",
        illumination.illuminated_fraction().as_percent()
    );
    println!(
        "uneclipsed airless V magnitude = {:.9} mag ({:?})",
        visual_magnitude.magnitude().as_magnitudes(),
        visual_magnitude.applicability(),
    );
    println!(
        "Moon-to-Earth light time      = {:.9} s ({} iterations, {} ns residual)",
        moon.light_time().as_seconds_f64(),
        moon.iterations(),
        moon.light_time_residual().as_nanoseconds(),
    );
    println!(
        "Sun-to-Earth light time       = {:.9} s ({} iterations, {} ns residual)",
        sun.light_time().as_seconds_f64(),
        sun.iterations(),
        sun.light_time_residual().as_nanoseconds(),
    );
    println!(
        "Sun-to-Moon light time        = {:.9} s ({} iterations, {} ns residual)",
        sunlight.light_time().as_seconds_f64(),
        sunlight.iterations(),
        sunlight.residual().as_nanoseconds(),
    );
    println!(
        "sunlight reception is lunar emission epoch = {}",
        sunlight.reception_epoch() == moon.emission_epoch()
    );
    println!(
        "model excludes                 = station parallax, lunar limb topography, libration, opposition surge"
    );

    Ok(())
}
