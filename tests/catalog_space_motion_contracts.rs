use approx::assert_abs_diff_eq;
use hyastro::{
    catalog::{
        BarycentricCatalogState, CatalogProperMotion, CatalogRadialVelocity, Error, Parallax,
        ParallaxMeasurement, SpatialCatalogCovariance, SpatialCatalogParameter,
        SpatialCatalogPlace, SpatialCatalogStandardUncertainties,
    },
    frame::{Bcrs, EquatorialDirection, Icrs},
    math::{Angle, Declination, Length, RightAscension, Speed, Vector3},
    time::{JulianDate, Tcb},
    uncertainty::{CorrelationMatrix, StandardUncertainty},
};

struct SofaReference;

impl SofaReference {
    const RIGHT_ASCENSION: f64 = 0.016_867_56;
    const DECLINATION: f64 = -1.093_989_828;
    const RIGHT_ASCENSION_RATE: f64 = -1.783_235_16e-5;
    const DECLINATION_RATE: f64 = 2.336_024_047e-6;
    const PARALLAX_ARCSECONDS: f64 = 0.747_23;
    const RADIAL_VELOCITY_KILOMETRES_PER_SECOND: f64 = -21.6;

    fn catalog() -> SpatialCatalogPlace {
        let declination = Declination::try_from_radians(Self::DECLINATION).unwrap();
        SpatialCatalogPlace::new(
            JulianDate::<Tcb>::from_parts(2_400_000.5, 50_083.0).unwrap(),
            EquatorialDirection::<Icrs>::new(
                RightAscension::wrap_radians(Self::RIGHT_ASCENSION).unwrap(),
                declination,
            ),
            CatalogProperMotion::from_radians_per_julian_year(
                Self::RIGHT_ASCENSION_RATE * declination.as_radians().cos(),
                Self::DECLINATION_RATE,
            )
            .unwrap(),
            Parallax::from_arcseconds(Self::PARALLAX_ARCSECONDS).unwrap(),
            CatalogRadialVelocity::from_kilometres_per_second(
                Self::RADIAL_VELOCITY_KILOMETRES_PER_SECOND,
            )
            .unwrap(),
        )
    }
}

#[test]
fn signed_parallax_measurement_requires_explicit_physical_interpretation() {
    let negative = ParallaxMeasurement::from_milliarcseconds(-0.35, 0.12).unwrap();
    assert_abs_diff_eq!(negative.as_milliarcseconds(), -0.35, epsilon = 1.0e-15);
    assert_abs_diff_eq!(
        negative.standard_uncertainty_milliarcseconds(),
        0.12,
        epsilon = 1.0e-15
    );
    assert!(matches!(
        negative.try_physical(),
        Err(Error::InvalidPhysicalParallax { .. })
    ));
    assert!(matches!(
        ParallaxMeasurement::from_milliarcseconds(1.0, -0.1),
        Err(Error::Uncertainty(_))
    ));

    let physical = ParallaxMeasurement::from_milliarcseconds(548.31, 0.05)
        .unwrap()
        .try_physical()
        .unwrap();
    assert_abs_diff_eq!(physical.as_milliarcseconds(), 548.31, epsilon = 1.0e-12);
}

