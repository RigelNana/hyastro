use hyastro::{
    time::{EarthRotationSample, Instant, Tai, TimeContext, Ut1MinusUtc, Utc},
    uncertainty::StandardUncertainty,
};

fn main() {
    let epoch = Instant::<Utc>::from_instant(
        Instant::<Tai>::from_tai_nanoseconds_since_1900(0),
        &TimeContext::builtin(),
    )
    .unwrap();
    let sample = EarthRotationSample::new(epoch, Ut1MinusUtc::from_seconds(0.0).unwrap());
    let angular =
        StandardUncertainty::new(hyastro::math::Angle::from_degrees(0.1).unwrap()).unwrap();
    let _ = sample.with_standard_uncertainty(angular);
}
