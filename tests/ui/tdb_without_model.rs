use hyastro::time::{Instant, Tai, Tdb, TimeContext};

fn main() {
    let context = TimeContext::builtin();
    let tai = Instant::<Tai>::from_tai_nanoseconds_since_1900(0);
    let _ = Instant::<Tdb>::from_instant(tai, &context);
}
