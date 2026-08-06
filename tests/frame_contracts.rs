use approx::assert_abs_diff_eq;
use hyastro::{
    frame::{
        Axes, Bcrs, Cirs, CoordinateFrame, Error, Gcrs, Icrs, Itrs, OriginId, ReferenceEpoch,
        State, StateTransform, Tirs,
    },
    math::{AngularSpeed, Length, Matrix3, Point3, Rotation, RotationTolerance, Speed, Vector3},
    time::{Instant, Tai},
};

#[test]
fn static_frames_expose_their_complete_semantics() {
    let icrs = Icrs::definition();
    assert_eq!(icrs.name(), "ICRS");
    assert_eq!(icrs.origin(), OriginId::NotApplicable);
    assert_eq!(icrs.axes(), Axes::IcrsAligned);

    let bcrs = Bcrs::definition();
    assert_eq!(bcrs.origin(), OriginId::SolarSystemBarycenter);
    assert_eq!(bcrs.axes(), Axes::IcrsAligned);

    let gcrs = Gcrs::definition();
    assert_eq!(gcrs.origin(), OriginId::EarthCenter);
    assert_eq!(gcrs.axes(), Axes::Gcrs);

    let cirs = Cirs::definition();
    assert_eq!(cirs.origin(), OriginId::EarthCenter);
    assert_eq!(cirs.axes(), Axes::CelestialIntermediate);
    assert_eq!(cirs.reference_epoch(), ReferenceEpoch::OfDate);

    let tirs = Tirs::definition();
    assert_eq!(tirs.origin(), OriginId::EarthCenter);
    assert_eq!(tirs.axes(), Axes::TerrestrialIntermediate);
    assert_eq!(tirs.reference_epoch(), ReferenceEpoch::OfDate);

    let itrs = Itrs::definition();
    assert_eq!(itrs.origin(), OriginId::EarthCenter);
    assert_eq!(itrs.axes(), Axes::Terrestrial);
}

