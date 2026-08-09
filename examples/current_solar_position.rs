use std::{
    env,
    ffi::OsString,
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use hyastro::{
    astro::{
        AirTemperature, Astrometry, AtmosphericConditions, AtmosphericPressure,
        ObservingWavelength, ReceptionLightTimeOptions, RelativeHumidity,
    },
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{CelestialBody, Ephemeris, KernelManifest, SphericalBodyFigure},
    math::Longitude,
    time::{
        Duration, EarthAttitudeTable, EarthOrientationAcceptance, Gregorian, Hifitime,
        IersFinals2000A, Jiff, JulianDate, ModifiedJulianDate, TimeContext, Utc,
    },
};

struct Inputs {
    kernel_path: PathBuf,
    eop_path: PathBuf,
    latitude: GeodeticLatitude,
    longitude: GeodeticLongitude,
    height: EllipsoidalHeight,
    atmosphere: AtmosphericConditions,
    epoch: Option<jiff::Timestamp>,
}

impl Inputs {
    const USAGE: &'static str = "usage: cargo run --features anise,jiff --example current_solar_position -- /path/to/de440s.bsp /path/to/finals.all LATITUDE_DEG LONGITUDE_DEG_EAST ELLIPSOIDAL_HEIGHT_METRES PRESSURE_HPA TEMPERATURE_C RELATIVE_HUMIDITY WAVELENGTH_MICROMETRES [UTC_TIMESTAMP]";

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
        let pressure = AtmosphericPressure::from_hectopascals(Self::decimal(
            Self::required(&mut arguments)?,
            "atmospheric pressure",
        )?)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
        let temperature = AirTemperature::from_degrees_celsius(Self::decimal(
            Self::required(&mut arguments)?,
            "air temperature",
        )?)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
        let relative_humidity = RelativeHumidity::from_fraction(Self::decimal(
            Self::required(&mut arguments)?,
            "relative humidity",
        )?)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
        let wavelength = ObservingWavelength::from_micrometres(Self::decimal(
            Self::required(&mut arguments)?,
            "observing wavelength",
        )?)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
        let atmosphere =
            AtmosphericConditions::new(pressure, temperature, relative_humidity, wavelength);
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
            atmosphere,
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

    // Fixed-site direction astrometry requires observed Earth attitude:
    // UT1−UTC, polar motion, and celestial-pole offsets. A current finals.all
    // may omit predicted LOD, so site velocity explicitly uses nominal rotation.
    let eop_text = fs::read_to_string(&inputs.eop_path)?;
    let eop_data = IersFinals2000A::parse(&eop_text)?;
    let current_mjd = JulianDate::<Utc>::from_instant(now, &base)?
        .to_modified()?
        .as_f64_lossy()
        .floor();
    let samples = eop_data.try_earth_attitude_samples_in(
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
    let eop = EarthAttitudeTable::new(&samples, "runtime IERS finals2000A", expires)?;
    let time = base.with_earth_attitude(eop);

    let earth = Earth::wgs84();
    let site = earth.fixed_site(
        "command-line site",
        GeodeticPosition::new(inputs.longitude, inputs.latitude, inputs.height),
    )?;
    let ephemeris = Ephemeris::load(KernelManifest::inspect([inputs.kernel_path])?)?;
    let astrometry = Astrometry::new(&time, &ephemeris);
    let solar_time = astrometry.solar_time(now, ReceptionLightTimeOptions::standard())?;
    let local_solar_time =
        solar_time.at_longitude(Longitude::try_from_radians(inputs.longitude.as_radians())?)?;
    let observer = astrometry.fixed_observer_with_nominal_rotation_at(&site, now)?;
    let vacuum = observer
        .vacuum_observed_place(CelestialBody::Sun, ReceptionLightTimeOptions::standard())?;
    let solar_disk = vacuum.apparent_disk(SphericalBodyFigure::IAU_2015_NOMINAL_SUN)?;
    let solar_deflection = vacuum.solar_light_deflection();
    let observed = vacuum.apply_refraction(inputs.atmosphere)?;
    let vacuum_horizontal = vacuum.horizontal();
    let horizontal = observed.horizontal();
    let intermediate = vacuum.intermediate_equatorial().coordinates();

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
        "vacuum altitude      = {:+.7}°",
        vacuum_horizontal.altitude().as_degrees()
    );
    println!(
        "observed altitude    = {:+.7}°",
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
    let mean_solar_clock = local_solar_time.mean_solar_time().as_time_of_day();
    let apparent_solar_clock = local_solar_time.apparent_solar_time().as_time_of_day();
    println!(
        "mean solar time      = {:02}:{:02}:{:02}.{:09}",
        mean_solar_clock.hour(),
        mean_solar_clock.minute(),
        mean_solar_clock.second(),
        mean_solar_clock.nanosecond(),
    );
    println!(
        "apparent solar time  = {:02}:{:02}:{:02}.{:09}",
        apparent_solar_clock.hour(),
        apparent_solar_clock.minute(),
        apparent_solar_clock.second(),
        apparent_solar_clock.nanosecond(),
    );
    println!(
        "equation of time     = {:+.6} min (apparent minus mean)",
        local_solar_time.equation_of_time().as_minutes(),
    );
    println!(
        "refraction           = {:+.6} arcsec ({:?})",
        observed.refraction().amount().as_degrees() * 3_600.0,
        observed.refraction().accuracy(),
    );
    println!(
        "distance             = {:.9} au",
        vacuum.distance().as_astronomical_units()
    );
    println!(
        "angular diameter     = {:.6} arcmin ({})",
        solar_disk.diameter().as_degrees() * 60.0,
        solar_disk.figure().identifier(),
    );
    println!(
        "one-way light time   = {:.9} s ({} iterations, {} ns residual)",
        vacuum.light_time().as_seconds_f64(),
        vacuum.iterations(),
        vacuum.light_time_residual().as_nanoseconds(),
    );
    println!(
        "solar deflection     = {:+.6} arcsec ({:?})",
        solar_deflection.correction().as_degrees() * 3_600.0,
        solar_deflection.disposition(),
    );
    println!(
        "above horizon        = {}",
        if horizontal.altitude().as_radians() >= 0.0 {
            "yes"
        } else {
            "no"
        }
    );
    println!("EOP                  = {}", time.earth_attitude().version());
    println!("site velocity model  = {:?}", observer.velocity_model());
    println!("model                = topocentric atmospheric observed solar centre");
    println!(
        "refraction model     = {}",
        hyastro::astro::RefractionCorrection::MODEL
    );
    println!(
        "deflection model     = {}",
        hyastro::astro::SolarLightDeflection::<Utc>::MODEL
    );
    println!(
        "applied              = station parallax, finite-distance solar-deflection evaluation, combined observer aberration with IERS nominal Earth rotation, IAU 2006/2000A Earth attitude, polar motion, atmospheric refraction"
    );
    println!("not applied          = Shapiro delay");

    Ok(())
}
