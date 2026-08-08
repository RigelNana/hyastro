use hyastro::{
    frame::{EquatorialDirection, Itrs},
    math::{Declination, RightAscension},
};

fn main() {
    let _ = EquatorialDirection::<Itrs>::new(
        RightAscension::try_from_degrees(12.0).unwrap(),
        Declination::try_from_degrees(-30.0).unwrap(),
    );
}
