use std::io::{Error, ErrorKind};

use hyastro::{
    ephem::{CelestialBody, Ephemeris, EphemerisQuery, KernelManifest},
    frame::Bcrs,
    time::{DateTime, Gregorian, Hifitime, Tdb},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kernel_path = std::env::args_os().nth(1).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "usage: cargo run --features anise --example ephemeris_de440s -- /path/to/de440s.bsp",
        )
    })?;
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel_path])?)?;
    let epoch = Hifitime::new().resolve(DateTime::<Gregorian, Tdb>::from_components(
        2000, 1, 1, 12, 0, 0, 0,
    )?)?;

    let sun_from_ssb = ephemeris.state(EphemerisQuery::<Bcrs, _>::new(
        CelestialBody::Sun,
        CelestialBody::SolarSystemBarycenter,
        epoch,
    ))?;
    let earth_from_ssb = ephemeris.state(EphemerisQuery::<Bcrs, _>::new(
        CelestialBody::Earth,
        CelestialBody::SolarSystemBarycenter,
        epoch,
    ))?;
    let sun_from_earth = ephemeris.state(EphemerisQuery::<Bcrs, _>::new(
        CelestialBody::Sun,
        CelestialBody::Earth,
        epoch,
    ))?;
    let chained = sun_from_ssb.checked_chain(earth_from_ssb.checked_reversed()?)?;
    assert!(
        sun_from_earth
            .position()
            .checked_sub(chained.position())?
            .magnitude()?
            .as_metres()
            < 1.0e-3
    );

    let coverage = ephemeris.coverage(EphemerisQuery::<Bcrs, _>::new(
        CelestialBody::Sun,
        CelestialBody::Earth,
        epoch,
    ))?;
    let [x, y, z] = sun_from_earth.position().components();
    let [vx, vy, vz] = sun_from_earth.velocity().components();
    println!(
        "Sun relative to Earth at 2000-01-01T12:00:00 TDB\nposition = [{:.9}, {:.9}, {:.9}] km\nvelocity = [{:.12}, {:.12}, {:.12}] km/s",
        x.as_kilometres(),
        y.as_kilometres(),
        z.as_kilometres(),
        vx.as_kilometres_per_second(),
        vy.as_kilometres_per_second(),
        vz.as_kilometres_per_second(),
    );
    println!(
        "coverage = {}..={} TAI ns; kernels = {}",
        coverage.start().tai_nanoseconds_since_1900(),
        coverage.end().tai_nanoseconds_since_1900(),
        ephemeris.manifest().kernel_count(),
    );
    Ok(())
}
