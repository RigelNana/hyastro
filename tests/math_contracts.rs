use core::f64::consts::{FRAC_PI_2, PI, TAU};

use approx::{assert_abs_diff_eq, assert_relative_eq};
use garde::Validate;
use hyastro::math::{
    Angle, Declination, Direction, EquatorialDirection, Error, Latitude, Length, Longitude,
    Matrix3, PositionAngle, Quaternion, RightAscension, RootOptions, Rotation, RotationTolerance,
    Separation, SphericalDirection, Vector3,
};
use proptest::prelude::*;
use rstest::rstest;

#[derive(Debug)]
struct Icrs;

#[test]
fn length_conversions_share_one_canonical_unit() {
    let astronomical_unit = Length::from_astronomical_units(1.0).unwrap();
    assert_abs_diff_eq!(
        astronomical_unit.as_metres(),
        Length::METRES_PER_AU,
        epsilon = 0.0
    );
    assert_relative_eq!(
        astronomical_unit.as_light_seconds(),
        499.004_783_836_156_4,
        max_relative = 1.0e-15
    );
    assert!(matches!(
        Length::from_metres(f64::NAN),
        Err(Error::NonFinite { .. })
    ));
}

#[rstest]
#[case(-FRAC_PI_2)]
#[case(0.0)]
#[case(FRAC_PI_2)]
fn latitude_accepts_closed_interval_boundaries(#[case] radians: f64) {
    assert_eq!(
        Latitude::try_from_radians(radians).unwrap().as_radians(),
        radians
    );
}

#[rstest]
#[case(-FRAC_PI_2 - f64::EPSILON)]
#[case(FRAC_PI_2 + f64::EPSILON)]
fn latitude_rejects_values_outside_its_semantic_interval(#[case] radians: f64) {
    assert!(matches!(
        Latitude::try_from_radians(radians),
        Err(Error::OutOfRange { .. })
    ));
}

proptest! {
    #[test]
    fn cyclic_angles_are_normalized_into_their_declared_intervals(turns in -1_000_i32..1_000, offset in -PI..PI) {
        let input = f64::from(turns) * TAU + offset;
        let right_ascension = RightAscension::wrap_radians(input).unwrap();
        prop_assert!(right_ascension.as_radians() >= 0.0);
        prop_assert!(right_ascension.as_radians() < TAU);

        let longitude = Longitude::wrap_radians(input).unwrap();
        prop_assert!(longitude.as_radians() > -PI);
        prop_assert!(longitude.as_radians() <= PI);
    }
}

#[test]
fn vector_arithmetic_preserves_frame_and_quantity() {
    let left = Vector3::<Icrs, Length>::new(
        Length::from_metres(1.0).unwrap(),
        Length::from_metres(2.0).unwrap(),
        Length::from_metres(3.0).unwrap(),
    );
    let right = Vector3::<Icrs, Length>::new(
        Length::from_metres(4.0).unwrap(),
        Length::from_metres(-5.0).unwrap(),
        Length::from_metres(6.0).unwrap(),
    );

    assert_abs_diff_eq!(left.dot(right).unwrap().value(), 12.0, epsilon = 0.0);
    assert_abs_diff_eq!(
        left.magnitude().unwrap().as_metres(),
        14.0_f64.sqrt(),
        epsilon = 1.0e-15
    );
    assert_eq!(
        left.checked_add(right).unwrap().components(),
        [
            Length::from_metres(5.0).unwrap(),
            Length::from_metres(-3.0).unwrap(),
            Length::from_metres(9.0).unwrap(),
        ]
    );
}

#[test]
fn zero_vector_cannot_become_a_direction() {
    assert_eq!(
        Direction::<Icrs>::try_from_components([0.0, 0.0, 0.0]),
        Err(Error::ZeroVector)
    );
}

#[test]
fn matrix_inverse_restores_identity() {
    let matrix =
        Matrix3::try_from_rows([[2.0, -1.0, 0.0], [1.0, 2.0, 1.0], [0.0, 3.0, 1.0]]).unwrap();
    let product = matrix.checked_mul(matrix.inverse().unwrap()).unwrap();
    for row in 0..3 {
        for column in 0..3 {
            let expected = if row == column { 1.0 } else { 0.0 };
            assert_abs_diff_eq!(product.rows()[row][column], expected, epsilon = 1.0e-14);
        }
    }
    assert_eq!(product.element(0, 0), Some(product.rows()[0][0]));
    assert_eq!(product.element(3, 0), None);
    assert_eq!(product.element(0, 3), None);
}

