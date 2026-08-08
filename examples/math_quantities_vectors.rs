use hyastro::{
    frame::Icrs,
    math::{Acceleration, Dimensionless, Length, Point3, Speed, Vector3},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let one_au = Length::from_astronomical_units(1.0)?;
    let radial_speed = Speed::from_kilometres_per_second(29.78)?;
    let acceleration = Acceleration::from_metres_per_second_squared(0.005_93)?;
    let scale = Dimensionless::new(0.5)?;

    println!("distance     = {:.3} km", one_au.as_kilometres());
    println!(
        "radial speed = {:.6} au/day",
        radial_speed.as_astronomical_units_per_day()
    );
    println!(
        "acceleration = {:.5} m/s²",
        acceleration.as_metres_per_second_squared()
    );
    println!("scale        = {}", scale.value());

    let position = Vector3::<Icrs, Length>::new(
        Length::from_metres(1.0)?,
        Length::from_metres(2.0)?,
        Length::from_metres(3.0)?,
    );
    let offset = Vector3::<Icrs, Length>::new(
        Length::from_metres(4.0)?,
        Length::from_metres(-5.0)?,
        Length::from_metres(6.0)?,
    );

    let translated = position.checked_add(offset)?;
    let dot = position.dot(offset)?;
    let cross = position.cross(offset)?;
    let direction = position.direction()?;
    let parallel = offset.project_onto(direction)?;
    let perpendicular = offset.reject_from(direction)?;

    assert_eq!(translated.x().as_metres(), 5.0);
    assert_eq!(translated.y().as_metres(), -3.0);
    assert_eq!(translated.z().as_metres(), 9.0);
    assert_eq!(dot.value(), 12.0);
    assert_eq!(cross.x().value(), 27.0);
    assert_eq!(cross.y().value(), 6.0);
    assert_eq!(cross.z().value(), -13.0);
    assert!((parallel.dot(perpendicular)?.value()).abs() < 1.0e-14);

    let origin = Point3::<Icrs>::new(
        Length::from_metres(10.0)?,
        Length::from_metres(20.0)?,
        Length::from_metres(30.0)?,
    );
    let moved = origin.checked_translate(position)?;
    let recovered_displacement = moved.displacement_from(origin)?;
    assert_eq!(recovered_displacement, position);

    println!("|position|   = {:.9} m", position.magnitude()?.as_metres());
    println!("position dir = {:?}", direction.components());
    println!("translated   = {:?}", translated.components());
    println!("moved point  = {:?}", moved.position().components());

    Ok(())
}
