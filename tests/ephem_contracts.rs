use hyastro::{
    ephem::{CelestialBody, EphemerisQuery, Error, RelativeState},
    frame::Bcrs,
    math::{Length, Speed, Vector3},
    time::{Duration, Instant, Tai},
};

fn state(
    target: CelestialBody,
    center: CelestialBody,
    x_metres: f64,
    vx_metres_per_second: f64,
    epoch: Instant<Tai>,
) -> RelativeState<Bcrs, Tai> {
    RelativeState::try_new(
        target,
        center,
        Vector3::new(
            Length::from_metres(x_metres).unwrap(),
            Length::from_metres(0.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(vx_metres_per_second).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
        ),
        epoch,
    )
    .unwrap()
}

#[test]
fn query_preserves_target_center_frame_and_physical_epoch() {
    let epoch = Instant::<Tai>::from_tai_nanoseconds_since_1900(123_456_789);
    let query = EphemerisQuery::<Bcrs, _>::new(CelestialBody::Moon, CelestialBody::Earth, epoch);

    assert_eq!(query.target(), CelestialBody::Moon);
    assert_eq!(query.center(), CelestialBody::Earth);
    assert_eq!(query.epoch(), epoch);
}

#[test]
fn identity_state_must_be_exactly_zero() {
    let epoch = Instant::<Tai>::from_tai_nanoseconds_since_1900(0);
    let result = RelativeState::<Bcrs, _>::try_new(
        CelestialBody::Earth,
        CelestialBody::Earth,
        Vector3::new(
            Length::from_metres(1.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(0.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
        ),
        epoch,
    );

    assert!(matches!(
        result,
        Err(Error::NonZeroIdentityState {
            body: CelestialBody::Earth
        })
    ));
    assert_eq!(
        RelativeState::<Bcrs, _>::zero(CelestialBody::Earth, epoch)
            .unwrap()
            .position()
            .x()
            .as_metres(),
        0.0
    );
}

#[test]
fn reversing_and_chaining_states_preserves_body_semantics() {
    let epoch = Instant::<Tai>::from_tai_nanoseconds_since_1900(42);
    let sun_from_ssb = state(
        CelestialBody::Sun,
        CelestialBody::SolarSystemBarycenter,
        10.0,
        3.0,
        epoch,
    );
    let earth_from_ssb = state(
        CelestialBody::Earth,
        CelestialBody::SolarSystemBarycenter,
        4.0,
        1.0,
        epoch,
    );

    let sun_from_earth = sun_from_ssb
        .checked_chain(earth_from_ssb.checked_reversed().unwrap())
        .unwrap();
    assert_eq!(sun_from_earth.target(), CelestialBody::Sun);
    assert_eq!(sun_from_earth.center(), CelestialBody::Earth);
    assert_eq!(sun_from_earth.position().x().as_metres(), 6.0);
    assert_eq!(sun_from_earth.velocity().x().as_metres_per_second(), 2.0);
}

#[test]
fn state_chain_rejects_disconnected_bodies_and_different_epochs() {
    let epoch = Instant::<Tai>::from_tai_nanoseconds_since_1900(42);
    let later = epoch
        .checked_add(Duration::from_seconds(1).unwrap())
        .unwrap();
    let sun_from_ssb = state(
        CelestialBody::Sun,
        CelestialBody::SolarSystemBarycenter,
        10.0,
        3.0,
        epoch,
    );
    let moon_from_earth = state(CelestialBody::Moon, CelestialBody::Earth, 1.0, 0.1, epoch);
    assert!(matches!(
        sun_from_ssb.checked_chain(moon_from_earth),
        Err(Error::DisconnectedChain { .. })
    ));

    let ssb_from_earth = state(
        CelestialBody::SolarSystemBarycenter,
        CelestialBody::Earth,
        -4.0,
        -1.0,
        later,
    );
    assert!(matches!(
        sun_from_ssb.checked_chain(ssb_from_earth),
        Err(Error::EpochMismatch { .. })
    ));
}
