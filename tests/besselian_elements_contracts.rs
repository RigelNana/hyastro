use approx::assert_abs_diff_eq;
use hyastro::{
    astro::Astrometry,
    earth::Earth,
    ephem::SofaAnalyticEphemeris,
    event::{
        BesselianDerivativeMethod, BesselianElementsOptions, BesselianLimbModel,
        BesselianPolynomialOptions, Events, SolarEclipseSearchOptions,
    },
    math::Angle,
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, IersC04,
        ModifiedJulianDate, TimeContext, Tt, Utc,
    },
};

#[cfg(feature = "anise")]
use hyastro::ephem::{Ephemeris, KernelManifest};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");

#[test]
fn analytic_besselian_elements_retain_the_oriented_plane_and_derivatives() {
    let base = TimeContext::builtin();
    let data = IersC04::parse(C04).unwrap();
    let samples = data
        .try_samples_in(
            &base,
            ModifiedJulianDate::<Utc>::from_parts(60_406.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(60_410.0, 0.0).unwrap(),
            EarthOrientationAcceptance::FinalOnly,
        )
        .unwrap();
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let eop = EarthOrientationTable::new(&samples, "C04 Besselian test", expires).unwrap();
    let time = base.with_earth_orientation(eop);
    let epoch = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 4, 8, 18, 0, 0, 0).unwrap())
        .unwrap();
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let earth = Earth::wgs84();
    let search_options = SolarEclipseSearchOptions::standard();
    let element_options = BesselianElementsOptions::physical(earth, search_options);
    let elements = events
        .solar_eclipse_besselian_elements_at(&earth, epoch, element_options)
        .unwrap();
    let derivatives = elements.derivatives();
    let d_degrees_per_hour = derivatives.d_degrees_per_tt_hour();
    let mu_degrees_per_hour = derivatives.mu_degrees_per_tt_hour();

    println!(
        "x={:.9} y={:.9} d={:.9} mu={:.9} l1={:.9} l2={:.9} f1={:.9} f2={:.9}",
        elements.x().as_equatorial_radii(),
        elements.y().as_equatorial_radii(),
        elements.d().as_degrees(),
        elements.mu().as_degrees(),
        elements.l1().as_equatorial_radii(),
        elements.l2().as_equatorial_radii(),
        elements.tan_f1().value(),
        elements.tan_f2().value(),
    );
    println!(
        "dx={:.9} dy={:.9} dd={:.9} dmu={:.9} dl1={:.9} dl2={:.9}",
        derivatives.x().as_per_tt_hour(),
        derivatives.y().as_per_tt_hour(),
        d_degrees_per_hour,
        mu_degrees_per_hour,
        derivatives.l1().as_per_tt_hour(),
        derivatives.l2().as_per_tt_hour(),
    );

    assert_eq!(elements.epoch(), epoch);
    assert_eq!(elements.fundamental_plane().epoch(), epoch);
    assert_eq!(
        elements
            .fundamental_plane()
            .shadow_axis()
            .coordinates()
            .declination(),
        elements.d()
    );
    assert_eq!(elements.earth(), earth);
    assert_eq!(
        elements.limb_model().physical_model(),
        Some(search_options.model())
    );
    assert_eq!(elements.ephemeris().model(), SofaAnalyticEphemeris::MODEL);
    assert_eq!(elements.astrometric_evaluations(), 6);
    assert_eq!(
        derivatives.method(),
        BesselianDerivativeMethod::SymmetricDifference {
            half_step: Duration::from_seconds(60).unwrap(),
        }
    );

    assert_abs_diff_eq!(
        elements.x().as_equatorial_radii(),
        -0.318_157,
        epsilon = 0.002
    );
    assert_abs_diff_eq!(
        elements.y().as_equatorial_radii(),
        0.219_747,
        epsilon = 0.002
    );
    assert_abs_diff_eq!(elements.d().as_degrees(), 7.586_20, epsilon = 0.001);
    assert_abs_diff_eq!(elements.mu().as_degrees(), 89.591_22, epsilon = 0.001);
    assert_abs_diff_eq!(
        elements.l1().as_equatorial_radii(),
        0.535_813,
        epsilon = 0.002
    );
    assert_abs_diff_eq!(
        elements.l2().as_equatorial_radii(),
        -0.010_274,
        epsilon = 0.002
    );
    assert_abs_diff_eq!(elements.tan_f1().value(), 0.004_668_3, epsilon = 5.0e-5);
    assert_abs_diff_eq!(elements.tan_f2().value(), 0.004_645_0, epsilon = 5.0e-5);
    assert_abs_diff_eq!(
        derivatives.x().as_per_tt_hour(),
        0.511_710_5,
        epsilon = 0.001
    );
    assert_abs_diff_eq!(
        derivatives.y().as_per_tt_hour(),
        0.270_958_6,
        epsilon = 0.001
    );
    assert_abs_diff_eq!(d_degrees_per_hour, 0.014_844, epsilon = 1.0e-4);
    assert_abs_diff_eq!(mu_degrees_per_hour, 15.004_084, epsilon = 1.0e-4);
}

