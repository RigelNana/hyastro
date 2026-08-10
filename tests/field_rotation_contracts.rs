#![cfg(feature = "anise")]

use hyastro::{
    astro::{
        Astrometry, FieldRotation, FieldRotationDirection, FieldRotationOptions, ParallacticAngle,
        ParallacticAngleAt, ReceptionLightTimeOptions,
    },
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{CelestialBody, SofaAnalyticEphemeris},
    math::{Declination, HourAngle, Latitude},
    time::{
        CelestialPoleOffsetX, CelestialPoleOffsetY, DateTime, Duration, EarthOrientationSample,
        EarthOrientationTable, ExcessLengthOfDay, Gregorian, PolarMotionX, PolarMotionY,
        TimeContext, Ut1MinusUtc, Utc,
    },
};

#[test]
fn parallactic_angle_matches_the_sofa_hd2pa_reference() {
    let angle = ParallacticAngle::from_equatorial(
        HourAngle::try_from_radians(1.1).unwrap(),
        Declination::try_from_radians(1.2).unwrap(),
        Latitude::try_from_radians(0.3).unwrap(),
    )
    .unwrap();

    // SOFA iauHd2pa validation value for H=1.1, Dec=1.2, latitude=0.3 radians.
    assert!((angle.as_radians() - 1.906_227_428_001_995_6).abs() <= 1.0e-13);
    assert_eq!(
        ParallacticAngle::wrap_degrees(540.0).unwrap().as_degrees(),
        180.0
    );
}

#[test]
fn symmetric_samples_preserve_direction_across_the_signed_angle_branch() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let offset = Duration::from_seconds(1).unwrap();
    let rotation = FieldRotation::from_symmetric_samples(
        ParallacticAngleAt::new(
            epoch.checked_sub(offset).unwrap(),
            ParallacticAngle::wrap_degrees(179.0).unwrap(),
        ),
        ParallacticAngleAt::new(epoch, ParallacticAngle::wrap_degrees(180.0).unwrap()),
        ParallacticAngleAt::new(
            epoch.checked_add(offset).unwrap(),
            ParallacticAngle::wrap_degrees(-179.0).unwrap(),
        ),
    )
    .unwrap();

    assert_eq!(rotation.epoch(), epoch);
    assert_eq!(rotation.sample_offset().unwrap(), offset);
    assert_eq!(
        rotation.direction(),
        FieldRotationDirection::IncreasingPositionAngle
    );
    assert!((rotation.position_angle_change().as_degrees() - 2.0).abs() <= 1.0e-12);
    assert!((rotation.rate().as_degrees_per_second() - 1.0).abs() <= 1.0e-12);
    assert!(
        (rotation.rate().magnitude().as_radians_per_second() - 1_f64.to_radians()).abs() <= 1.0e-15
    );
    assert!(FieldRotationOptions::new(Duration::ZERO).is_err());
}

#[test]
fn field_rotation_rate_matches_the_analytic_parallactic_derivative() {
    let time = TimeContext::builtin();
    let epoch = time
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let offset = Duration::from_seconds(1).unwrap();
    let latitude = 35_f64.to_radians();
    let declination = 20_f64.to_radians();
    let hour_angle = 40_f64.to_radians();
    let earth_rate = 7.292_115_0e-5;
    let angle_at = |seconds: f64| {
        ParallacticAngle::from_equatorial(
            HourAngle::wrap_radians(hour_angle + seconds * earth_rate).unwrap(),
            Declination::try_from_radians(declination).unwrap(),
            Latitude::try_from_radians(latitude).unwrap(),
        )
        .unwrap()
    };
    let rotation = FieldRotation::from_symmetric_samples(
        ParallacticAngleAt::new(epoch.checked_sub(offset).unwrap(), angle_at(-1.0)),
        ParallacticAngleAt::new(epoch, angle_at(0.0)),
        ParallacticAngleAt::new(epoch.checked_add(offset).unwrap(), angle_at(1.0)),
    )
    .unwrap();

    let numerator = latitude.cos() * hour_angle.sin();
    let denominator =
        latitude.sin() * declination.cos() - latitude.cos() * declination.sin() * hour_angle.cos();
    let numerator_derivative = latitude.cos() * hour_angle.cos();
    let denominator_derivative = latitude.cos() * declination.sin() * hour_angle.sin();
    let expected = (denominator * numerator_derivative - numerator * denominator_derivative)
        / (denominator * denominator + numerator * numerator)
        * earth_rate;

    assert_eq!(
        rotation.direction(),
        FieldRotationDirection::IncreasingPositionAngle
    );
    assert!((rotation.rate().as_radians_per_second() - expected).abs() <= 1.0e-12);
}

#[test]
fn fixed_site_workflow_evaluates_complete_moving_target_samples() {
    let base = TimeContext::builtin();
    let left = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 24, 0, 0, 0, 0).unwrap())
        .unwrap();
    let epoch = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 25, 7, 0, 0, 0).unwrap())
        .unwrap();
    let right = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 26, 0, 0, 0, 0).unwrap())
        .unwrap();
    let expires = base
        .resolve(DateTime::<Gregorian, Utc>::from_components(2024, 3, 27, 0, 0, 0, 0).unwrap())
        .unwrap();
    let sample = |sample_epoch| {
        EarthOrientationSample::new(
            sample_epoch,
            Ut1MinusUtc::from_seconds(0.0).unwrap(),
            ExcessLengthOfDay::from_milliseconds(0.0).unwrap(),
            PolarMotionX::from_arcseconds(0.0).unwrap(),
            PolarMotionY::from_arcseconds(0.0).unwrap(),
            CelestialPoleOffsetX::from_milliarcseconds(0.0).unwrap(),
            CelestialPoleOffsetY::from_milliarcseconds(0.0).unwrap(),
        )
    };
    let samples = [sample(left), sample(right)];
    let eop = EarthOrientationTable::new(&samples, "synthetic field rotation", expires).unwrap();
    let time = base.with_earth_orientation(eop);
    let site = Earth::wgs84()
        .fixed_site(
            "Beijing",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(116.391).unwrap(),
                GeodeticLatitude::try_from_degrees(39.9075).unwrap(),
                EllipsoidalHeight::from_metres(43.5).unwrap(),
            ),
        )
        .unwrap();
    let ephemeris = SofaAnalyticEphemeris::new();
    let astrometry = Astrometry::new(&time, &ephemeris);
    let light_time = ReceptionLightTimeOptions::standard();
    let options = FieldRotationOptions::standard();
    let rotation = astrometry
        .field_rotation_at(&site, CelestialBody::Moon, epoch, light_time, options)
        .unwrap();
    let direct = astrometry
        .fixed_observer_at(&site, epoch)
        .unwrap()
        .vacuum_observed_place(CelestialBody::Moon, light_time)
        .unwrap()
        .parallactic_angle()
        .unwrap();

    assert_eq!(rotation.current(), direct);
    assert_eq!(rotation.sample_offset().unwrap(), options.sample_offset());
    assert!(rotation.rate().as_radians_per_second().is_finite());
    assert!(rotation.rate().magnitude().as_radians_per_second() > 0.0);
    assert_ne!(rotation.direction(), FieldRotationDirection::Stationary);
}
