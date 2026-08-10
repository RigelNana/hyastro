use std::error::Error;

use hyastro::{
    astro::Astrometry,
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::SofaAnalyticEphemeris,
    event::{Events, SolarEclipseContact, SolarEclipseSearchOptions},
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, IersC04,
        ModifiedJulianDate, TimeContext, TimeInterval, Utc,
    },
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");

fn main() -> Result<(), Box<dyn Error>> {
    let base = TimeContext::builtin();
    let data = IersC04::parse(C04)?;
    let samples = data.try_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(60_406.0, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(60_410.0, 0.0)?,
        EarthOrientationAcceptance::FinalOnly,
    )?;
    let expires = samples
        .last()
        .ok_or("C04 eclipse window is empty")?
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let eop = EarthOrientationTable::new(&samples, "IERS C04 2024 eclipse window", expires)?;
    let time = base.with_earth_orientation(eop);
    let site = Earth::wgs84().fixed_site(
        "Dallas, Texas",
        GeodeticPosition::new(
            GeodeticLongitude::try_from_degrees(-96.7970)?,
            GeodeticLatitude::try_from_degrees(32.7767)?,
            EllipsoidalHeight::from_metres(131.0)?,
        ),
    )?;
    let start = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2024, 4, 8, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2024, 4, 9, 0, 0, 0, 0,
    )?)?;
    let ephemeris = SofaAnalyticEphemeris::new();
    let eclipses = Events::new(Astrometry::new(&time, &ephemeris)).local_solar_eclipses_in(
        &site,
        TimeInterval::new(start, end)?,
        SolarEclipseSearchOptions::standard(),
    )?;
    let eclipse = eclipses
        .first()
        .ok_or("the analytic ephemeris did not find the 2024-04-08 Dallas eclipse")?;

    println!("site = {}", eclipse.site().identifier());
    println!("kind = {:?}", eclipse.kind());
    print_contact(&time, eclipse.first_contact())?;
    if let Some(contact) = eclipse.second_contact() {
        print_contact(&time, contact)?;
    }
    let maximum = eclipse.maximum().observation();
    println!(
        "MAX {:?} magnitude={:.6} obscuration={:.4}% Sun altitude={:.3} deg",
        time.represent::<Gregorian, Utc>(maximum.instant())?,
        maximum.magnitude().as_ratio(),
        maximum.obscuration().as_percent(),
        maximum.solar_horizontal().altitude().as_degrees(),
    );
    if let Some(contact) = eclipse.third_contact() {
        print_contact(&time, contact)?;
    }
    print_contact(&time, eclipse.fourth_contact())?;
    println!(
        "partial duration = {:.3} min",
        eclipse.partial_phase_duration().as_seconds_f64() / 60.0
    );
    if let Some(duration) = eclipse.central_phase_duration() {
        println!(
            "central duration = {:.3} min",
            duration.as_seconds_f64() / 60.0
        );
    }
    println!("ephemeris = {}", eclipse.ephemeris_provenance().model());
    println!("Earth attitude = {:?}", eclipse.earth_attitude_provenance());
    Ok(())
}

fn print_contact(
    time: &TimeContext<'_, EarthOrientationTable<'_>>,
    contact: SolarEclipseContact<Utc>,
) -> Result<(), Box<dyn Error>> {
    let observation = contact.observation();
    println!(
        "{:?} {:?} P={:.3} deg Sun altitude={:.3} deg visible={}",
        contact.kind(),
        time.represent::<Gregorian, Utc>(contact.instant())?,
        contact.limb_position_angle().as_degrees(),
        observation.solar_horizontal().altitude().as_degrees(),
        observation.solar_disk_is_above_horizon(),
    );
    Ok(())
}