#[test]
fn state_transform_applies_rotation_translation_and_velocity_terms() {
    let epoch = Instant::<Tai>::from_tai_nanoseconds_since_1900(123_456_789);
    let tolerance = RotationTolerance::new(1.0e-12, 1.0e-12).unwrap();
    let rotation = Rotation::<Gcrs, Itrs>::try_from_matrix(Matrix3::identity(), tolerance).unwrap();
    let transform = StateTransform::new(
        epoch,
        rotation,
        Vector3::new(
            AngularSpeed::from_radians_per_second(0.0).unwrap(),
            AngularSpeed::from_radians_per_second(0.0).unwrap(),
            AngularSpeed::from_radians_per_second(2.0).unwrap(),
        ),
        Vector3::new(
            Length::from_metres(10.0).unwrap(),
            Length::from_metres(20.0).unwrap(),
            Length::from_metres(30.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(1.0).unwrap(),
            Speed::from_metres_per_second(2.0).unwrap(),
            Speed::from_metres_per_second(3.0).unwrap(),
        ),
    );
    let state = State::new(
        Point3::new(
            Length::from_metres(3.0).unwrap(),
            Length::from_metres(4.0).unwrap(),
            Length::from_metres(5.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(7.0).unwrap(),
            Speed::from_metres_per_second(11.0).unwrap(),
            Speed::from_metres_per_second(13.0).unwrap(),
        ),
        epoch,
    );

    let transformed: State<Itrs, Tai> = transform.apply_state(state).unwrap();
    let position = transformed.position().position();
    assert_eq!(position.x().as_metres(), 13.0);
    assert_eq!(position.y().as_metres(), 24.0);
    assert_eq!(position.z().as_metres(), 35.0);
    let velocity = transformed.velocity();
    assert_eq!(velocity.x().as_metres_per_second(), 0.0);
    assert_eq!(velocity.y().as_metres_per_second(), 19.0);
    assert_eq!(velocity.z().as_metres_per_second(), 16.0);
    assert_eq!(transformed.epoch(), epoch);
}

#[test]
fn inverse_state_transform_round_trips_position_and_velocity() {
    let epoch = Instant::<Tai>::from_tai_nanoseconds_since_1900(987_654_321);
    let tolerance = RotationTolerance::new(1.0e-12, 1.0e-12).unwrap();
    let rotation = Rotation::<Gcrs, Itrs>::try_from_matrix(
        Matrix3::try_from_rows([[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]).unwrap(),
        tolerance,
    )
    .unwrap();
    let transform = StateTransform::new(
        epoch,
        rotation,
        Vector3::new(
            AngularSpeed::from_radians_per_second(0.01).unwrap(),
            AngularSpeed::from_radians_per_second(-0.02).unwrap(),
            AngularSpeed::from_radians_per_second(0.03).unwrap(),
        ),
        Vector3::new(
            Length::from_metres(100.0).unwrap(),
            Length::from_metres(-200.0).unwrap(),
            Length::from_metres(300.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(1.5).unwrap(),
            Speed::from_metres_per_second(-2.5).unwrap(),
            Speed::from_metres_per_second(3.5).unwrap(),
        ),
    );
    let original = State::new(
        Point3::new(
            Length::from_metres(7.0).unwrap(),
            Length::from_metres(11.0).unwrap(),
            Length::from_metres(13.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(17.0).unwrap(),
            Speed::from_metres_per_second(19.0).unwrap(),
            Speed::from_metres_per_second(23.0).unwrap(),
        ),
        epoch,
    );

    let transformed = transform.apply_state(original).unwrap();
    let recovered = transform
        .inverse()
        .unwrap()
        .apply_state(transformed)
        .unwrap();
    let recovered_position = recovered.position().position();
    let original_position = original.position().position();
    assert_abs_diff_eq!(
        recovered_position.x().as_metres(),
        original_position.x().as_metres(),
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        recovered_position.y().as_metres(),
        original_position.y().as_metres(),
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        recovered_position.z().as_metres(),
        original_position.z().as_metres(),
        epsilon = 1.0e-12
    );
    let recovered_velocity = recovered.velocity();
    let original_velocity = original.velocity();
    assert_abs_diff_eq!(
        recovered_velocity.x().as_metres_per_second(),
        original_velocity.x().as_metres_per_second(),
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        recovered_velocity.y().as_metres_per_second(),
        original_velocity.y().as_metres_per_second(),
        epsilon = 1.0e-12
    );
    assert_abs_diff_eq!(
        recovered_velocity.z().as_metres_per_second(),
        original_velocity.z().as_metres_per_second(),
        epsilon = 1.0e-12
    );
}

#[test]
fn composed_state_transform_matches_sequential_application() {
    let epoch = Instant::<Tai>::from_tai_nanoseconds_since_1900(42);
    let tolerance = RotationTolerance::new(1.0e-12, 1.0e-12).unwrap();
    let first = StateTransform::new(
        epoch,
        Rotation::<Gcrs, Cirs>::try_from_matrix(Matrix3::identity(), tolerance).unwrap(),
        Vector3::new(
            AngularSpeed::from_radians_per_second(0.0).unwrap(),
            AngularSpeed::from_radians_per_second(0.0).unwrap(),
            AngularSpeed::from_radians_per_second(2.0).unwrap(),
        ),
        Vector3::new(
            Length::from_metres(3.0).unwrap(),
            Length::from_metres(4.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(1.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
        ),
    );
    let second = StateTransform::new(
        epoch,
        Rotation::<Cirs, Itrs>::try_from_matrix(
            Matrix3::try_from_rows([[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]).unwrap(),
            tolerance,
        )
        .unwrap(),
        Vector3::new(
            AngularSpeed::from_radians_per_second(0.0).unwrap(),
            AngularSpeed::from_radians_per_second(0.0).unwrap(),
            AngularSpeed::from_radians_per_second(5.0).unwrap(),
        ),
        Vector3::new(
            Length::from_metres(0.0).unwrap(),
            Length::from_metres(6.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(0.0).unwrap(),
            Speed::from_metres_per_second(2.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
        ),
    );
    let state = State::new(
        Point3::new(
            Length::from_metres(7.0).unwrap(),
            Length::from_metres(8.0).unwrap(),
            Length::from_metres(9.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(10.0).unwrap(),
            Speed::from_metres_per_second(11.0).unwrap(),
            Speed::from_metres_per_second(12.0).unwrap(),
        ),
        epoch,
    );

    let sequential = second
        .apply_state(first.apply_state(state).unwrap())
        .unwrap();
    let composed = first.then(second).unwrap().apply_state(state).unwrap();
    assert_eq!(composed, sequential);
}

#[test]
fn state_transform_rejects_a_different_physical_epoch() {
    let transform_epoch = Instant::<Tai>::from_tai_nanoseconds_since_1900(100);
    let state_epoch = Instant::<Tai>::from_tai_nanoseconds_since_1900(101);
    let transform = StateTransform::<Gcrs, Gcrs, Tai>::identity(transform_epoch).unwrap();
    let state = State::new(
        Point3::new(
            Length::from_metres(0.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(0.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
            Speed::from_metres_per_second(0.0).unwrap(),
        ),
        state_epoch,
    );

    assert!(matches!(
        transform.apply_state(state),
        Err(Error::EpochMismatch {
            transform_tai_nanoseconds: 100,
            value_tai_nanoseconds: 101,
        })
    ));
    let following = StateTransform::<Gcrs, Gcrs, Tai>::identity(state_epoch).unwrap();
    assert!(matches!(
        transform.then(following),
        Err(Error::EpochMismatch {
            transform_tai_nanoseconds: 100,
            value_tai_nanoseconds: 101,
        })
    ));
}