#[test]
fn besselian_lunar_position_corrections_are_retained_and_applied() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 4, 8, 18, 0, 0, 0).unwrap())
        .unwrap();
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let earth = Earth::wgs84();
    let standard_model = BesselianLimbModel::nasa_five_millennium();
    let latitude_correction = Angle::from_degrees(0.5 / 3_600.0).unwrap();
    let longitude_correction = Angle::from_degrees(1.0 / 3_600.0).unwrap();
    let corrected_model = BesselianLimbModel::new(
        "test non-zero lunar position correction",
        "test contract",
        standard_model.solar_radius(),
        standard_model.penumbral_lunar_radius(),
        standard_model.umbral_lunar_radius(),
        latitude_correction,
        longitude_correction,
    )
    .unwrap();
    let standard_options = BesselianElementsOptions::nasa_five_millennium();
    let corrected_options =
        BesselianElementsOptions::new(corrected_model, standard_options.light_time());
    let standard = events
        .solar_eclipse_besselian_elements_at(&earth, epoch, standard_options)
        .unwrap();
    let corrected = events
        .solar_eclipse_besselian_elements_at(&earth, epoch, corrected_options)
        .unwrap();

    assert_eq!(
        corrected.limb_model().lunar_latitude_correction(),
        latitude_correction
    );
    assert_eq!(
        corrected.limb_model().lunar_longitude_correction(),
        longitude_correction
    );
    let coordinate_change =
        (corrected.x().as_equatorial_radii() - standard.x().as_equatorial_radii()).abs()
            + (corrected.y().as_equatorial_radii() - standard.y().as_equatorial_radii()).abs();
    assert!(coordinate_change > 1.0e-5, "{coordinate_change}");
}

#[test]
fn analytic_besselian_polynomial_has_bounded_evaluation_and_analytic_rates() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 4, 8, 18, 0, 0, 0).unwrap())
        .unwrap();
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let earth = Earth::wgs84();
    let polynomial = events
        .solar_eclipse_besselian_polynomial(
            &earth,
            epoch,
            BesselianPolynomialOptions::nasa_six_hour(),
        )
        .unwrap();
    let fitted = polynomial.elements_at(epoch).unwrap();
    let direct = events
        .solar_eclipse_besselian_elements_at(
            &earth,
            epoch,
            BesselianElementsOptions::nasa_five_millennium(),
        )
        .unwrap();

    assert_eq!(polynomial.reference_epoch(), epoch);
    assert_eq!(
        polynomial.validity().duration(),
        Duration::from_seconds(6 * 3_600).unwrap()
    );
    assert_eq!(polynomial.astrometric_evaluations(), 10);
    assert_eq!(
        polynomial.limb_model(),
        BesselianLimbModel::nasa_five_millennium()
    );
    assert_eq!(
        polynomial.limb_model().source(),
        BesselianLimbModel::NASA_FIVE_MILLENNIUM_SOURCE
    );
    assert_abs_diff_eq!(
        polynomial
            .limb_model()
            .penumbral_lunar_radius()
            .as_equatorial_radii(),
        0.272_488,
        epsilon = f64::EPSILON
    );
    assert_abs_diff_eq!(
        polynomial
            .limb_model()
            .umbral_lunar_radius()
            .as_equatorial_radii(),
        0.272_281,
        epsilon = f64::EPSILON
    );
    assert_eq!(
        polynomial
            .limb_model()
            .lunar_latitude_correction()
            .as_radians(),
        0.0
    );
    assert_eq!(
        polynomial
            .limb_model()
            .lunar_longitude_correction()
            .as_radians(),
        0.0
    );
    assert_eq!(
        fitted.derivatives().method(),
        BesselianDerivativeMethod::AnalyticPolynomial
    );
    assert_eq!(fitted.derivatives().numerical_time_step(), None);
    assert_eq!(
        polynomial.derivatives_at(epoch).unwrap(),
        fitted.derivatives()
    );
    assert_eq!(fitted.astrometric_evaluations(), 0);
    assert_abs_diff_eq!(
        fitted.x().as_equatorial_radii(),
        direct.x().as_equatorial_radii(),
        epsilon = 2.0e-6
    );
    assert_abs_diff_eq!(
        fitted.y().as_equatorial_radii(),
        direct.y().as_equatorial_radii(),
        epsilon = 2.0e-6
    );
    assert_abs_diff_eq!(
        fitted.l1().as_equatorial_radii(),
        direct.l1().as_equatorial_radii(),
        epsilon = 2.0e-6
    );
    assert_abs_diff_eq!(
        fitted.l2().as_equatorial_radii(),
        direct.l2().as_equatorial_radii(),
        epsilon = 2.0e-6
    );
    assert!(polynomial.residuals().x().as_equatorial_radii() < 2.0e-6);
    assert!(polynomial.residuals().y().as_equatorial_radii() < 2.0e-6);

    let before = polynomial
        .validity()
        .start()
        .checked_sub(Duration::from_nanoseconds(1))
        .unwrap();
    assert!(matches!(
        polynomial.elements_at(before),
        Err(hyastro::event::Error::BesselianPolynomialOutsideValidity { .. })
    ));
    assert!(matches!(
        polynomial.derivatives_at(before),
        Err(hyastro::event::Error::BesselianPolynomialOutsideValidity { .. })
    ));
}

