use hyastro::math::{Ab, ApparentMagnitude, JohnsonV, Vega};

fn main() {
    let vega = ApparentMagnitude::<JohnsonV, Vega>::from_magnitudes(1.0).unwrap();
    let ab = ApparentMagnitude::<JohnsonV, Ab>::from_magnitudes(1.0).unwrap();
    let _ = vega.difference_from(ab);
}