#[test]
fn sofa_catalog_to_state_matches_the_official_starpv_reference() {
    let catalog = SofaReference::catalog();
    let state = catalog.barycentric_state().unwrap();
    let [x, y, z] = state.position().components();
    let [vx, vy, vz] = state.velocity().components();

    assert_abs_diff_eq!(
        x.as_astronomical_units(),
        126_668.591_274_316_06,
        epsilon = 1.0e-10
    );
    assert_abs_diff_eq!(
        y.as_astronomical_units(),
        2_136.792_716_839_935,
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        z.as_astronomical_units(),
        -245_251.233_987_683,
        epsilon = 1.0e-10
    );
    assert_abs_diff_eq!(
        vx.as_astronomical_units_per_day(),
        -0.004_051_854_008_955_66,
        epsilon = 1.0e-13
    );
    assert_abs_diff_eq!(
        vy.as_astronomical_units_per_day(),
        -0.006_253_919_754_414_778,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        vz.as_astronomical_units_per_day(),
        0.011_893_537_145_881_094,
        epsilon = 1.0e-13
    );

    let round_trip = state.catalog_place().unwrap();
    assert_abs_diff_eq!(
        round_trip.direction().right_ascension().as_radians(),
        SofaReference::RIGHT_ASCENSION,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        round_trip.direction().declination().as_radians(),
        SofaReference::DECLINATION,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        round_trip.parallax().as_arcseconds(),
        SofaReference::PARALLAX_ARCSECONDS,
        epsilon = 1.0e-13
    );
    assert_abs_diff_eq!(
        round_trip.radial_velocity().as_kilometres_per_second(),
        SofaReference::RADIAL_VELOCITY_KILOMETRES_PER_SECOND,
        epsilon = 1.0e-10
    );
}

#[test]
fn full_space_motion_matches_the_official_starpm_reference_and_reverses() {
    let catalog = SofaReference::catalog();
    let propagated = catalog
        .propagate_to(JulianDate::<Tcb>::from_parts(2_400_000.5, 53_736.0).unwrap())
        .unwrap();
    let direction = propagated.direction();

    assert_abs_diff_eq!(
        direction.right_ascension().as_radians(),
        0.016_689_190_694_142_56,
        epsilon = 1.0e-13
    );
    assert_abs_diff_eq!(
        direction.declination().as_radians(),
        -1.093_966_454_217_128,
        epsilon = 1.0e-13
    );
    assert_abs_diff_eq!(
        propagated
            .proper_motion()
            .right_ascension_radians_per_julian_year_at(direction.declination())
            .unwrap(),
        -1.783_662_682_153_176_5e-5,
        epsilon = 1.0e-17
    );
    assert_abs_diff_eq!(
        propagated
            .proper_motion()
            .declination_radians_per_julian_year(),
        2.338_092_915_983_989_6e-6,
        epsilon = 1.0e-17
    );
    assert_abs_diff_eq!(
        propagated.parallax().as_arcseconds(),
        0.747_353_383_531_771_9,
        epsilon = 1.0e-13
    );
    assert_abs_diff_eq!(
        propagated.radial_velocity().as_kilometres_per_second(),
        -21.599_051_704_764_17,
        epsilon = 1.0e-11
    );

    let restored = propagated.propagate_to(catalog.reference_epoch()).unwrap();
    assert!(
        restored
            .direction()
            .separation_to(catalog.direction())
            .unwrap()
            .as_radians()
            <= 1.0e-15
    );
    assert_abs_diff_eq!(
        restored.parallax().as_arcseconds(),
        catalog.parallax().as_arcseconds(),
        epsilon = 1.0e-13
    );
}

