use hyastro::{
    frame::{Frames, Gcrs, Itrs},
    time::{Instant, Tai},
};

fn unsupported(frames: &Frames<'_, '_, '_>, epoch: Instant<Tai>) {
    let _ = frames.at::<Gcrs, Itrs, Tai>(epoch);
}

fn main() {}
