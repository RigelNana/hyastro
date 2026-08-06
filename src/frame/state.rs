use crate::{
    math::{Point3, Speed, Vector3},
    time::{Instant, TimeScale},
};

use super::CoordinateFrame;

/// Position and velocity in one complete coordinate frame at one physical epoch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State<F: CoordinateFrame, S: TimeScale> {
    position: Point3<F>,
    velocity: Vector3<F, Speed>,
    epoch: Instant<S>,
}

impl<F: CoordinateFrame, S: TimeScale> State<F, S> {
    /// Constructs a state from a position, velocity, and typed physical epoch.
    pub const fn new(position: Point3<F>, velocity: Vector3<F, Speed>, epoch: Instant<S>) -> Self {
        Self {
            position,
            velocity,
            epoch,
        }
    }

    /// Returns the state position.
    pub const fn position(self) -> Point3<F> {
        self.position
    }

    /// Returns the state velocity.
    pub const fn velocity(self) -> Vector3<F, Speed> {
        self.velocity
    }

    /// Returns the physical epoch and its representation scale.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Decomposes the state into position, velocity, and epoch.
    pub fn into_parts(self) -> (Point3<F>, Vector3<F, Speed>, Instant<S>) {
        (self.position, self.velocity, self.epoch)
    }
}
