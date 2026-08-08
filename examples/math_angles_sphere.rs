use hyastro::{
    frame::{
        EclipticDirection, EquatorialDirection, GalacticDirection, Gcrs, Icrs,
        MeanEclipticEquinoxJ2000, MeanEquatorEquinoxJ2000,
    },
    math::{
        Altitude, Angle, Azimuth, Declination, DegreesMinutesSeconds, HourAngle,
        HoursMinutesSeconds, PhaseAngle, RightAscension, ZenithDistance,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vega_ra: HoursMinutesSeconds = "18h36m56.33635s".parse()?;
    let vega_dec: DegreesMinutesSeconds = "+38°47′01.2802″".parse()?;
    let polaris_ra: HoursMinutesSeconds = "02:31:49.09".parse()?;
    let polaris_dec: DegreesMinutesSeconds = "+89:15:50.8".parse()?;

    let vega = EquatorialDirection::<Icrs>::new(
        RightAscension::try_from_hms(vega_ra)?,
        Declination::try_from_dms(vega_dec)?,
    );
    let polaris = EquatorialDirection::<Icrs>::new(
        RightAscension::try_from_hms(polaris_ra)?,
        Declination::try_from_dms(polaris_dec)?,
    );
    let vega_galactic = GalacticDirection::from_icrs(vega)?;
    let vega_ecliptic = EclipticDirection::<MeanEclipticEquinoxJ2000>::from_icrs(vega)?;
    assert!(vega_galactic.to_icrs()?.separation_to(vega)?.as_radians() < 1.0e-14);
    assert!(vega_ecliptic.to_icrs()?.separation_to(vega)?.as_radians() < 1.0e-14);
    let vega_gcrs = EquatorialDirection::<Gcrs>::new(vega.right_ascension(), vega.declination());
    let vega_mean_j2000 = EquatorialDirection::<MeanEquatorEquinoxJ2000>::from_gcrs(vega_gcrs)?;
    assert!(
        vega_mean_j2000
            .to_gcrs()?
            .separation_to(vega_gcrs)?
            .as_radians()
            < 1.0e-14
    );

    let separation = vega.separation_to(polaris)?;
    let vega_spherical = vega.to_spherical()?;
    let polaris_spherical = polaris.to_spherical()?;
    let position_angle = vega_spherical.position_angle_to(polaris_spherical)?;
    let midpoint = vega_spherical.slerp(polaris_spherical, 0.5)?;
    let midpoint_equatorial =
        EquatorialDirection::<Icrs>::from_direction(midpoint.to_direction()?)?;
    let basis = vega_spherical.tangent_basis()?;

    assert!((basis.east().dot(basis.north())).abs() < 1.0e-15);
    assert!(
        (vega_spherical.separation_to(midpoint)?.as_radians()
            - midpoint.separation_to(polaris_spherical)?.as_radians())
        .abs()
            < 1.0e-14
    );

    let angle = Angle::from_degrees(30.0)?;
    let hour_angle = HourAngle::wrap_hours(-1.5)?;
    let azimuth = Azimuth::wrap_degrees(725.0)?;
    let altitude = Altitude::try_from_degrees(35.0)?;
    let zenith_distance = ZenithDistance::try_from_degrees(55.0)?;
    let phase_angle = PhaseAngle::try_from_degrees(42.0)?;

    assert!((angle.sin().value() - 0.5).abs() < 1.0e-15);
    assert!((hour_angle.as_hours() - 22.5).abs() < 1.0e-14);
    assert!((azimuth.as_degrees() - 5.0).abs() < 1.0e-13);
    assert!((altitude.as_degrees() + zenith_distance.as_degrees() - 90.0).abs() < 1.0e-13);

    println!(
        "Vega    RA={:.5} Dec={:.4}",
        vega.right_ascension().to_hms(),
        vega.declination().to_dms()
    );
    println!(
        "Polaris RA={:.5} Dec={:.3}",
        polaris.right_ascension().to_hms(),
        polaris.declination().to_dms()
    );
    println!(
        "Vega galactic l={:.9}° b={:.9}°",
        vega_galactic.longitude().as_degrees(),
        vega_galactic.latitude().as_degrees()
    );
    println!(
        "Vega ecliptic l={:.9}° b={:.9}°",
        vega_ecliptic.longitude().as_degrees(),
        vega_ecliptic.latitude().as_degrees()
    );
    println!(
        "Vega mean J2000 α={:.9}° δ={:.9}°",
        vega_mean_j2000.right_ascension().as_degrees(),
        vega_mean_j2000.declination().as_degrees()
    );
    println!("separation     = {:.9}°", separation.as_degrees());
    println!("position angle = {:.9}°", position_angle.as_degrees());
    println!(
        "midpoint       = {:.3} {:.3}",
        midpoint_equatorial.right_ascension().to_hms(),
        midpoint_equatorial.declination().to_dms()
    );
    println!("wrapped HA     = {:.3} h", hour_angle.as_hours());
    println!("wrapped az     = {:.3}°", azimuth.as_degrees());
    println!("phase angle    = {:.3}°", phase_angle.as_degrees());

    Ok(())
}
