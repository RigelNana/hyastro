#![cfg(feature = "anise")]

use hyastro::{
    astro::{Astrometry, LunarRotationModel, ReceptionLightTimeOptions},
    ephem::{Ephemeris, KernelManifest, SofaAnalyticEphemeris},
    time::{DateTime, Gregorian, Hifitime, Tdb, TimeContext, Utc},
};

fn circular_difference_degrees(left: f64, right: f64) -> f64 {
    ((left - right + 180.0).rem_euclid(360.0) - 180.0).abs()
}

#[test]
fn iau_2009_rotation_elements_match_the_published_coefficient_evaluation() {
    let j2000 = Hifitime::new()
        .resolve(DateTime::<Gregorian, Tdb>::from_components(2000, 1, 1, 12, 0, 0, 0).unwrap())
        .unwrap();
    let rotation = LunarRotationModel::Iau2009Wgccre
        .rotation_at(j2000)
        .unwrap();

    assert_eq!(rotation.model(), LunarRotationModel::Iau2009Wgccre);
    assert!((rotation.mean().pole_right_ascension().as_degrees() - 269.9949).abs() <= 1.0e-12);
    assert!((rotation.mean().pole_declination().as_degrees() - 66.5392).abs() <= 1.0e-12);
    assert!((rotation.mean().prime_meridian_angle().as_degrees() - 38.3213).abs() <= 1.0e-12);
    assert!(
        (rotation.instantaneous().pole_right_ascension().as_degrees() - 266.857_733_444_951_35)
            .abs()
            <= 1.0e-11
    );
    assert!(
        (rotation.instantaneous().pole_declination().as_degrees() - 65.641_102_747_845_32).abs()
            <= 1.0e-11
    );
    assert!(
        (rotation.instantaneous().prime_meridian_angle().as_degrees() - 41.195_263_980_745_22)
            .abs()
            <= 1.0e-11
    );
    assert!(!LunarRotationModel::Iau2009Wgccre.identifier().is_empty());
    assert!(!LunarRotationModel::Iau2009Wgccre.source().is_empty());
    assert!(!LunarRotationModel::Iau2009Wgccre.applicability().is_empty());
}

#[test]
fn analytic_ephemeris_libration_tracks_horizons_at_full_moon() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 25, 7, 0, 0, 0).unwrap())
        .unwrap();
    let ephemeris = SofaAnalyticEphemeris::new();
    let orientation = Astrometry::new(&time, &ephemeris)
        .lunar_disk_orientation_at(
            epoch,
            ReceptionLightTimeOptions::standard(),
            LunarRotationModel::Iau2009Wgccre,
        )
        .unwrap();
    let optical = orientation.optical_libration();
    let physical = orientation.physical_libration();
    let total = orientation.total_libration();

    assert_eq!(orientation.illumination().reception_epoch(), epoch);
    assert_eq!(
        orientation.rotation().model(),
        LunarRotationModel::Iau2009Wgccre
    );
    assert!(
        circular_difference_degrees(
            optical.longitude().as_degrees() + physical.longitude().as_degrees(),
            total.longitude().as_degrees(),
        ) <= 1.0e-12
    );
    assert!(
        (optical.latitude().as_degrees() + physical.latitude().as_degrees()
            - total.latitude().as_degrees())
        .abs()
            <= 1.0e-12
    );

    // JPL Horizons DE441 geocentric observer table, 2024-03-25 07:00 UTC,
    // quantities 14, 16, and 17: ObsSub-LON=358.421989°, ObsSub-LAT=-1.283199°,
    // SN.ang=23.21°, NP.ang=21.8150°. The SOFA Moon98 ephemeris and WGCCRE
    // analytic orientation are lower-accuracy substitutes, so this test uses a
    // deliberately model-appropriate angular envelope.
    assert!(circular_difference_degrees(total.longitude().as_degrees(), 358.421_989) <= 0.15);
    assert!((total.latitude().as_degrees() - (-1.283_199)).abs() <= 0.15);
    assert!(
        circular_difference_degrees(orientation.axis_position_angle().as_degrees(), 21.8150)
            <= 0.15
    );
    assert!(
        circular_difference_degrees(orientation.bright_limb_position_angle().as_degrees(), 23.21,)
            <= 0.20
    );
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local DE440-family BSP"]
fn de440_libration_matches_horizons_disk_coordinates() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 25, 7, 0, 0, 0).unwrap())
        .unwrap();
    let orientation = Astrometry::new(&time, &ephemeris)
        .lunar_disk_orientation_at(
            epoch,
            ReceptionLightTimeOptions::standard(),
            LunarRotationModel::Iau2009Wgccre,
        )
        .unwrap();
    let total = orientation.total_libration();

    assert!(circular_difference_degrees(total.longitude().as_degrees(), 358.421_989) <= 0.03);
    assert!((total.latitude().as_degrees() - (-1.283_199)).abs() <= 0.01);
    assert!(
        circular_difference_degrees(orientation.axis_position_angle().as_degrees(), 21.8150)
            <= 0.03
    );
}
