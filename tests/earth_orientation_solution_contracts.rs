use core::f64::consts::{PI, TAU};

use approx::assert_abs_diff_eq;
use hyastro::{
    frame::{Frames, Gcrs, Itrs},
    math::{AngularSpeed, Matrix3},
    time::{
        CelestialPoleOffsetX, CelestialPoleOffsetY, DateTime, Duration, EarthOrientationSample,
        EarthOrientationTable, ExcessLengthOfDay, Gregorian, Instant, JulianDate, PolarMotionX,
        PolarMotionY, TimeContext, Tt, Ut1MinusUtc, Utc,
    },
};

fn tt_epoch_at_modified_julian_date(modified_julian_date: f64) -> Instant<Tt> {
    let base = TimeContext::builtin();
    let date = JulianDate::<Tt>::from_parts(2_400_000.5, modified_julian_date)
        .unwrap()
        .to_datetime::<Gregorian>()
        .unwrap();
    base.resolve(date).unwrap()
}

fn utc_epoch_at_nominal_modified_julian_date(modified_julian_date: f64) -> Instant<Utc> {
    let base = TimeContext::builtin();
    let uniform_label = JulianDate::<Tt>::from_parts(2_400_000.5, modified_julian_date)
        .unwrap()
        .to_datetime::<Gregorian>()
        .unwrap();
    let utc_label =
        DateTime::<Gregorian, Utc>::new(uniform_label.date(), uniform_label.time()).unwrap();
    base.resolve(utc_label).unwrap()
}

fn eop_span(
    center: Instant<Utc>,
    ut1_minus_utc: f64,
    polar_motion_x: f64,
    polar_motion_y: f64,
    celestial_pole_offset_x: f64,
    celestial_pole_offset_y: f64,
) -> ([EarthOrientationSample; 2], Instant<Utc>) {
    let day = Duration::from_days(1).unwrap();
    let left = center.checked_sub(day).unwrap();
    let right = center.checked_add(day).unwrap();
    let expires = right.checked_add(day).unwrap();
    let ut1_minus_utc = Ut1MinusUtc::from_seconds(ut1_minus_utc).unwrap();
    let excess_length_of_day = ExcessLengthOfDay::from_milliseconds(0.0).unwrap();
    let polar_motion_x = PolarMotionX::from_arcseconds(polar_motion_x).unwrap();
    let polar_motion_y = PolarMotionY::from_arcseconds(polar_motion_y).unwrap();
    let celestial_pole_offset_x =
        CelestialPoleOffsetX::from_milliarcseconds(celestial_pole_offset_x).unwrap();
    let celestial_pole_offset_y =
        CelestialPoleOffsetY::from_milliarcseconds(celestial_pole_offset_y).unwrap();
    let zero_rate = AngularSpeed::from_radians_per_second(0.0).unwrap();
    let sample = |epoch| {
        EarthOrientationSample::new(
            epoch,
            ut1_minus_utc,
            excess_length_of_day,
            polar_motion_x,
            polar_motion_y,
            celestial_pole_offset_x,
            celestial_pole_offset_y,
        )
        .with_polar_motion_rates(zero_rate, zero_rate)
    };
    ([sample(left), sample(right)], expires)
}

fn assert_matrix_close(actual: Matrix3, expected: Matrix3, epsilon: f64) {
    let actual = actual.rows();
    let expected = expected.rows();
    for row in 0..3 {
        for column in 0..3 {
            assert_abs_diff_eq!(
                actual[row][column],
                expected[row][column],
                epsilon = epsilon
            );
        }
    }
}

fn signed_angle(radians: f64) -> f64 {
    let wrapped = radians.rem_euclid(TAU);
    if wrapped > PI { wrapped - TAU } else { wrapped }
}

#[test]
fn precession_nutation_matches_sofa_reference_vectors() {
    let base = TimeContext::builtin();
    let epoch = tt_epoch_at_modified_julian_date(50_123.999_9);
    let center = Instant::<Utc>::from_instant(epoch, &base).unwrap();
    let (samples, expires) = eop_span(center, 0.0, 0.0, 0.0, 0.0, 0.0);
    let table = EarthOrientationTable::new(&samples, "SOFA PFW06 and PNM06A", expires).unwrap();
    let time = base.with_earth_orientation(table);
    let solution = Frames::new(&time).earth_orientation_at(epoch).unwrap();
    let precession_nutation = solution.precession_nutation();
    let angles = precession_nutation.fukushima_williams();

    assert_abs_diff_eq!(
        angles.gamma_bar().as_radians(),
        -0.224_338_767_099_799_57e-5,
        epsilon = 1.0e-16
    );
    assert_abs_diff_eq!(
        angles.phi_bar().as_radians(),
        0.409_101_460_239_131_3,
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        angles.psi_bar().as_radians(),
        -0.950_195_417_801_303_2e-3,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        angles.mean_obliquity().as_radians(),
        0.409_101_431_658_736_75,
        epsilon = 1.0e-12
    );

    let expected_bias_precession_nutation = Matrix3::try_from_rows([
        [
            0.999_999_583_279_420_5,
            0.837_238_277_263_096_2e-3,
            0.363_968_477_114_062_3e-3,
        ],
        [
            -0.837_253_374_474_368_4e-3,
            0.999_999_648_649_286_2,
            0.413_290_594_461_101_95e-4,
        ],
        [
            -0.363_933_746_962_946_5e-3,
            -0.416_337_760_591_066_4e-4,
            0.999_999_932_909_426,
        ],
    ])
    .unwrap();
    assert_matrix_close(
        precession_nutation.bias_precession_nutation_matrix(),
        expected_bias_precession_nutation,
        1.0e-14,
    );

    let composed_bias_precession = precession_nutation
        .precession_matrix()
        .checked_mul(precession_nutation.frame_bias_matrix())
        .unwrap();
    assert_matrix_close(
        composed_bias_precession,
        precession_nutation.bias_precession_matrix(),
        2.0e-15,
    );
    let composed_bias_precession_nutation = precession_nutation
        .nutation_matrix()
        .checked_mul(precession_nutation.bias_precession_matrix())
        .unwrap();
    assert_matrix_close(
        composed_bias_precession_nutation,
        precession_nutation.bias_precession_nutation_matrix(),
        2.0e-15,
    );
}

