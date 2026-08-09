#![cfg(feature = "anise")]

use hyastro::{
    astro::{
        AirTemperature, Astrometry, AtmosphericConditions, AtmosphericPressure,
        ObservingWavelength, ReceptionLightTimeOptions, RelativeHumidity,
    },
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{CelestialBody, Ephemeris, KernelManifest},
    event::{
        Events, HorizonCriterion, HorizonEventKind, HorizonSearchOptions, HorizonVisibility,
        TwilightEventKind, TwilightLevel, TwilightState,
    },
    math::Angle,
    time::{
        DateTime, Duration, EarthAttitudeTable, EarthOrientationAcceptance, EarthOrientationTable,
        Gregorian, Hifitime, IersC04, ModifiedJulianDate, Tdb, TimeContext, TimeInterval, Utc,
    },
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");

#[test]
fn horizon_search_options_reject_invalid_controls() {
    let light_time = ReceptionLightTimeOptions::standard();
    assert!(
        HorizonSearchOptions::new(
            Duration::ZERO,
            Duration::from_milliseconds(1).unwrap(),
            Angle::from_radians(1.0e-9).unwrap(),
            64,
            1_000,
            light_time,
        )
        .is_err()
    );
    assert!(
        HorizonSearchOptions::new(
            Duration::from_seconds(3_600).unwrap(),
            Duration::from_milliseconds(1).unwrap(),
            Angle::from_radians(0.0).unwrap(),
            64,
            1_000,
            light_time,
        )
        .is_err()
    );
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local de440s.bsp"]
fn de440s_horizon_events_distinguish_geometry_refraction_and_polar_reachability() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let ephemeris = Ephemeris::load(KernelManifest::inspect([path]).unwrap()).unwrap();
    let base = TimeContext::builtin();
    let eop_data = IersC04::parse(C04).unwrap();
    let samples = eop_data
        .try_samples_in(
            &base,
            ModifiedJulianDate::<Utc>::from_parts(51_543.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(51_547.0, 0.0).unwrap(),
            EarthOrientationAcceptance::FinalOnly,
        )
        .unwrap();
    let attitude_samples = eop_data
        .try_earth_attitude_samples_in(
            &base,
            ModifiedJulianDate::<Utc>::from_parts(51_543.0, 0.0).unwrap(),
            ModifiedJulianDate::<Utc>::from_parts(51_547.0, 0.0).unwrap(),
            EarthOrientationAcceptance::FinalOnly,
        )
        .unwrap();
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let eop = EarthOrientationTable::new(&samples, "C04 horizon test", expires).unwrap();
    let time = base.with_earth_orientation(eop);
    let attitude =
        EarthAttitudeTable::new(&attitude_samples, "C04 horizon attitude test", expires).unwrap();
    let attitude_time = base.with_earth_attitude(attitude);
    let start = Hifitime::new()
        .resolve(DateTime::<Gregorian, Tdb>::from_components(2000, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let end = Hifitime::new()
        .resolve(DateTime::<Gregorian, Tdb>::from_components(2000, 1, 2, 0, 0, 0, 0).unwrap())
        .unwrap();
    let interval = TimeInterval::new(start, end).unwrap();
    let earth = Earth::wgs84();
    let site = earth
        .fixed_site(
            "Greenwich",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(0.0).unwrap(),
                GeodeticLatitude::try_from_degrees(51.4779).unwrap(),
                EllipsoidalHeight::from_metres(46.0).unwrap(),
            ),
        )
        .unwrap();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let options = HorizonSearchOptions::standard();
    let geometric = events
        .horizon_events_in(
            &site,
            CelestialBody::Sun,
            interval,
            HorizonCriterion::geometric_center(),
            options,
        )
        .unwrap();
    assert_eq!(geometric.visibility(), HorizonVisibility::RisesAndSets);
    assert!(
        geometric
            .events()
            .iter()
            .any(|event| event.kind() == HorizonEventKind::Rise)
    );
    assert!(
        geometric
            .events()
            .iter()
            .any(|event| event.kind() == HorizonEventKind::Set)
    );
    assert!(
        geometric
            .events()
            .iter()
            .any(|event| event.kind() == HorizonEventKind::UpperTransit)
    );
    assert!(
        geometric
            .events()
            .iter()
            .any(|event| event.kind() == HorizonEventKind::LowerTransit)
    );
    for event in geometric.events() {
        assert!(
            event.evidence().residual().as_radians().abs()
                <= options.angular_tolerance().as_radians()
        );
    }
    let nominal_events = Events::new(Astrometry::new(&attitude_time, &ephemeris));
    let nominal = nominal_events
        .horizon_events_with_nominal_rotation_in(
            &site,
            CelestialBody::Sun,
            interval,
            HorizonCriterion::geometric_center(),
            options,
        )
        .unwrap();
    assert_eq!(nominal.visibility(), geometric.visibility());
    for kind in [
        HorizonEventKind::Rise,
        HorizonEventKind::UpperTransit,
        HorizonEventKind::Set,
        HorizonEventKind::LowerTransit,
    ] {
        let measured = geometric
            .events()
            .iter()
            .find(|event| event.kind() == kind)
            .unwrap()
            .instant();
        let nominal = nominal
            .events()
            .iter()
            .find(|event| event.kind() == kind)
            .unwrap()
            .instant();
        let delta = if nominal >= measured {
            nominal.duration_since(measured).unwrap()
        } else {
            measured.duration_since(nominal).unwrap()
        };
        assert!(delta.as_seconds_f64() < 1.0);
    }

    let atmosphere = AtmosphericConditions::new(
        AtmosphericPressure::from_hectopascals(1_013.25).unwrap(),
        AirTemperature::from_degrees_celsius(10.0).unwrap(),
        RelativeHumidity::from_fraction(0.8).unwrap(),
        ObservingWavelength::from_micrometres(0.55).unwrap(),
    );
    let refracted = events
        .horizon_events_in(
            &site,
            CelestialBody::Sun,
            interval,
            HorizonCriterion::refracted_center(atmosphere),
            options,
        )
        .unwrap();
    let geometric_rise = geometric
        .events()
        .iter()
        .find(|event| event.kind() == HorizonEventKind::Rise)
        .unwrap();
    let refracted_rise = refracted
        .events()
        .iter()
        .find(|event| event.kind() == HorizonEventKind::Rise)
        .unwrap();
    let geometric_set = geometric
        .events()
        .iter()
        .find(|event| event.kind() == HorizonEventKind::Set)
        .unwrap();
    let refracted_set = refracted
        .events()
        .iter()
        .find(|event| event.kind() == HorizonEventKind::Set)
        .unwrap();
    assert!(refracted_rise.instant() < geometric_rise.instant());
    assert!(refracted_set.instant() > geometric_set.instant());
    assert!(
        refracted
            .events()
            .iter()
            .all(|event| event.observed_place().is_some())
    );

    let civil = events
        .twilight_events_in(&site, interval, TwilightLevel::Civil, options)
        .unwrap();
    let nautical = events
        .twilight_events_in(&site, interval, TwilightLevel::Nautical, options)
        .unwrap();
    let astronomical = events
        .twilight_events_in(&site, interval, TwilightLevel::Astronomical, options)
        .unwrap();
    assert_eq!(civil.state(), TwilightState::DawnAndDusk);
    let civil_dawn = civil
        .events()
        .iter()
        .find(|event| event.kind() == TwilightEventKind::Dawn)
        .unwrap()
        .instant();
    let nautical_dawn = nautical
        .events()
        .iter()
        .find(|event| event.kind() == TwilightEventKind::Dawn)
        .unwrap()
        .instant();
    let astronomical_dawn = astronomical
        .events()
        .iter()
        .find(|event| event.kind() == TwilightEventKind::Dawn)
        .unwrap()
        .instant();
    assert!(astronomical_dawn < nautical_dawn);
    assert!(nautical_dawn < civil_dawn);
    let civil_dusk = civil
        .events()
        .iter()
        .find(|event| event.kind() == TwilightEventKind::Dusk)
        .unwrap()
        .instant();
    let nautical_dusk = nautical
        .events()
        .iter()
        .find(|event| event.kind() == TwilightEventKind::Dusk)
        .unwrap()
        .instant();
    let astronomical_dusk = astronomical
        .events()
        .iter()
        .find(|event| event.kind() == TwilightEventKind::Dusk)
        .unwrap()
        .instant();
    assert!(civil_dusk < nautical_dusk);
    assert!(nautical_dusk < astronomical_dusk);

    let arctic = earth
        .fixed_site(
            "80 north",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(0.0).unwrap(),
                GeodeticLatitude::try_from_degrees(80.0).unwrap(),
                EllipsoidalHeight::from_metres(0.0).unwrap(),
            ),
        )
        .unwrap();
    let antarctic = earth
        .fixed_site(
            "80 south",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(0.0).unwrap(),
                GeodeticLatitude::try_from_degrees(-80.0).unwrap(),
                EllipsoidalHeight::from_metres(0.0).unwrap(),
            ),
        )
        .unwrap();
    assert_eq!(
        events
            .horizon_events_in(
                &arctic,
                CelestialBody::Sun,
                interval,
                HorizonCriterion::geometric_center(),
                options,
            )
            .unwrap()
            .visibility(),
        HorizonVisibility::NeverRisesOverInterval
    );
    assert_eq!(
        events
            .horizon_events_in(
                &antarctic,
                CelestialBody::Sun,
                interval,
                HorizonCriterion::geometric_center(),
                options,
            )
            .unwrap()
            .visibility(),
        HorizonVisibility::CircumpolarOverInterval
    );
}