// NASA polynomial Besselian elements for 2024-04-08 at 18:00 TDT:
// https://eclipse.gsfc.nasa.gov/SEbeselm/SEbeselm2001/SE2024Apr08Tbeselm.html
#[cfg(feature = "anise")]
#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-series BSP"]
fn de440_besselian_polynomial_matches_the_nasa_six_hour_table() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Tt>::from_components(2024, 4, 8, 18, 0, 0, 0).unwrap())
        .unwrap();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let earth = Earth::wgs84();
    let polynomial = events
        .solar_eclipse_besselian_polynomial(
            &earth,
            epoch,
            BesselianPolynomialOptions::nasa_six_hour(),
        )
        .unwrap();
    let elements = polynomial.elements_at(epoch).unwrap();
    let derivatives = elements.derivatives();
    let d_degrees_per_hour = derivatives.d_degrees_per_tt_hour();
    let mu_degrees_per_hour = derivatives.mu_degrees_per_tt_hour();

    println!("DE440 Besselian polynomial: {polynomial:#?}");
    assert_eq!(polynomial.astrometric_evaluations(), 10);
    assert_eq!(
        derivatives.method(),
        BesselianDerivativeMethod::AnalyticPolynomial
    );
    assert_abs_diff_eq!(
        polynomial.x().coefficient(2).unwrap(),
        0.000_032_6,
        epsilon = 5.0e-6
    );
    assert_abs_diff_eq!(
        polynomial.x().coefficient(3).unwrap(),
        -0.000_008_5,
        epsilon = 2.0e-6
    );
    assert_abs_diff_eq!(
        polynomial.y().coefficient(2).unwrap(),
        -0.000_059_4,
        epsilon = 5.0e-6
    );
    assert_abs_diff_eq!(
        polynomial.y().coefficient(3).unwrap(),
        -0.000_004_7,
        epsilon = 2.0e-6
    );
    assert_abs_diff_eq!(
        polynomial.d().coefficient_degrees(2).unwrap(),
        -0.000_002,
        epsilon = 2.0e-6
    );
    assert_abs_diff_eq!(
        polynomial.l1().coefficient(2).unwrap(),
        -0.000_012_8,
        epsilon = 2.0e-6
    );
    assert_abs_diff_eq!(
        polynomial.l2().coefficient(2).unwrap(),
        -0.000_012_7,
        epsilon = 2.0e-6
    );
    assert_abs_diff_eq!(
        elements.x().as_equatorial_radii(),
        -0.318_157,
        epsilon = 3.0e-4
    );
    assert_abs_diff_eq!(
        elements.y().as_equatorial_radii(),
        0.219_747,
        epsilon = 3.0e-4
    );
    assert_abs_diff_eq!(elements.d().as_degrees(), 7.586_20, epsilon = 3.0e-4);
    assert_abs_diff_eq!(elements.mu().as_degrees(), 89.591_22, epsilon = 0.001);
    assert_eq!(
        elements.limb_model(),
        BesselianLimbModel::nasa_five_millennium()
    );
    assert_abs_diff_eq!(
        elements.l1().as_equatorial_radii(),
        0.535_813,
        epsilon = 1.0e-4
    );
    assert_abs_diff_eq!(
        elements.l2().as_equatorial_radii(),
        -0.010_274,
        epsilon = 1.0e-4
    );
    assert_abs_diff_eq!(elements.tan_f1().value(), 0.004_668_3, epsilon = 5.0e-6);
    assert_abs_diff_eq!(elements.tan_f2().value(), 0.004_645_0, epsilon = 5.0e-6);
    assert_abs_diff_eq!(
        derivatives.x().as_per_tt_hour(),
        0.511_710_5,
        epsilon = 3.0e-4
    );
    assert_abs_diff_eq!(
        derivatives.y().as_per_tt_hour(),
        0.270_958_6,
        epsilon = 3.0e-4
    );
    assert_abs_diff_eq!(d_degrees_per_hour, 0.014_844, epsilon = 2.0e-5);
    assert_abs_diff_eq!(mu_degrees_per_hour, 15.004_084, epsilon = 2.0e-5);
    assert_abs_diff_eq!(
        derivatives.l1().as_per_tt_hour(),
        0.000_061_8,
        epsilon = 3.0e-5
    );
    assert_abs_diff_eq!(
        derivatives.l2().as_per_tt_hour(),
        0.000_061_5,
        epsilon = 3.0e-5
    );
}
