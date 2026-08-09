use hyastro::{
    frame::Frames,
    math::{HoursMinutesSeconds, Longitude},
    time::{
        Duration, EarthOrientationAcceptance, EarthRotationTable, Gregorian, Hifitime,
        IersFinals2000A, Jiff, JulianDate, ModifiedJulianDate, TimeContext, Utc,
    },
};

const FINALS_2000_A: &str = include_str!("../data/eop/finals2000a-2026-08-09.all");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = TimeContext::builtin();
    let unix_now = Jiff::new().import_timestamp(jiff::Timestamp::now());
    let now = Hifitime::new().resolve_unix(unix_now);
    let current_mjd = JulianDate::<Utc>::from_instant(now, &base)?.to_modified()?;
    let current_day = current_mjd.as_f64_lossy().floor();
    let start = ModifiedJulianDate::<Utc>::from_parts(current_day, 0.0)?;
    let end = ModifiedJulianDate::<Utc>::from_parts(current_day + 1.0, 0.0)?;

    let data = IersFinals2000A::parse(FINALS_2000_A)?;
    let samples = data.try_earth_rotation_samples_in(
        &base,
        start,
        end,
        EarthOrientationAcceptance::IncludePredicted,
    )?;
    let expires = samples
        .last()
        .expect("the current finals2000A interval contains records")
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let table = EarthRotationTable::new(&samples, "IERS finals2000A snapshot 2026-08-06", expires)?;
    let time = base.with_earth_rotation(table);
    let solution = Frames::new(&time).sidereal_time_at(now)?;

    let longitude = Longitude::try_from_degrees(121.458766667)?;
    let local_mean = solution.local_mean_sidereal_time(longitude)?;
    let local_apparent = solution.local_apparent_sidereal_time(longitude)?;
    let rotation = time.earth_rotation_at(now)?;
    let utc_label = base.represent::<Gregorian, Utc>(now)?;

    println!("UTC       = {utc_label:?}");
    println!("EOP       = {}", time.earth_rotation().version());
    println!(
        "UT1−UTC   = {:+.7} s",
        rotation.ut1_minus_utc().as_seconds()
    );
    println!(
        "ERA       = {:.9} h ({:.9}°)",
        solution.earth_rotation_angle().as_hours(),
        solution.earth_rotation_angle().as_degrees()
    );
    println!(
        "GMST      = {:.9} h",
        solution.greenwich_mean_sidereal_time().as_hours()
    );
    let hms = HoursMinutesSeconds::from_decimal_hours(local_apparent.as_hours())?;
    println!("GAST      = {} h", hms);
    println!("LMST      = {:.9} h", local_mean.as_hours());
    println!("LAST      = {:.9} h", local_apparent.as_hours());
    println!("longitude = {:+.6}° east", longitude.as_degrees());

    Ok(())
}
