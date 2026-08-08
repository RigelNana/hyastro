use hyastro::{
    frame::{Cirs, Frames, State, Tirs},
    time::{EarthOrientationTable, Tai, Tt},
};

fn mismatch(
    frames: &Frames<'_, '_, EarthOrientationTable<'_>>,
    state: State<Cirs, Tai>,
) {
    let _: State<Tirs, Tt> = frames.transform(state).unwrap();
}

fn main() {}