#[test]
fn six_parameter_covariance_propagates_in_fixed_units_and_reverses() {
    let catalog = SofaReference::catalog();
    let radians_per_milliarcsecond = core::f64::consts::PI / 648_000_000.0;
    let proper_motion_uncertainties =
        CatalogProperMotion::from_milliarcseconds_per_julian_year(0.08, 0.06).unwrap();
    let uncertainties = SpatialCatalogStandardUncertainties::new(
        StandardUncertainty::new(Angle::from_radians(0.12 * radians_per_milliarcsecond).unwrap())
            .unwrap(),
        StandardUncertainty::new(Angle::from_radians(0.10 * radians_per_milliarcsecond).unwrap())
            .unwrap(),
        StandardUncertainty::new(Angle::from_radians(0.05 * radians_per_milliarcsecond).unwrap())
            .unwrap(),
        StandardUncertainty::new(proper_motion_uncertainties.right_ascension_cos_declination())
            .unwrap(),
        StandardUncertainty::new(proper_motion_uncertainties.declination()).unwrap(),
        StandardUncertainty::new(Speed::from_metres_per_second(300.0).unwrap()).unwrap(),
    );
    let mut coefficients = CorrelationMatrix::<6>::identity().coefficients();
    coefficients[0][1] = -0.20;
    coefficients[1][0] = -0.20;
    coefficients[2][3] = 0.30;
    coefficients[3][2] = 0.30;
    let covariance = SpatialCatalogCovariance::new(
        uncertainties,
        CorrelationMatrix::try_from_coefficients(coefficients).unwrap(),
    )
    .unwrap();
    let measured = catalog.with_covariance(covariance).unwrap();

    let identity = measured.propagate_to(catalog.reference_epoch()).unwrap();
    assert_eq!(identity.result(), measured);
    assert_eq!(
        identity.jacobian().canonical_derivative(
            SpatialCatalogParameter::Parallax,
            SpatialCatalogParameter::Parallax,
        ),
        1.0
    );

    let epoch = JulianDate::<Tcb>::from_parts(2_400_000.5, 53_736.0).unwrap();
    let propagation = measured.propagate_to(epoch).unwrap();
    let propagated = propagation.result().covariance();
    let propagated_uncertainties = propagated.standard_uncertainties();
    assert_abs_diff_eq!(
        propagated_uncertainties
            .right_ascension_tangent_plane()
            .value()
            .as_radians()
            / radians_per_milliarcsecond,
        0.810_062,
        epsilon = 5.0e-6
    );
    assert_abs_diff_eq!(
        propagated_uncertainties.parallax().value().as_radians() / radians_per_milliarcsecond,
        0.050_046,
        epsilon = 5.0e-6
    );
    assert_abs_diff_eq!(
        propagated.correlation(
            SpatialCatalogParameter::Parallax,
            SpatialCatalogParameter::RightAscensionProperMotion,
        ),
        0.294_707,
        epsilon = 5.0e-6
    );
    assert_abs_diff_eq!(
        propagation.jacobian().canonical_derivative(
            SpatialCatalogParameter::RightAscensionTangentPlane,
            SpatialCatalogParameter::RightAscensionProperMotion,
        ),
        3.156_713e8,
        epsilon = 5.0e4
    );

    let restored = propagation
        .result()
        .propagate_to(catalog.reference_epoch())
        .unwrap()
        .result()
        .covariance();
    let restored_uncertainties = restored.standard_uncertainties();
    assert_abs_diff_eq!(
        restored_uncertainties
            .right_ascension_tangent_plane()
            .value()
            .as_radians(),
        uncertainties
            .right_ascension_tangent_plane()
            .value()
            .as_radians(),
        epsilon = 5.0e-8 * radians_per_milliarcsecond
    );
    assert_abs_diff_eq!(
        restored.correlation(
            SpatialCatalogParameter::Parallax,
            SpatialCatalogParameter::RightAscensionProperMotion,
        ),
        0.30,
        epsilon = 2.0e-5
    );
}

#[test]
fn invalid_space_motion_is_rejected_instead_of_using_sofa_fallbacks() {
    let zero = Length::from_metres(0.0).unwrap();
    let stationary = Speed::from_metres_per_second(0.0).unwrap();
    assert!(matches!(
        BarycentricCatalogState::new(
            JulianDate::<Tcb>::from_j2000_offset_days(0.0).unwrap(),
            Vector3::<Bcrs, Length>::new(zero, zero, zero),
            Vector3::<Bcrs, Speed>::new(stationary, stationary, stationary),
        ),
        Err(Error::NullBarycentricPosition)
    ));

    let tiny_parallax = SpatialCatalogPlace::new(
        SofaReference::catalog().reference_epoch(),
        SofaReference::catalog().direction(),
        SofaReference::catalog().proper_motion(),
        Parallax::from_arcseconds(1.0e-8).unwrap(),
        SofaReference::catalog().radial_velocity(),
    );
    assert!(matches!(
        tiny_parallax.barycentric_state(),
        Err(Error::SpaceMotionFallbackRejected { status, .. }) if status > 0
    ));
}
