use approx::assert_abs_diff_eq;
use hyastro::{
    earth::{
        Earth, EllipsoidalHeight, Error as EarthError, GeodeticLatitude, GeodeticLongitude,
        GeodeticPosition, ReferenceEllipsoid,
    },
    frame::{Frames, Itrs},
    math::{Length, Point3},
    time::{
        CelestialPoleOffsetX, CelestialPoleOffsetY, DateTime, EarthOrientationSample,
        EarthOrientationTable, ExcessLengthOfDay, Gregorian, PolarMotionX, PolarMotionY,
        TimeContext, Ut1MinusUtc, Utc,
    },
};

#[test]
fn reference_ellipsoids_expose_canonical_derived_quantities() {
    let wgs84 = ReferenceEllipsoid::WGS84;
    assert_eq!(wgs84.identifier(), "WGS 84");
    assert_abs_diff_eq!(wgs84.semi_major_axis().as_metres(), 6_378_137.0);
    assert_abs_diff_eq!(
        wgs84.inverse_flattening(),
        298.257_223_563,
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        wgs84.semi_minor_axis().as_metres(),
        6_356_752.314_245_179,
        epsilon = 1.0e-9
    );
    assert_abs_diff_eq!(
        wgs84.first_eccentricity_squared(),
        6.694_379_990_141_316_5e-3,
        epsilon = 1.0e-18
    );

    assert!(matches!(
        ReferenceEllipsoid::new("invalid", Length::from_metres(0.0).unwrap(), 0.1),
        Err(EarthError::InvalidEllipsoid {
            field: "semi-major axis",
            ..
        })
    ));
    assert!(matches!(
        ReferenceEllipsoid::new("invalid", Length::from_metres(1.0).unwrap(), 1.0),
        Err(EarthError::InvalidEllipsoid {
            field: "flattening",
            ..
        })
    ));
}

#[test]
fn geodetic_to_itrs_matches_the_sofa_reference_case() {
    let ellipsoid = ReferenceEllipsoid::new(
        "SOFA custom reference",
        Length::from_metres(6_378_136.0).unwrap(),
        0.003_352_8,
    )
    .unwrap();
    let earth = Earth::new(ellipsoid);
    let position = GeodeticPosition::new(
        GeodeticLongitude::try_from_radians(3.1).unwrap(),
        GeodeticLatitude::try_from_radians(-0.5).unwrap(),
        EllipsoidalHeight::from_metres(2_500.0).unwrap(),
    );

    let [x, y, z] = earth
        .itrs_position(position)
        .unwrap()
        .position()
        .components()
        .map(Length::as_metres);
    assert_abs_diff_eq!(x, -5_598_999.666_511_633, epsilon = 1.0e-7);
    assert_abs_diff_eq!(y, 233_011.635_146_305_72, epsilon = 1.0e-7);
    assert_abs_diff_eq!(z, -3_040_909.051_731_413, epsilon = 1.0e-7);
}

#[test]
fn itrs_to_geodetic_matches_the_sofa_reference_case() {
    let ellipsoid = ReferenceEllipsoid::new(
        "SOFA custom reference",
        Length::from_metres(6_378_136.0).unwrap(),
        0.003_352_8,
    )
    .unwrap();
    let earth = Earth::new(ellipsoid);
    let position = Point3::<Itrs>::new(
        Length::from_metres(2.0e6).unwrap(),
        Length::from_metres(3.0e6).unwrap(),
        Length::from_metres(5.244e6).unwrap(),
    );

    let geodetic = earth.geodetic_position(position).unwrap();
    assert_abs_diff_eq!(
        geodetic.longitude().as_radians(),
        0.982_793_723_247_329,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        geodetic.latitude().as_radians(),
        0.971_601_837_757_041_2,
        epsilon = 1.0e-14
    );
    assert_abs_diff_eq!(
        geodetic.height().as_metres(),
        332.368_624_957_644,
        epsilon = 1.0e-8
    );
}

#[test]
fn geodetic_round_trips_cover_poles_subsurface_and_high_altitude() {
    let earth = Earth::wgs84();
    let cases = [
        (0.0, 0.0, 0.0),
        (45.0, 90.0, 100.0),
        (-120.0, -90.0, -500.0),
        (170.0, 34.0, -1_000.0),
        (-30.0, -20.0, 1.0e9),
    ];

    for (longitude, latitude, height) in cases {
        let geodetic = GeodeticPosition::new(
            GeodeticLongitude::try_from_degrees(longitude).unwrap(),
            GeodeticLatitude::try_from_degrees(latitude).unwrap(),
            EllipsoidalHeight::from_metres(height).unwrap(),
        );
        let original = earth.itrs_position(geodetic).unwrap();
        let recovered = earth.geodetic_position(original).unwrap();
        let round_trip = earth.itrs_position(recovered).unwrap();
        let original_components = original.position().components().map(Length::as_metres);
        let round_trip_components = round_trip.position().components().map(Length::as_metres);

        for index in 0..3 {
            let scale = original_components[index].abs().max(1.0);
            assert_abs_diff_eq!(
                round_trip_components[index],
                original_components[index],
                epsilon = scale * 5.0e-13
            );
        }
    }
}

