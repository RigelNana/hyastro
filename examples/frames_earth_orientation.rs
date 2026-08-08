use hyastro::{
    frame::{Cirs, CoordinateFrame, EquatorialDirection, Frames, Gcrs, Icrs, Itrs, State, Tirs},
    math::{Direction, Length, Point3, Speed, Vector3},
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
    let eop = EarthOrientationTable::new(&samples, "IERS C04 frame example", expires)?;
    let time = base.with_earth_orientation(eop);
    let epoch = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2007, 10, 15, 12, 0, 0, 0,
    )?)?;
    let frames = Frames::new(&time);
    let solution = frames.earth_orientation_at(epoch)?;

    assert_eq!(Icrs::definition().name(), "ICRS");
    assert_eq!(Gcrs::definition().name(), "GCRS");
    assert_eq!(Cirs::definition().name(), "CIRS");
    assert_eq!(Tirs::definition().name(), "TIRS");
    assert_eq!(Itrs::definition().name(), "ITRS");

    let modeled_cio = solution.modeled_cio_gcrs_to_tirs_matrix();
    let equinox = solution.equinox_gcrs_to_tirs_matrix();
    for row in 0..3 {
        for column in 0..3 {
            assert!(
                (modeled_cio.rows()[row][column] - equinox.rows()[row][column]).abs() < 2.0e-15
            );
        }
    }

    let line_of_sight = Direction::<Gcrs>::try_from_components([1.0, 2.0, 3.0])?;
    let gcrs_equatorial = EquatorialDirection::from_direction(line_of_sight)?;
    let cirs_equatorial = solution.intermediate_equatorial(gcrs_equatorial)?;
    assert!(
        solution
            .gcrs_from_intermediate_equatorial(cirs_equatorial)?
            .separation_to(gcrs_equatorial)?
            .as_radians()
            < 1.0e-14
    );

    let terrestrial_line_of_sight = solution.gcrs_to_itrs().apply_direction(line_of_sight)?;
    assert!((terrestrial_line_of_sight.dot(terrestrial_line_of_sight) - 1.0).abs() < 1.0e-15);

    let gcrs_state = State::<Gcrs, Utc>::new(
        Point3::new(
            Length::from_kilometres(6_378.0)?,
            Length::from_kilometres(1.0)?,
            Length::from_kilometres(2.0)?,
        ),
        Vector3::new(
            Speed::from_kilometres_per_second(0.0)?,
            Speed::from_kilometres_per_second(7.5)?,
            Speed::from_kilometres_per_second(1.0)?,
        ),
        epoch,
    );
    let itrs_state: State<Itrs, Utc> = frames.transform(gcrs_state)?;
    let recovered: State<Gcrs, Utc> = frames.transform(itrs_state)?;
    let recovered_position = recovered.position().position();
    let original_position = gcrs_state.position().position();
    let recovered_velocity = recovered.velocity();
    let original_velocity = gcrs_state.velocity();

    assert!(
        (recovered_position.x().as_metres() - original_position.x().as_metres()).abs() < 1.0e-8
    );
    assert!(
        (recovered_position.y().as_metres() - original_position.y().as_metres()).abs() < 1.0e-8
    );
    assert!(
        (recovered_position.z().as_metres() - original_position.z().as_metres()).abs() < 1.0e-8
    );
    assert!(
        (recovered_velocity.x().as_metres_per_second()
            - original_velocity.x().as_metres_per_second())
        .abs()
            < 1.0e-9
    );
    assert!(
        (recovered_velocity.y().as_metres_per_second()
            - original_velocity.y().as_metres_per_second())
        .abs()
            < 1.0e-9
    );
    assert!(
        (recovered_velocity.z().as_metres_per_second()
            - original_velocity.z().as_metres_per_second())
        .abs()
            < 1.0e-9
    );

    let state_transform = frames.at::<Gcrs, Itrs, Utc>(epoch)?;
    let cip = solution.cip();
    let precession_nutation = solution.precession_nutation();
    let itrs_position = itrs_state.position().position();
    let itrs_velocity = itrs_state.velocity();

    println!("epoch                 = {:?}", solution.epoch());
    println!(
        "TT                    = {:?}",
        solution.terrestrial_time().parts()
    );
    println!(
        "UT1                   = {:?}",
        solution.universal_time().parts()
    );
    println!(
        "CIP (X,Y)             = ({:.12e}, {:.12e}) rad",
        cip.x().as_radians(),
        cip.y().as_radians()
    );
    println!(
        "CIO locator s         = {:.12e} rad",
        solution.cio_locator().as_radians()
    );
    println!(
        "TIO locator s′        = {:.12e} rad",
        solution.tio_locator().as_radians()
    );
    println!(
        "ERA / GMST / GAST     = {:.12} / {:.12} / {:.12} rad",
        solution.earth_rotation_angle().as_radians(),
        solution.greenwich_mean_sidereal_time().as_radians(),
        solution.greenwich_apparent_sidereal_time().as_radians()
    );
    println!(
        "CIRS α / δ             = {:.12} / {:.12} rad",
        cirs_equatorial.coordinates().right_ascension().as_radians(),
        cirs_equatorial.coordinates().declination().as_radians()
    );
    println!(
        "BPN determinant       = {:.15}",
        precession_nutation
            .bias_precession_nutation_matrix()
            .determinant()
    );
    println!(
        "GCRS→ITRS determinant = {:.15}",
        solution.gcrs_to_itrs().rotation().matrix().determinant()
    );
    println!(
        "angular velocity      = {:?}",
        state_transform.angular_velocity().components()
    );
    println!(
        "ITRS position         = [{:.3}, {:.3}, {:.3}] m",
        itrs_position.x().as_metres(),
        itrs_position.y().as_metres(),
        itrs_position.z().as_metres()
    );
    println!(
        "ITRS velocity         = [{:.6}, {:.6}, {:.6}] m/s",
        itrs_velocity.x().as_metres_per_second(),
        itrs_velocity.y().as_metres_per_second(),
        itrs_velocity.z().as_metres_per_second()
    );

    Ok(())
}
