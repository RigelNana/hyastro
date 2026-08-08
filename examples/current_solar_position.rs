use std::{
    env,
    f64::consts::{PI, TAU},
    ffi::OsString,
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use hyastro::{
    astro::{Astrometry, ReceptionLightTimeOptions},
    earth::{GeodeticLatitude, GeodeticLongitude},
    ephem::{Ephemeris, KernelManifest},
    frame::Frames,
    math::{Altitude, Azimuth},
    time::{
        Duration, EarthOrientationAcceptance, EarthRotationTable, Gregorian, Hifitime,
        IersFinals2000A, Jiff, JulianDate, ModifiedJulianDate, TimeContext, Utc,
    },
};

struct Inputs {
    kernel_path: PathBuf,
    eop_path: PathBuf,
    latitude: GeodeticLatitude,
    longitude: GeodeticLongitude,
}

impl Inputs {
    const USAGE: &'static str = "usage: cargo run --features anise,jiff --example current_solar_position -- /path/to/de440s.bsp /path/to/finals.all LATITUDE_DEG LONGITUDE_DEG_EAST";

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
        if arguments.next().is_some() {
            return Err(Error::new(ErrorKind::InvalidInput, Self::USAGE));
        }
        Ok(Self {
            kernel_path,
            eop_path,
            latitude,
            longitude,
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = Inputs::from_process()?;

    // The device clock supplies the current POSIX timestamp. Jiff imports it,
    // and Hifitime maps it to hyastro's exact physical UTC instant.
    let unix_now = Jiff::new().import_timestamp(jiff::Timestamp::now());
    let now = Hifitime::new().resolve_unix(unix_now);
    let base = TimeContext::builtin();

    // Sidereal time needs measured/predicted UT1−UTC. Pass a recently downloaded
    // IERS finals.all file rather than silently using a stale bundled snapshot.
    let eop_text = fs::read_to_string(&inputs.eop_path)?;
    let eop_data = IersFinals2000A::parse(&eop_text)?;
    let current_mjd = JulianDate::<Utc>::from_instant(now, &base)?
        .to_modified()?
        .as_f64_lossy()
        .floor();
    let samples = eop_data.try_earth_rotation_samples_in(
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
    let earth_rotation = EarthRotationTable::new(&samples, "runtime IERS finals2000A", expires)?;
    let time = base.with_earth_rotation(earth_rotation);

    let ephemeris = Ephemeris::load(KernelManifest::inspect([inputs.kernel_path])?)?;
    let astrometry = Astrometry::new(&time, &ephemeris);
    let apparent =
        astrometry.solar_apparent_ecliptic(now, ReceptionLightTimeOptions::standard())?;

    // Convert the apparent direction to true equator/equinox of date. Combining
    // its right ascension with local apparent sidereal time gives the local hour
    // angle used by the standard equatorial-to-horizontal rotation.
    let frames = Frames::new(&time);
    let celestial = frames.celestial_orientation_at(now)?;
    let gcrs = celestial.gcrs_from_true_ecliptic(apparent.coordinates())?;
    let equatorial = celestial.true_equatorial(gcrs)?.coordinates();
    let sidereal = frames.sidereal_time_at(now)?;
    let local_sidereal = sidereal.local_apparent_sidereal_time(inputs.longitude.as_longitude())?;
    let hour_angle = (local_sidereal.as_radians() - equatorial.right_ascension().as_radians() + PI)
        .rem_euclid(TAU)
        - PI;

    let latitude = inputs.latitude.as_radians();
    let declination = equatorial.declination().as_radians();
    let (latitude_sine, latitude_cosine) = latitude.sin_cos();
    let (declination_sine, declination_cosine) = declination.sin_cos();
    let (hour_angle_sine, hour_angle_cosine) = hour_angle.sin_cos();
    let east = -declination_cosine * hour_angle_sine;
    let north =
        declination_sine * latitude_cosine - declination_cosine * hour_angle_cosine * latitude_sine;
    let up =
        declination_sine * latitude_sine + declination_cosine * hour_angle_cosine * latitude_cosine;
    let altitude = Altitude::try_from_radians(up.clamp(-1.0, 1.0).asin())?;
    let azimuth = Azimuth::wrap_radians(east.atan2(north))?;

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
        "site (WGS84)        = latitude {:+.7}°, longitude {:+.7}° east",
        inputs.latitude.as_degrees(),
        inputs.longitude.as_degrees(),
    );
    println!("altitude            = {:+.7}°", altitude.as_degrees());
    println!(
        "azimuth             = {:.7}° east of north",
        azimuth.as_degrees()
    );
    println!(
        "right ascension     = {:.9} h",
        equatorial.right_ascension().as_hours()
    );
    println!(
        "declination         = {:+.9}°",
        equatorial.declination().as_degrees()
    );
    println!(
        "distance            = {:.9} au",
        apparent.distance().as_astronomical_units()
    );
    println!(
        "above horizon       = {}",
        if altitude.as_radians() >= 0.0 {
            "yes"
        } else {
            "no"
        }
    );
    println!("EOP                 = {}", time.earth_rotation().version());
    println!(
        "model               = geocentric apparent solar centre projected onto the local horizon"
    );
    println!("not applied         = station parallax, polar motion, atmospheric refraction");

    Ok(())
}
