use std::{
    env,
    ffi::OsString,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use hyastro::{
    astro::Astrometry,
    ephem::{Ephemeris, KernelManifest},
    event::{AngularEventSearchOptions, Events},
    time::{FixedUtcOffset, TimeContext},
};

struct Inputs {
    kernel_path: PathBuf,
    year: i32,
    utc_offset_hours: i32,
}

impl Inputs {
    const USAGE: &'static str = "usage: cargo run --features anise --example solar_terms_year -- /path/to/de440s.bsp YEAR UTC_OFFSET_HOURS";

    fn from_process() -> Result<Self, Error> {
        let mut arguments = env::args_os().skip(1);
        let kernel_path = PathBuf::from(Self::required(&mut arguments)?);
        let year = Self::integer(Self::required(&mut arguments)?, "year")?;
        let utc_offset_hours = Self::integer(Self::required(&mut arguments)?, "UTC offset hours")?;
        if !(-23..=23).contains(&utc_offset_hours) || arguments.next().is_some() {
            return Err(Error::new(ErrorKind::InvalidInput, Self::USAGE));
        }
        Ok(Self {
            kernel_path,
            year,
            utc_offset_hours,
        })
    }

    fn required(arguments: &mut impl Iterator<Item = OsString>) -> Result<OsString, Error> {
        arguments
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, Self::USAGE))
    }

    fn integer(value: OsString, field: &str) -> Result<i32, Error> {
        let value = value
            .into_string()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, format!("{field} must be UTF-8")))?;
        value.parse::<i32>().map_err(|source| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("invalid {field} {value:?}: {source}"),
            )
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = Inputs::from_process()?;
    let offset_seconds = inputs
        .utc_offset_hours
        .checked_mul(3_600)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, Inputs::USAGE))?;
    let offset = FixedUtcOffset::from_seconds(offset_seconds)?;

    let time = TimeContext::builtin();
    let ephemeris = Ephemeris::load(KernelManifest::inspect([inputs.kernel_path])?)?;
    let astrometry = Astrometry::new(&time, &ephemeris);
    let year = Events::new(astrometry).solar_term_year(
        inputs.year,
        offset,
        AngularEventSearchOptions::standard(),
    )?;

    println!(
        "{} solar terms at UTC{:+03}:00 (geocentric apparent solar longitude)",
        year.year(),
        inputs.utc_offset_hours,
    );
    for entry in year.entries() {
        let event = entry.event();
        let term = event.term();
        let local = entry.local_time();
        let date = local.date();
        let clock = local.time();
        let evidence = event.evidence();
        println!(
            "{:02}-{:02} {:02}:{:02}:{:02}.{:09}  {:<2}  {:<20} λ={:>6.1}°  ±{:.6} s",
            date.month(),
            date.day(),
            clock.hour(),
            clock.minute(),
            clock.second(),
            clock.nanosecond(),
            term.chinese_name(),
            term.english_name(),
            term.target_longitude().as_degrees(),
            evidence.time_uncertainty().as_seconds_f64(),
        );
    }

    Ok(())
}
