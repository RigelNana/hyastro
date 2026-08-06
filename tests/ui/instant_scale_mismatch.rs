use hyastro::time::{Instant, Tai, Tt};

fn compare(tai: Instant<Tai>, tt: Instant<Tt>) {
    let _ = tai.duration_since(tt);
}

fn main() {}
