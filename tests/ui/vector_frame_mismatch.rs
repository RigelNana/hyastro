use hyastro::math::{Length, Vector3};

struct Icrs;
struct Galactic;

fn main() {
    let icrs = Vector3::<Icrs, Length>::new(
        Length::from_metres(1.0).unwrap(),
        Length::from_metres(2.0).unwrap(),
        Length::from_metres(3.0).unwrap(),
    );
    let galactic = Vector3::<Galactic, Length>::new(
        Length::from_metres(1.0).unwrap(),
        Length::from_metres(2.0).unwrap(),
        Length::from_metres(3.0).unwrap(),
    );

    let _ = icrs.checked_add(galactic);
}
