use hyastro::{
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    frame::Frames,
    math::Length,
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, IersC04,
        ModifiedJulianDate, TimeContext, Utc,
    },
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = TimeContext::builtin();
    let data = IersC04::parse(C04)?;
    let samples = data.try_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(54_387.0, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(54_390.0, 0.0)?,
        EarthOrientationAcceptance::FinalOnly,
    )?;
    let expires = samples
        .last()
        .expect("the requested C04 interval contains records")
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let eop = EarthOrientationTable::new(&samples, "IERS C04 Earth-site example", expires)?;
    let time = base.with_earth_orientation(eop);
    let epoch = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2007, 10, 15, 12, 0, 0, 0,
    )?)?;
    let frames = Frames::new(&time);

    let earth = Earth::wgs84();
    let site = earth.fixed_site(
        "Beijing reference site",
        GeodeticPosition::new(
            GeodeticLongitude::try_from_degrees(116.391)?,
            GeodeticLatitude::try_from_degrees(39.9075)?,
            EllipsoidalHeight::from_metres(43.5)?,
        ),
    )?;
    let recovered = earth.geodetic_position(site.itrs_position())?;
    let geocentric_latitude = earth.geocentric_latitude(site.itrs_position())?;
    let gcrs_state = site.gcrs_state(epoch, &frames)?;
    let gcrs_enu = site.gcrs_east_north_up(epoch, &frames)?;

    let [x, y, z] = site
        .itrs_position()
        .position()
        .components()
        .map(Length::as_metres);
    let [vx, vy, vz] = gcrs_state
        .velocity()
        .components()
        .map(|value| value.as_metres_per_second());
    let inertial_speed = gcrs_state.velocity().magnitude()?.as_metres_per_second();

    assert!((recovered.longitude().as_degrees() - 116.391).abs() < 1.0e-12);
    assert!((recovered.latitude().as_degrees() - 39.9075).abs() < 1.0e-12);
    assert!((recovered.height().as_metres() - 43.5).abs() < 1.0e-8);
    assert!(gcrs_enu.east().dot(gcrs_enu.north()).abs() < 1.0e-15);
    assert!(gcrs_enu.north().dot(gcrs_enu.up()).abs() < 1.0e-15);

    println!("site                  = {}", site.identifier());
    println!(
        "ellipsoid             = {} (a={:.3} m, 1/f={:.9})",
        site.reference_ellipsoid().identifier(),
        site.reference_ellipsoid().semi_major_axis().as_metres(),
        site.reference_ellipsoid().inverse_flattening()
    );
    println!(
        "geodetic lon/lat/h    = {:.9}° / {:.9}° / {:.3} m",
        recovered.longitude().as_degrees(),
        recovered.latitude().as_degrees(),
        recovered.height().as_metres()
    );
    println!(
        "geocentric latitude   = {:.9}°",
        geocentric_latitude.as_degrees()
    );
    println!("ITRS position         = [{x:.3}, {y:.3}, {z:.3}] m");
    println!("GCRS velocity         = [{vx:.6}, {vy:.6}, {vz:.6}] m/s");
    println!("GCRS inertial speed   = {inertial_speed:.6} m/s");
    println!(
        "GCRS local up         = [{:.12}, {:.12}, {:.12}]",
        gcrs_enu.up().components()[0],
        gcrs_enu.up().components()[1],
        gcrs_enu.up().components()[2]
    );

    Ok(())
}