#[test]
fn cip_sidereal_quantities_and_both_celestial_chains_are_coherent() {
    let base = TimeContext::builtin();
    let epoch = tt_epoch_at_modified_julian_date(53_736.0);
    let center = Instant::<Utc>::from_instant(epoch, &base).unwrap();
    let (samples, expires) = eop_span(center, 0.125, 0.15, -0.25, 0.269, -0.274);
    let table = EarthOrientationTable::new(&samples, "SOFA XYS06A and EE06A", expires).unwrap();
    let time = base.with_earth_orientation(table);
    let frames = Frames::new(&time);
    let solution = frames.earth_orientation_at(epoch).unwrap();
    let precession_nutation = solution.precession_nutation();

    assert_abs_diff_eq!(
        precession_nutation.nutation_longitude().as_radians(),
        -9.630_912_025_820_31e-6,
        epsilon = 1.0e-13
    );
    assert_abs_diff_eq!(
        precession_nutation.nutation_obliquity().as_radians(),
        0.406_323_849_688_725e-4,
        epsilon = 1.0e-13
    );
    assert_abs_diff_eq!(
        precession_nutation.mean_obliquity().as_radians(),
        0.409_078_976_335_651,
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        solution.modeled_cip().x().as_radians(),
        0.579_130_848_283_529_3e-3,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        solution.modeled_cip().y().as_radians(),
        0.402_058_009_945_402e-4,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(
        solution.modeled_cio_locator().as_radians(),
        -1.220_032_294_164_58e-8,
        epsilon = 1.0e-18
    );
    assert_abs_diff_eq!(
        solution.equation_of_equinoxes().as_radians(),
        -0.883_419_507_204_379e-5,
        epsilon = 1.0e-15
    );

    let offset_x = solution
        .observations()
        .celestial_pole_offset_x()
        .as_angle()
        .as_radians();
    let offset_y = solution
        .observations()
        .celestial_pole_offset_y()
        .as_angle()
        .as_radians();
    assert_abs_diff_eq!(
        solution.cip().x().as_radians() - solution.modeled_cip().x().as_radians(),
        offset_x,
        epsilon = 1.0e-19
    );
    assert_abs_diff_eq!(
        solution.cip().y().as_radians() - solution.modeled_cip().y().as_radians(),
        offset_y,
        epsilon = 1.0e-19
    );
    assert_ne!(solution.cio_locator(), solution.modeled_cio_locator());

    assert_matrix_close(
        solution.modeled_cio_gcrs_to_tirs_matrix(),
        solution.equinox_gcrs_to_tirs_matrix(),
        2.0e-15,
    );
    assert_abs_diff_eq!(
        signed_angle(
            solution.earth_rotation_angle().as_radians()
                - solution.greenwich_apparent_sidereal_time().as_radians()
        ),
        solution.equation_of_origins().as_radians(),
        epsilon = 2.0e-15
    );
    assert_abs_diff_eq!(
        signed_angle(
            solution.greenwich_apparent_sidereal_time().as_radians()
                - solution.greenwich_mean_sidereal_time().as_radians()
        ),
        solution.equation_of_equinoxes().as_radians(),
        epsilon = 2.0e-15
    );

    let composed_rotation = solution
        .gcrs_to_cirs()
        .then(solution.cirs_to_tirs())
        .unwrap()
        .then(solution.tirs_to_itrs())
        .unwrap();
    assert_matrix_close(
        composed_rotation.rotation().matrix(),
        solution.gcrs_to_itrs().rotation().matrix(),
        2.0e-15,
    );
    let frame_transform = frames.at::<Gcrs, Itrs, Tt>(epoch).unwrap();
    assert_matrix_close(
        frame_transform.rotation().matrix(),
        solution.gcrs_to_itrs().rotation().matrix(),
        2.0e-15,
    );
}

#[test]
fn earth_rotation_angle_matches_the_sofa_ut1_reference_vector() {
    let base = TimeContext::builtin();
    let epoch = utc_epoch_at_nominal_modified_julian_date(54_388.0);
    let (samples, expires) = eop_span(epoch, 0.0, 0.0, 0.0, 0.0, 0.0);
    let table = EarthOrientationTable::new(&samples, "SOFA ERA00", expires).unwrap();
    let time = base.with_earth_orientation(table);
    let solution = Frames::new(&time).earth_orientation_at(epoch).unwrap();

    assert_abs_diff_eq!(
        solution.universal_time().as_f64_lossy(),
        2_454_388.5,
        epsilon = 0.0
    );
    assert_abs_diff_eq!(
        solution.earth_rotation_angle().as_radians(),
        0.402_283_724_002_815_8,
        epsilon = 1.0e-12
    );
}
