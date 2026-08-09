use approx::assert_relative_eq;
use hyastro::math::{ApparentMagnitude, FluxRatio, JohnsonV, MagnitudeDifference, Vega};

#[test]
fn apparent_magnitudes_preserve_negative_values_and_directional_flux_ratios() {
    let reference = ApparentMagnitude::<JohnsonV, Vega>::from_magnitudes(0.0).unwrap();
    let target = ApparentMagnitude::<JohnsonV, Vega>::from_magnitudes(5.0).unwrap();
    let bright = ApparentMagnitude::<JohnsonV, Vega>::from_magnitudes(-5.0).unwrap();

    assert_eq!(bright.as_magnitudes(), -5.0);
    assert_eq!(
        target.difference_from(reference).unwrap().as_magnitudes(),
        5.0
    );
    assert_relative_eq!(
        target.flux_ratio_to(reference).unwrap().as_ratio(),
        0.01,
        max_relative = 2.0e-15,
    );
    assert_relative_eq!(
        reference.flux_ratio_to(target).unwrap().as_ratio(),
        100.0,
        max_relative = 2.0e-15,
    );
}

#[test]
fn magnitude_differences_and_flux_ratios_round_trip() {
    for difference in [-12.5, -1.0, 0.0, 1.0, 12.5] {
        let difference = MagnitudeDifference::from_magnitudes(difference).unwrap();
        let restored = difference.flux_ratio().unwrap().magnitude_difference();
        assert_relative_eq!(
            restored.as_magnitudes(),
            difference.as_magnitudes(),
            epsilon = 2.0e-15,
        );
    }
}

#[test]
fn photometric_scalars_reject_non_finite_and_non_positive_inputs() {
    assert!(ApparentMagnitude::<JohnsonV, Vega>::from_magnitudes(f64::NAN).is_err());
    assert!(MagnitudeDifference::from_magnitudes(f64::INFINITY).is_err());
    assert!(FluxRatio::from_ratio(0.0).is_err());
    assert!(FluxRatio::from_ratio(-1.0).is_err());
    assert!(FluxRatio::from_ratio(f64::INFINITY).is_err());

    assert!(
        MagnitudeDifference::from_magnitudes(1_000.0)
            .unwrap()
            .flux_ratio()
            .is_err()
    );
    assert!(
        MagnitudeDifference::from_magnitudes(-1_000.0)
            .unwrap()
            .flux_ratio()
            .is_err()
    );
}

#[cfg(feature = "anise")]
mod lunar {
    use hyastro::{
        astro::{
            Astrometry, HorizonsCompatibleLunarV, LunarVApplicability, ReceptionLightTimeOptions,
        },
        ephem::{Ephemeris, KernelManifest},
        time::{DateTime, Gregorian, TimeContext, Utc},
    };

    fn evaluate_at(
        ephemeris: &Ephemeris,
        time: &TimeContext,
        components: (i32, u8, u8, u8, u8),
    ) -> hyastro::astro::GeocentricLunarVMagnitude<Utc> {
        let (year, month, day, hour, minute) = components;
        let epoch = time
            .resolve(
                DateTime::<Gregorian, Utc>::from_components(year, month, day, hour, minute, 0, 0)
                    .unwrap(),
            )
            .unwrap();
        let illumination = Astrometry::new(time, ephemeris)
            .lunar_illumination_at(epoch, ReceptionLightTimeOptions::standard())
            .unwrap();
        HorizonsCompatibleLunarV::evaluate(illumination).unwrap()
    }

    #[test]
    #[ignore = "requires HYASTRO_DE440S to name a local DE440-family BSP"]
    fn de440_lunar_v_matches_horizons_and_reports_model_limitations() {
        let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
        let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
        let time = TimeContext::builtin();

        let eclipse = evaluate_at(&ephemeris, &time, (2024, 3, 25, 7, 0));
        let reconstructed = 0.23
            + eclipse.distance_correction().as_magnitudes()
            + eclipse.phase_correction().as_magnitudes();

        // JPL Horizons DE441, geocentric observer, 2024-03-25 07:00 UTC,
        // quantity 9: APmag=-12.580. Horizons does not apply eclipse dimming.
        assert!((eclipse.magnitude().as_magnitudes() - (-12.580)).abs() <= 0.001);
        assert!((eclipse.magnitude().as_magnitudes() - reconstructed).abs() <= f64::EPSILON);
        assert_eq!(
            eclipse.applicability(),
            LunarVApplicability::EarthShadowIntersection
        );
        assert_eq!(
            eclipse.model_identifier(),
            HorizonsCompatibleLunarV::IDENTIFIER
        );
        assert!(eclipse.flux_ratio_to_zero_magnitude().unwrap().as_ratio() > 1.0);

        let uneclipsed_full_moon = evaluate_at(&ephemeris, &time, (2024, 4, 23, 23, 49));
        assert_eq!(
            uneclipsed_full_moon.applicability(),
            LunarVApplicability::NearFullMoonKnownBias
        );

        let quarter = evaluate_at(&ephemeris, &time, (2024, 4, 15, 19, 13));
        assert_eq!(quarter.applicability(), LunarVApplicability::Nominal);
    }
}
