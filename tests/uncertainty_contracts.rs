use hyastro::{
    math::{Angle, Length},
    time::Duration,
    uncertainty::{CorrelationMatrix, Error, StandardUncertainty},
};

#[test]
fn standard_uncertainty_preserves_quantity_and_rejects_negative_values() {
    let angular = StandardUncertainty::new(Angle::from_degrees(0.25).unwrap()).unwrap();
    assert_eq!(angular.value().as_degrees(), 0.25);
    assert!(!angular.is_zero());

    let zero = StandardUncertainty::new(Duration::ZERO).unwrap();
    assert!(zero.is_zero());
    assert_eq!(zero.value(), Duration::ZERO);

    let length = StandardUncertainty::new(Length::from_metres(12.5).unwrap()).unwrap();
    assert_eq!(length.value().as_metres(), 12.5);

    assert!(matches!(
        StandardUncertainty::new(Angle::from_degrees(-0.25).unwrap()),
        Err(Error::NegativeStandardUncertainty {
            quantity: "angle",
            unit: "rad",
            ..
        })
    ));
    assert!(matches!(
        StandardUncertainty::new(Duration::from_nanoseconds(-1)),
        Err(Error::NegativeStandardUncertainty {
            quantity: "duration",
            unit: "s",
            ..
        })
    ));
}

#[test]
fn correlation_matrix_canonicalizes_valid_input_and_accepts_singular_psd_cases() {
    let almost_symmetric = [
        [1.0, 0.25, -0.10],
        [0.25 + 5.0e-13, 1.0, 0.30],
        [-0.10, 0.30, 1.0],
    ];
    let matrix = CorrelationMatrix::try_from_coefficients(almost_symmetric).unwrap();
    assert_eq!(matrix.coefficient(0, 0), Some(1.0));
    assert_eq!(matrix.coefficient(0, 1), matrix.coefficient(1, 0));
    assert_eq!(matrix.coefficient(3, 0), None);

    let singular = CorrelationMatrix::try_from_coefficients([
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    ])
    .unwrap();
    assert_eq!(singular.coefficient(1, 2), Some(1.0));
}

#[test]
fn correlation_matrix_rejects_asymmetry_bounds_and_non_psd_input() {
    assert!(matches!(
        CorrelationMatrix::try_from_coefficients([[1.0, 0.2], [0.3, 1.0]]),
        Err(Error::AsymmetricCorrelation { .. })
    ));
    assert!(matches!(
        CorrelationMatrix::try_from_coefficients([[1.0, 1.1], [1.1, 1.0]]),
        Err(Error::InvalidCorrelationCoefficient { .. })
    ));
    assert!(matches!(
        CorrelationMatrix::try_from_coefficients([
            [1.0, 0.9, 0.9],
            [0.9, 1.0, -0.9],
            [0.9, -0.9, 1.0],
        ]),
        Err(Error::CorrelationMatrixNotPositiveSemidefinite { .. })
    ));
}
