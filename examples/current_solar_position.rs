use std::{
    env,
    ffi::OsString,
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use hyastro::{
    astro::{Astrometry, ReceptionLightTimeOptions},
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{CelestialBody, Ephemeris, KernelManifest},
    time::{
        Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, Hifitime,
        IersFinals2000A, Jiff, JulianDate, ModifiedJulianDate, TimeContext, Utc,
    },
};

struct Inputs {
    kernel_path: PathBuf,
    eop_path: PathBuf,
    latitude: GeodeticLatitude,
    longitude: GeodeticLongitude,
    height: EllipsoidalHeight,
    epoch: Option<jiff::Timestamp>,
}

impl Inputs {
    const USAGE: &'static str = "usage: cargo run --features anise,jiff --example current_solar_position -- /path/to/de440s.bsp /path/to/finals.all LATITUDE_DEG LONGITUDE_DEG_EAST ELLIPSOIDAL_HEIGHT_METRES [UTC_TIMESTAMP]";

    fn from_process() -> Result<Self, Error> {
        let mut arguments = env::args_os().skip(1);
        let kernel_path = PathBuf::from(Self::required(&mut arguments)?);
        let eop_path = PathBuf::from(Self::required(&mut arguments)?);
        let latitude = GeodeticLatitude::try_from_degrees(Self::decimal(
            Self::required(&mut arguments)?,
            "latitude",
        )?)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
        let longitude = GeodeticLongitude::wrap_degrees(Self::decimal(
            Self::required(&mut arguments)?,
            "east-positive longitude",
        )?)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
        let height = EllipsoidalHeight::from_metres(Self::decimal(
            Self::required(&mut arguments)?,
            "ellipsoidal height",
        )?)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
        let epoch = arguments.next().map(Self::timestamp).transpose()?;
        if arguments.next().is_some() {
            return Err(Error::new(ErrorKind::InvalidInput, Self::USAGE));
        }
        Ok(Self {
            kernel_path,
            eop_path,
            latitude,
            longitude,
            height,
            epoch,
        })
    }

    fn required(arguments: &mut impl Iterator<Item = OsString>) -> Result<OsString, Error> {
        arguments
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, Self::USAGE))
    }

    fn decimal(value: OsString, field: &str) -> Result<f64, Error> {
        let value = value
            .into_string()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, format!("{field} must be UTF-8")))?;
        value.parse::<f64>().map_err(|source| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("invalid {field} {value:?}: {source}"),
            )
        })
    }

    fn timestamp(value: OsString) -> Result<jiff::Timestamp, Error> {
        let value = value
            .into_string()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "UTC timestamp must be UTF-8"))?;
        value.parse::<jiff::Timestamp>().map_err(|source| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("invalid UTC timestamp {value:?}: {source}"),
            )
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = Inputs::from_process()?;

    // With no timestamp argument, the device clock supplies the current POSIX
    // timestamp. An explicit UTC timestamp makes the complete pipeline
    // reproducible and allows use with historical EOP products.
    let timestamp = inputs.epoch.unwrap_or_else(jiff::Timestamp::now);
    let unix_now = Jiff::new().import_timestamp(timestamp);
    let now = Hifitime::new().resolve_unix(unix_now);
    let base = TimeContext::builtin();

    // Fixed-site astrometry requires the complete observed Earth attitude:
    // UT1−UTC and LOD, polar motion, and celestial-pole offsets. Pass a
    // recently downloaded IERS finals.all instead of a bundled snapshot.
    let eop_text = fs::read_to_string(&inputs.eop_path)?;
    let eop_data = IersFinals2000A::parse(&eop_text)?;
    let current_mjd = JulianDate::<Utc>::from_instant(now, &base)?
        .to_modified()?
        .as_f64_lossy()
        .floor();
    let samples = eop_data.try_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(current_mjd, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(current_mjd + 1.0, 0.0)?,
        EarthOrientationAcceptance::IncludePredicted,
    )?;
    let expires = samples
        .last()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "finals.all has no current EOP rows"))?
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let eop = EarthOrientationTable::new(&samples, "runtime IERS finals2000A", expires)?;
    let time = base.with_earth_orientation(eop);

    let earth = Earth::wgs84();
    let site = earth.fixed_site(
        "command-line site",
        GeodeticPosition::new(inputs.longitude, inputs.latitude, inputs.height),
    )?;
    let ephemeris = Ephemeris::load(KernelManifest::inspect([inputs.kernel_path])?)?;
    let astrometry = Astrometry::new(&time, &ephemeris);
    let observer = astrometry.fixed_observer_at(&site, now)?;
    let observed = observer
        .vacuum_observed_place(CelestialBody::Sun, ReceptionLightTimeOptions::standard())?;
    let horizontal = observed.horizontal();
    let intermediate = observed.intermediate_equatorial().coordinates();

    let utc = base.represent::<Gregorian, Utc>(now)?;
    let date = utc.date();
    let clock = utc.time();
    println!(
        "UTC                 = {:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z",
        date.year(),
        date.month(),
        date.day(),
        clock.hour(),
        clock.minute(),
        clock.second(),
        clock.nanosecond(),
    );
    println!(
        "site (WGS84)        = latitude {:+.7}°, longitude {:+.7}° east, height {:+.3} m",
        inputs.latitude.as_degrees(),
        inputs.longitude.as_degrees(),
        inputs.height.as_metres(),
    );
    println!(
        "altitude            = {:+.7}°",
        horizontal.altitude().as_degrees()
    );
    if let Some(azimuth) = horizontal.azimuth() {
        println!(
            "azimuth             = {:.7}° east of north",
            azimuth.as_degrees()
        );
    } else {
        println!("azimuth             = undefined at zenith or nadir");
    }
    println!(
        "CIRS right ascension = {:.9} h",
        intermediate.right_ascension().as_hours()
    );
    println!(
        "CIRS declination     = {:+.9}°",
        intermediate.declination().as_degrees()
    );
    println!(
        "distance             = {:.9} au",
        observed.distance().as_astronomical_units()
    );
    println!(
        "one-way light time   = {:.9} s ({} iterations, {} ns residual)",
        observed.light_time().as_seconds_f64(),
        observed.iterations(),
        observed.light_time_residual().as_nanoseconds(),
    );
    println!(
        "above horizon        = {}",
        if horizontal.altitude().as_radians() >= 0.0 {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "EOP                  = {}",
        time.earth_orientation().version()
    );
    println!("model                = topocentric vacuum observed solar centre");
    println!(
        "applied              = station parallax, combined observer aberration, IAU 2006/2000A Earth attitude, polar motion"
    );
    println!(
        "not applied          = atmospheric refraction, Shapiro delay, point-mass light deflection"
    );

    Ok(())
}
