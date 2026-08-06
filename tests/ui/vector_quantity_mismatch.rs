use hyastro::math::{Length, Speed, Vector3};

struct Icrs;

fn main() {
    let position = Vector3::<Icrs, Length>::new(
        Length::from_metres(1.0).unwrap(),
        Length::from_metres(2.0).unwrap(),
        Length::from_metres(3.0).unwrap(),
    );
    let velocity = Vector3::<Icrs, Speed>::new(
        Speed::from_metres_per_second(1.0).unwrap(),
        Speed::from_metres_per_second(2.0).unwrap(),
        Speed::from_metres_per_second(3.0).unwrap(),
    );

    let _ = position.checked_add(velocity);
}
