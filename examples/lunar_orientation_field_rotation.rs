use std::error::Error;

use hyastro::{
    astro::{Astrometry, FieldRotationOptions, LunarRotationModel, ReceptionLightTimeOptions},
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{CelestialBody, SofaAnalyticEphemeris},
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, IersC04,
        ModifiedJulianDate, TimeContext, Utc,
    },
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");

fn main() -> Result<(), Box<dyn Error>> {
    let base = TimeContext::builtin();
    let data = IersC04::parse(C04)?;
    let samples = data.try_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(60_393.0, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(60_396.0, 0.0)?,
        EarthOrientationAcceptance::FinalOnly,
    )?;
    let expires = samples
        .last()
        .ok_or("C04 lunar-orientation window is empty")?
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let eop = EarthOrientationTable::new(&samples, "IERS C04 2024 lunar window", expires)?;
    let time = base.with_earth_orientation(eop);
    let epoch = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2024, 3, 25, 7, 0, 0, 0,
    )?)?;
    let site = Earth::wgs84().fixed_site(
        "Beijing",
        GeodeticPosition::new(
            GeodeticLongitude::try_from_degrees(116.391)?,
            GeodeticLatitude::try_from_degrees(39.9075)?,
            EllipsoidalHeight::from_metres(43.5)?,
        ),
    )?;
    let ephemeris = SofaAnalyticEphemeris::new();
    let astrometry = Astrometry::new(&time, &ephemeris);
    let light_time = ReceptionLightTimeOptions::standard();

    let lunar = astrometry.lunar_disk_orientation_at(
        epoch,
        light_time,
        LunarRotationModel::Iau2009Wgccre,
    )?;
    let optical = lunar.optical_libration();
    let physical = lunar.physical_libration();
    let total = lunar.total_libration();
    println!("epoch = {:?}", time.represent::<Gregorian, Utc>(epoch)?);
    println!("ephemeris = {}", SofaAnalyticEphemeris::MODEL);
    println!("lunar rotation = {}", lunar.rotation().model().identifier());
    println!(
        "optical libration = lon {:+.6} deg, lat {:+.6} deg",
        optical.longitude().as_degrees(),
        optical.latitude().as_degrees(),
    );
    println!(
        "physical correction = lon {:+.6} deg, lat {:+.6} deg",
        physical.longitude().as_degrees(),
        physical.latitude().as_degrees(),
    );
    println!(
        "total libration = lon {:+.6} deg, lat {:+.6} deg",
        total.longitude().as_degrees(),
        total.latitude().as_degrees(),
    );
    println!(
        "axis PA = {:.6} deg; bright-limb PA = {:.6} deg",
        lunar.axis_position_angle().as_degrees(),
        lunar.bright_limb_position_angle().as_degrees(),
    );

    let field = astrometry.field_rotation_at(
        &site,
        CelestialBody::Moon,
        epoch,
        light_time,
        FieldRotationOptions::standard(),
    )?;
    println!("site = {}", site.identifier());
    println!(
        "parallactic angle = {:+.6} deg",
        field.parallactic_angle().as_degrees(),
    );
    println!(
        "field rotation = {:+.6} arcsec/s ({:?})",
        field.rate().as_arcseconds_per_second(),
        field.direction(),
    );
    println!(
        "symmetric sample offset = {:.3} s",
        field.sample_offset()?.as_seconds_f64(),
    );
    Ok(())
}
