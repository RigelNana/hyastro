#![cfg(feature = "anise")]

use approx::assert_abs_diff_eq;
use hyastro::{
    ephem::{CelestialBody, Ephemeris, EphemerisQuery, Error, KernelManifest},
    frame::Bcrs,
    time::{DateTime, Duration, Gregorian, Hifitime, Tdb},
};

#[test]
fn kernel_manifest_rejects_an_empty_load_order() {
    assert!(matches!(
        KernelManifest::inspect(Vec::<std::path::PathBuf>::new()),
        Err(Error::EmptyKernelManifest)
    ));
}

#[test]
#[ignore = "requires HYASTRO_DE440S to name a local de440s.bsp"]
fn de440s_state_chain_coverage_and_missing_target_are_stable() {
    let path = std::env::var_os("HYASTRO_DE440S").expect("HYASTRO_DE440S must be set");
    let manifest = KernelManifest::inspect([path]).unwrap();
    let expected_bytes = manifest.kernels()[0].byte_len();
    let ephemeris = Ephemeris::load(manifest).unwrap();
    assert_eq!(ephemeris.manifest().kernels()[0].byte_len(), expected_bytes);

    let epoch = Hifitime::new()
        .resolve(DateTime::<Gregorian, Tdb>::from_components(2000, 1, 1, 12, 0, 0, 0).unwrap())
        .unwrap();
    let sun_from_ssb = ephemeris
        .state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Sun,
            CelestialBody::SolarSystemBarycenter,
            epoch,
        ))
        .unwrap();
    let earth_from_ssb = ephemeris
        .state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Earth,
            CelestialBody::SolarSystemBarycenter,
            epoch,
        ))
        .unwrap();
    let sun_from_earth = ephemeris
        .state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Sun,
            CelestialBody::Earth,
            epoch,
        ))
        .unwrap();
    let chained = sun_from_ssb
        .checked_chain(earth_from_ssb.checked_reversed().unwrap())
        .unwrap();

    for (direct, derived) in sun_from_earth
        .position()
        .components()
        .into_iter()
        .zip(chained.position().components())
    {
        assert_abs_diff_eq!(direct.as_metres(), derived.as_metres(), epsilon = 1.0e-3);
    }
    for (direct, derived) in sun_from_earth
        .velocity()
        .components()
        .into_iter()
        .zip(chained.velocity().components())
    {
        assert_abs_diff_eq!(
            direct.as_metres_per_second(),
            derived.as_metres_per_second(),
            epsilon = 1.0e-9
        );
    }

    let query = EphemerisQuery::<Bcrs, _>::new(CelestialBody::Sun, CelestialBody::Earth, epoch);
    let coverage = ephemeris.coverage(query).unwrap();
    assert!(coverage.contains(epoch));
    assert!(
        ephemeris
            .state(EphemerisQuery::<Bcrs, _>::new(
                CelestialBody::Sun,
                CelestialBody::Earth,
                coverage.start(),
            ))
            .is_ok()
    );
    assert!(
        ephemeris
            .state(EphemerisQuery::<Bcrs, _>::new(
                CelestialBody::Sun,
                CelestialBody::Earth,
                coverage.end(),
            ))
            .is_ok()
    );

    let before = coverage
        .start()
        .checked_sub(Duration::from_seconds(1).unwrap())
        .unwrap();
    assert!(matches!(
        ephemeris.state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Sun,
            CelestialBody::Earth,
            before,
        )),
        Err(Error::Coverage { .. })
    ));
    assert!(matches!(
        ephemeris.state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Jupiter,
            CelestialBody::SolarSystemBarycenter,
            epoch,
        )),
        Err(Error::UnknownTarget {
            target: CelestialBody::Jupiter
        })
    ));
    assert!(matches!(
        ephemeris.state(EphemerisQuery::<Bcrs, _>::new(
            CelestialBody::Sun,
            CelestialBody::Jupiter,
            epoch,
        )),
        Err(Error::UnknownCenter {
            center: CelestialBody::Jupiter
        })
    ));
}
