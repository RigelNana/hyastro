use hyastro::{
    frame::{Bcrs, Frames, Itrs},
    time::{EarthOrientationTable, Instant, Tai},
};

fn unsupported(
    frames: &Frames<'_, '_, EarthOrientationTable<'_>>,
    epoch: Instant<Tai>,
) {
    let _ = frames.at::<Bcrs, Itrs, Tai>(epoch);
}

fn main() {}
