#![cfg(feature = "anise")]

use approx::assert_abs_diff_eq;
use hyastro::{
    astro::{
        AirTemperature, AstrometricCatalogPlace, AstrometricSpatialCatalogPlace, Astrometry,
        AtmosphericConditions, AtmosphericPressure, Error, ObservingWavelength, RelativeHumidity,
    },
    catalog::{
        CatalogProperMotion, CatalogRadialVelocity, Error as CatalogError, InfiniteCatalogPlace,
        Parallax, SpatialCatalogPlace,
    },
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{Ephemeris, KernelManifest},
    frame::{EquatorialDirection, Icrs},
    math::{Declination, RightAscension},
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, Hifitime,
        IersC04, JulianDate, ModifiedJulianDate, Tcb, Tdb, TimeContext, Utc,
    },
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");
const RADIANS_PER_MILLIARCSECOND: f64 = core::f64::consts::PI / 648_000_000.0;

#[test]
fn catalog_proper_motion_preserves_mu_alpha_star_convention() {
    let hifitime = Hifitime::new();
    let epoch = hifitime
        .resolve(DateTime::<Gregorian, Tdb>::from_components(2020, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let epoch_tcb = JulianDate::<Tcb>::from_instant(epoch, &hifitime).unwrap();
    let reference_epoch = epoch_tcb
        .checked_add_duration(Duration::from_julian_years(-1).unwrap())
        .unwrap();
    let catalog_direction = EquatorialDirection::<Icrs>::new(
        RightAscension::try_from_degrees(30.0).unwrap(),
        Declination::try_from_degrees(60.0).unwrap(),
    );
    let proper_motion =
        CatalogProperMotion::from_milliarcseconds_per_julian_year(100.0, 0.0).unwrap();
    let catalog = InfiniteCatalogPlace::new(reference_epoch, catalog_direction, proper_motion);

    let propagated = AstrometricCatalogPlace::from_catalog(catalog, epoch).unwrap();
    let propagated_direction = propagated.direction().coordinates();
    let separation = catalog_direction
        .separation_to(propagated_direction)
        .unwrap()
        .as_radians();
    let right_ascension_shift = propagated_direction.right_ascension().as_radians()
        - catalog_direction.right_ascension().as_radians();

    assert_abs_diff_eq!(propagated.elapsed_julian_years(), 1.0, epsilon = 1.0e-13);
    assert_abs_diff_eq!(
        separation,
        100.0 * RADIANS_PER_MILLIARCSECOND,
        epsilon = 2.0e-13
    );
    assert_abs_diff_eq!(
        right_ascension_shift * catalog_direction.declination().as_radians().cos(),
        100.0 * RADIANS_PER_MILLIARCSECOND,
        epsilon = 2.0e-13
    );
    assert_abs_diff_eq!(
        proper_motion.right_ascension_cos_declination_radians_per_julian_year(),
        100.0 * RADIANS_PER_MILLIARCSECOND,
        epsilon = 1.0e-22
    );
}

#[test]
fn nonzero_mu_alpha_star_is_rejected_at_the_icrs_pole() {
    let hifitime = Hifitime::new();
    let epoch = hifitime
        .resolve(DateTime::<Gregorian, Tdb>::from_components(2020, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let reference_epoch = JulianDate::<Tcb>::from_instant(epoch, &hifitime).unwrap();
    let catalog = InfiniteCatalogPlace::new(
        reference_epoch,
        EquatorialDirection::new(
            RightAscension::try_from_degrees(0.0).unwrap(),
            Declination::try_from_degrees(90.0).unwrap(),
        ),
        CatalogProperMotion::from_milliarcseconds_per_julian_year(1.0, 0.0).unwrap(),
    );

    assert!(matches!(
        AstrometricCatalogPlace::from_catalog(catalog, epoch),
        Err(Error::Catalog(
            CatalogError::UndefinedRightAscensionMotion { .. }
        ))
    ));
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local de440s.bsp"]
fn de440s_catalog_places_run_the_complete_fixed_site_chains() {
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
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1).unwrap())
        .unwrap();
    let eop = EarthOrientationTable::new(&samples, "C04 catalog test", expires).unwrap();
    let time = base.with_earth_orientation(eop);
    let hifitime = Hifitime::new();
    let epoch = hifitime
        .resolve(DateTime::<Gregorian, Tdb>::from_components(2000, 1, 1, 0, 0, 0, 0).unwrap())
        .unwrap();
    let site = Earth::wgs84()
        .fixed_site(
            "Greenwich",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(0.0).unwrap(),
                GeodeticLatitude::try_from_degrees(51.4779).unwrap(),
                EllipsoidalHeight::from_metres(46.0).unwrap(),
            ),
        )
        .unwrap();
    let astrometry = Astrometry::new(&time, &ephemeris);
    let observer = astrometry.fixed_observer_at(&site, epoch).unwrap();
    let catalog = InfiniteCatalogPlace::new(
        JulianDate::<Tcb>::from_j2000_offset_days(0.0).unwrap(),
        EquatorialDirection::new(
            RightAscension::try_from_degrees(37.954_560_67).unwrap(),
            Declination::try_from_degrees(89.264_108_97).unwrap(),
        ),
        CatalogProperMotion::from_milliarcseconds_per_julian_year(44.22, -11.74).unwrap(),
    );
    let astrometric = AstrometricCatalogPlace::from_catalog(catalog, epoch).unwrap();
    let vacuum = observer.vacuum_observed_catalog_place(astrometric).unwrap();
    let spatial_catalog = SpatialCatalogPlace::new(
        catalog.reference_epoch(),
        catalog.direction(),
        catalog.proper_motion(),
        Parallax::from_arcseconds(1.0).unwrap(),
        CatalogRadialVelocity::from_kilometres_per_second(0.0).unwrap(),
    );
    let spatial_astrometric =
        AstrometricSpatialCatalogPlace::from_spatial_catalog(spatial_catalog, epoch).unwrap();
    let spatial_vacuum = observer
        .vacuum_observed_spatial_catalog_place(spatial_astrometric)
        .unwrap();
    let antipodal_site = Earth::wgs84()
        .fixed_site(
            "Greenwich antipode",
            GeodeticPosition::new(
                GeodeticLongitude::try_from_degrees(180.0).unwrap(),
                GeodeticLatitude::try_from_degrees(-51.4779).unwrap(),
                EllipsoidalHeight::from_metres(46.0).unwrap(),
            ),
        )
        .unwrap();
    let antipodal_observer = astrometry
        .fixed_observer_at(&antipodal_site, epoch)
        .unwrap();
    let antipodal_vacuum = antipodal_observer
        .vacuum_observed_spatial_catalog_place(spatial_astrometric)
        .unwrap();
    let conditions = AtmosphericConditions::new(
        AtmosphericPressure::from_hectopascals(1_013.25).unwrap(),
        AirTemperature::from_degrees_celsius(10.0).unwrap(),
        RelativeHumidity::from_fraction(0.8).unwrap(),
        ObservingWavelength::from_micrometres(0.55).unwrap(),
    );
    let observed = vacuum.apply_refraction(conditions).unwrap();

    assert_eq!(vacuum.epoch(), epoch);
    assert_eq!(observed.epoch(), epoch);
    assert!((50.0..53.0).contains(&vacuum.horizontal().altitude().as_degrees()));
    assert!(
        observed.horizontal().altitude().as_radians() > vacuum.horizontal().altitude().as_radians()
    );
    assert!(observed.refraction().amount().as_radians() > 0.0);
    assert!(vacuum.corrections().solar_light_deflection().as_radians() > 0.0);
    assert!(vacuum.corrections().observer_aberration().as_radians() > 1.0e-7);
    assert_eq!(vacuum.corrections().parallax().as_radians(), 0.0);
    assert!(
        spatial_vacuum.corrections().parallax().as_radians() > 100.0 * RADIANS_PER_MILLIARCSECOND
    );
    assert!(
        (spatial_vacuum.corrections().parallax().as_radians()
            - antipodal_vacuum.corrections().parallax().as_radians())
        .abs()
            > 1.0e-13
    );

    let wrong_epoch = epoch
        .checked_add(Duration::from_seconds(1).unwrap())
        .unwrap();
    let wrong_stage = AstrometricCatalogPlace::from_catalog(catalog, wrong_epoch).unwrap();
    assert!(matches!(
        observer.vacuum_observed_catalog_place(wrong_stage),
        Err(Error::CatalogPlaceEpochMismatch { .. })
    ));
}
