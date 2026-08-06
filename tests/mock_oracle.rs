use approx::assert_abs_diff_eq;
use hyastro::math::Angle;
use mockall::{automock, predicate::eq};

#[automock]
trait TrigonometricOracle {
    fn sin_cos(&self, radians: f64) -> (f64, f64);
}

#[test]
fn angle_matches_an_injected_reference_oracle() {
    let radians = core::f64::consts::FRAC_PI_6;
    let mut oracle = MockTrigonometricOracle::new();
    oracle
        .expect_sin_cos()
        .with(eq(radians))
        .times(1)
        .return_const((0.5, 3.0_f64.sqrt() * 0.5));

    let expected = oracle.sin_cos(radians);
    let actual = Angle::from_radians(radians).unwrap().sin_cos();
    assert_abs_diff_eq!(actual.0.value(), expected.0, epsilon = 1.0e-15);
    assert_abs_diff_eq!(actual.1.value(), expected.1, epsilon = 1.0e-15);
}
