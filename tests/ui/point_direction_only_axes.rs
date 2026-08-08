use hyastro::{
    frame::Galactic,
    math::{Length, Point3},
};

fn main() {
    let _ = Point3::<Galactic>::new(
        Length::from_metres(1.0).unwrap(),
        Length::from_metres(2.0).unwrap(),
        Length::from_metres(3.0).unwrap(),
    );
}