#[test]
fn geocentric_origin_is_explicitly_undefined() {
    let zero = Length::from_metres(0.0).unwrap();
    let origin = Point3::<Itrs>::new(zero, zero, zero);
    let earth = Earth::wgs84();

    assert!(matches!(
        earth.geodetic_position(origin),
        Err(EarthError::UndefinedGeodeticPosition)
    ));
    assert!(matches!(
        earth.geocentric_latitude(origin),
        Err(EarthError::UndefinedGeodeticPosition)
    ));
}

#[test]
fn fixed_site_local_axes_have_explicit_enu_and_ned_semantics() {
    let earth = Earth::wgs84();
    let site = earth
        .fixed_site(
            "equator and Greenwich",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(0.0).unwrap(),
                GeodeticLatitude::try_from_degrees(0.0).unwrap(),
                EllipsoidalHeight::from_metres(0.0).unwrap(),
            ),
        )
        .unwrap();
    let enu = site.east_north_up();
    let ned = site.north_east_down();

    assert_eq!(enu.east().components(), [0.0, 1.0, 0.0]);
    assert_eq!(enu.north().components(), [0.0, 0.0, 1.0]);
    assert_eq!(enu.up().components(), [1.0, 0.0, 0.0]);
    assert_eq!(ned.north(), enu.north());
    assert_eq!(ned.east(), enu.east());
    assert_eq!(ned.down().components(), [-1.0, -0.0, -0.0]);
    assert_abs_diff_eq!(enu.east().dot(enu.north()), 0.0, epsilon = 1.0e-15);
    assert_abs_diff_eq!(enu.north().dot(enu.up()), 0.0, epsilon = 1.0e-15);
    assert_abs_diff_eq!(enu.up().dot(enu.east()), 0.0, epsilon = 1.0e-15);
}

#[test]
fn fixed_itrs_site_gains_gcrs_velocity_and_rotated_local_axes() {
    let base = TimeContext::builtin();
    let left = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let epoch = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 12, 0, 0, 0).unwrap())
        .unwrap();
    let right = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 2, 0, 0, 0, 0).unwrap())
        .unwrap();
    let expires = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 3, 0, 0, 0, 0).unwrap())
        .unwrap();
    let zero_dut1 = Ut1MinusUtc::from_seconds(0.0).unwrap();
    let zero_lod = ExcessLengthOfDay::from_milliseconds(0.0).unwrap();
    let zero_xp = PolarMotionX::from_arcseconds(0.0).unwrap();
    let zero_yp = PolarMotionY::from_arcseconds(0.0).unwrap();
    let zero_dx = CelestialPoleOffsetX::from_milliarcseconds(0.0).unwrap();
    let zero_dy = CelestialPoleOffsetY::from_milliarcseconds(0.0).unwrap();
    let samples = [
        EarthOrientationSample::new(
            left, zero_dut1, zero_lod, zero_xp, zero_yp, zero_dx, zero_dy,
        ),
        EarthOrientationSample::new(
            right, zero_dut1, zero_lod, zero_xp, zero_yp, zero_dx, zero_dy,
        ),
    ];
    let table = EarthOrientationTable::new(&samples, "synthetic zero EOP", expires).unwrap();
    let time = base.with_earth_orientation(table);
    let frames = Frames::new(&time);
    let site = Earth::wgs84()
        .fixed_site(
            "equatorial site",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(0.0).unwrap(),
                GeodeticLatitude::try_from_degrees(0.0).unwrap(),
                EllipsoidalHeight::from_metres(0.0).unwrap(),
            ),
        )
        .unwrap();

    let itrs = site.itrs_state(epoch);
    assert_eq!(
        itrs.velocity()
            .components()
            .map(|value| value.as_metres_per_second()),
        [0.0, 0.0, 0.0]
    );
    let gcrs = site.gcrs_state(epoch, &frames).unwrap();
    let inertial_speed = gcrs.velocity().magnitude().unwrap().as_metres_per_second();
    assert!((400.0..500.0).contains(&inertial_speed));

    let gcrs_enu = site.gcrs_east_north_up(epoch, &frames).unwrap();
    assert_abs_diff_eq!(
        gcrs_enu.east().dot(gcrs_enu.north()),
        0.0,
        epsilon = 1.0e-15
    );
    assert_abs_diff_eq!(gcrs_enu.north().dot(gcrs_enu.up()), 0.0, epsilon = 1.0e-15);
    assert_abs_diff_eq!(gcrs_enu.up().dot(gcrs_enu.east()), 0.0, epsilon = 1.0e-15);
}

#[test]
fn celestial_orientation_query_requires_tt_but_no_eop() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let result = Frames::new(&time).celestial_orientation_at(epoch).unwrap();

    assert_abs_diff_eq!(
        result.mean_obliquity().as_radians(),
        0.409_038_106_731_813_3,
        epsilon = 1.0e-15
    );
    assert!(result.true_obliquity().as_radians().is_finite());
}
