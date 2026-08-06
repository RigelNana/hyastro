use hyastro::time::{Instant, Tai, TimeContext, Ut1};

fn main() {
    let context = TimeContext::builtin();
    let tai = Instant::<Tai>::from_tai_nanoseconds_since_1900(0);
    let _ = Instant::<Ut1>::from_instant(tai, &context);
}
