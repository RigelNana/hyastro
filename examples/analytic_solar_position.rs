use hyastro::{
    astro::{Astrometry, ReceptionLightTimeOptions},
    ephem::SofaAnalyticEphemeris,
    time::{DateTime, Gregorian, TimeContext, Utc},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let time = TimeContext::builtin();
    let ephemeris = SofaAnalyticEphemeris::new();
    let epoch = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2026, 8, 9, 0, 0, 0, 0,
    )?)?;
    let apparent = Astrometry::new(&time, &ephemeris)
        .solar_apparent_place(epoch, ReceptionLightTimeOptions::standard())?;
    let equatorial = apparent.true_equatorial().coordinates();

    println!("epoch                       = 2026-08-09T00:00:00 UTC");
    println!(
        "ephemeris                   = {}",
        SofaAnalyticEphemeris::MODEL
    );
    println!(
        "apparent ecliptic longitude = {:.9}°",
        apparent.longitude().as_degrees()
    );
    println!(
        "true-of-date right ascension = {:.9} h",
        equatorial.right_ascension().as_hours()
    );
    println!(
        "true-of-date declination     = {:+.9}°",
        equatorial.declination().as_degrees()
    );
    println!(
        "Sun–Earth distance           = {:.9} au",
        apparent.distance().as_astronomical_units()
    );
    println!(
        "one-way light time           = {:.6} s",
        apparent.light_time().as_seconds_f64()
    );
    println!(
        "accuracy class               = analytical; Earth heliocentric RMS {:.1} km over 1900–2100",
        SofaAnalyticEphemeris::EARTH_HELIOCENTRIC_POSITION_RMS_KILOMETRES,
    );

    Ok(())
}
