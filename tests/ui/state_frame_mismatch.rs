use hyastro::{
    frame::{Gcrs, Itrs, State},
    math::{Length, Point3, Speed, Vector3},
    time::{Instant, Tai},
};

fn accepts_itrs(_: State<Itrs, Tai>) {}

fn main() {
    let gcrs = State::<Gcrs, Tai>::new(
        Point3::new(
            Length::from_metres(1.0).unwrap(),
            Length::from_metres(2.0).unwrap(),
            Length::from_metres(3.0).unwrap(),
        ),
        Vector3::new(
            Speed::from_metres_per_second(4.0).unwrap(),
            Speed::from_metres_per_second(5.0).unwrap(),
            Speed::from_metres_per_second(6.0).unwrap(),
        ),
        Instant::from_tai_nanoseconds_since_1900(0),
    );

    accepts_itrs(gcrs);
}
