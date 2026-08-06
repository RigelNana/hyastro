use hyastro::{
    frame::{Bcrs, Frames, Itrs},
    time::{Instant, Tai},
};

fn unsupported(frames: &Frames<'_, '_, '_>, epoch: Instant<Tai>) {
    let _ = frames.at::<Bcrs, Itrs, Tai>(epoch);
}

fn main() {}
