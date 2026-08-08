use hyastro::frame::{
    EclipticDirection, EclipticLatitude, GalacticLongitude, MeanEclipticEquinoxJ2000,
};

fn main() {
    let _ = EclipticDirection::<MeanEclipticEquinoxJ2000>::new(
        GalacticLongitude::try_from_degrees(120.0).unwrap(),
        EclipticLatitude::try_from_degrees(-30.0).unwrap(),
    );
}