#[test]
fn rotation_and_quaternion_round_trips_preserve_vectors() {
    let rotation = Rotation::<Icrs, Icrs>::around_z(Angle::from_degrees(90.0).unwrap()).unwrap();
    let vector = Vector3::<Icrs, Length>::new(
        Length::from_metres(2.0).unwrap(),
        Length::from_metres(0.0).unwrap(),
        Length::from_metres(0.0).unwrap(),
    );
    let rotated = rotation.apply_vector(vector).unwrap();
    assert_abs_diff_eq!(rotated.x().as_metres(), 0.0, epsilon = 2.0e-16);
    assert_abs_diff_eq!(rotated.y().as_metres(), 2.0, epsilon = 2.0e-16);

    let quaternion = Quaternion::from_rotation(rotation).unwrap();
    let restored = quaternion
        .inverse()
        .apply_vector(quaternion.apply_vector(vector).unwrap())
        .unwrap();
    assert_abs_diff_eq!(restored.x().as_metres(), 2.0, epsilon = 1.0e-14);
    assert_abs_diff_eq!(restored.y().as_metres(), 0.0, epsilon = 1.0e-14);
    assert_abs_diff_eq!(restored.z().as_metres(), 0.0, epsilon = 1.0e-14);
}

#[test]
fn invalid_rotation_matrix_reports_both_invariants() {
    let matrix =
        Matrix3::try_from_rows([[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]]).unwrap();
    let tolerance = RotationTolerance::new(1.0e-12, 1.0e-12).unwrap();
    assert!(matches!(
        Rotation::<Icrs, Icrs>::try_from_matrix(matrix, tolerance),
        Err(Error::InvalidRotation { .. })
    ));
    assert!(tolerance.validate().is_ok());
}

#[test]
fn spherical_distance_is_stable_near_zero_and_pi() {
    let origin = SphericalDirection::<Icrs>::new(
        Longitude::try_from_radians(0.0).unwrap(),
        Latitude::try_from_radians(0.0).unwrap(),
    );
    let near = SphericalDirection::<Icrs>::new(
        Longitude::try_from_radians(1.0e-12).unwrap(),
        Latitude::try_from_radians(0.0).unwrap(),
    );
    let opposite = SphericalDirection::<Icrs>::new(
        Longitude::try_from_radians(PI).unwrap(),
        Latitude::try_from_radians(0.0).unwrap(),
    );

    assert_relative_eq!(
        origin.separation_to(near).unwrap().as_radians(),
        1.0e-12,
        max_relative = 1.0e-12
    );
    assert_abs_diff_eq!(
        origin.separation_to(opposite).unwrap().as_radians(),
        PI,
        epsilon = 1.0e-15
    );
    assert_eq!(origin.slerp(opposite, 0.5), Err(Error::AntipodalDirections));
}

#[test]
fn destination_inverts_separation_and_position_angle() {
    let start = SphericalDirection::<Icrs>::new(
        Longitude::try_from_degrees(15.0).unwrap(),
        Latitude::try_from_degrees(-20.0).unwrap(),
    );
    let separation = Separation::try_from_degrees(37.0).unwrap();
    let bearing = PositionAngle::try_from_degrees(123.0).unwrap();
    let destination = start.destination(separation, bearing).unwrap();

    assert_abs_diff_eq!(
        start.separation_to(destination).unwrap().as_radians(),
        separation.as_radians(),
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        start.position_angle_to(destination).unwrap().as_radians(),
        bearing.as_radians(),
        epsilon = 1.0e-14
    );
}

#[test]
fn equatorial_and_cartesian_directions_round_trip() {
    let equatorial = EquatorialDirection::<Icrs>::new(
        RightAscension::try_from_degrees(201.25).unwrap(),
        Declination::try_from_degrees(-47.5).unwrap(),
    );
    let restored = EquatorialDirection::from_direction(equatorial.to_direction().unwrap()).unwrap();
    assert_abs_diff_eq!(
        restored.right_ascension().as_radians(),
        equatorial.right_ascension().as_radians(),
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        restored.declination().as_radians(),
        equatorial.declination().as_radians(),
        epsilon = 1.0e-15
    );
}

#[test]
fn bisection_returns_convergence_evidence() {
    let options = RootOptions::new(1.0e-14, 1.0e-14, 100).unwrap();
    assert!(options.validate().is_ok());
    let result = options.bisect(0.0, 2.0, |x| x * x - 2.0).unwrap();
    assert_abs_diff_eq!(result.root(), 2.0_f64.sqrt(), epsilon = 1.0e-13);
    assert!(result.residual().abs() <= 1.0e-13);
    assert!(result.iterations() > 0);
    assert!(result.lower() <= result.root());
    assert!(result.root() <= result.upper());
}

#[test]
fn bisection_rejects_non_bracketing_interval() {
    let options = RootOptions::new(1.0e-12, 1.0e-12, 32).unwrap();
    assert!(matches!(
        options.bisect(-1.0, 1.0, |x| x * x + 1.0),
        Err(Error::NotBracketed { .. })
    ));
}
