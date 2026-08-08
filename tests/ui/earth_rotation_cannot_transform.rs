use hyastro::{
    frame::{Frames, Gcrs, Itrs},
    time::{EarthRotationTable, Instant, Tai},
};

fn cannot_transform(
    frames: &Frames<'_, '_, EarthRotationTable<'_>>,
    epoch: Instant<Tai>,
) {
    let _ = frames.at::<Gcrs, Itrs, Tai>(epoch);
}

fn main() {}
