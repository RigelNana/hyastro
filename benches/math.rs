use std::hint::black_box;

use criterion::Criterion;
use hyastro::{
    frame::{EquatorialDirection, Icrs},
    math::{Angle, Declination, Length, RightAscension, RootOptions, Rotation, Vector3},
};

struct Inertial;

fn main() {
    let mut criterion = Criterion::default().configure_from_args();

    criterion.bench_function("right ascension normalization", |bencher| {
        bencher.iter(|| RightAscension::wrap_radians(black_box(-123.456)))
    });

    let vector = Vector3::<Inertial, Length>::new(
        Length::from_metres(1.0).unwrap(),
        Length::from_metres(2.0).unwrap(),
        Length::from_metres(3.0).unwrap(),
    );
    let rotation =
        Rotation::<Inertial, Inertial>::around_z(Angle::from_degrees(23.4).unwrap()).unwrap();
    criterion.bench_function("typed vector rotation", |bencher| {
        bencher.iter(|| rotation.apply_vector(black_box(vector)))
    });

    let left = EquatorialDirection::<Icrs>::new(
        RightAscension::try_from_degrees(12.0).unwrap(),
        Declination::try_from_degrees(-30.0).unwrap(),
    );
    let right = EquatorialDirection::<Icrs>::new(
        RightAscension::try_from_degrees(210.0).unwrap(),
        Declination::try_from_degrees(45.0).unwrap(),
    );
    criterion.bench_function("great circle separation", |bencher| {
        bencher.iter(|| left.separation_to(black_box(right)))
    });

    let root_options = RootOptions::new(1.0e-12, 1.0e-12, 100).unwrap();
    criterion.bench_function("bracketed root", |bencher| {
        bencher.iter(|| root_options.bisect(black_box(0.0), black_box(2.0), |x| x * x - 2.0))
    });

    criterion.final_summary();
}
