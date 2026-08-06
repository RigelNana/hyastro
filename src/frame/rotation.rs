use crate::{
    math::{Coordinate, Direction, Rotation, Vector3},
    time::{Instant, TimeScale},
};

use super::{CoordinateFrame, Error};

/// A source-to-target component rotation valid at one physical epoch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameRotation<From, To, S>
where
    From: CoordinateFrame,
    To: CoordinateFrame,
    S: TimeScale,
{
    epoch: Instant<S>,
    rotation: Rotation<From, To>,
}

impl<From, To, S> FrameRotation<From, To, S>
where
    From: CoordinateFrame,
    To: CoordinateFrame,
    S: TimeScale,
{
    /// Associates a validated typed rotation with its physical epoch.
    pub const fn new(epoch: Instant<S>, rotation: Rotation<From, To>) -> Self {
        Self { epoch, rotation }
    }

    /// Returns the physical epoch at which this rotation is valid.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the validated source-to-target component rotation.
    pub const fn rotation(self) -> Rotation<From, To> {
        self.rotation
    }

    /// Applies the component rotation to a free vector.
    pub fn apply_vector<Q: Coordinate>(
        self,
        vector: Vector3<From, Q>,
    ) -> Result<Vector3<To, Q>, Error> {
        self.rotation.apply_vector(vector).map_err(Error::from)
    }

    /// Applies the component rotation to a unit direction.
    pub fn apply_direction(self, direction: Direction<From>) -> Result<Direction<To>, Error> {
        self.rotation
            .apply_direction(direction)
            .map_err(Error::from)
    }

    /// Returns the inverse rotation at the same physical epoch.
    pub fn inverse(self) -> FrameRotation<To, From, S> {
        FrameRotation::new(self.epoch, self.rotation.inverse())
    }

    /// Composes this rotation with a following rotation at the same epoch.
    pub fn then<Next>(
        self,
        next: FrameRotation<To, Next, S>,
    ) -> Result<FrameRotation<From, Next, S>, Error>
    where
        Next: CoordinateFrame,
    {
        self.ensure_epoch(next.epoch)?;
        Ok(FrameRotation::new(
            self.epoch,
            self.rotation.then(next.rotation)?,
        ))
    }

    fn ensure_epoch(self, value: Instant<S>) -> Result<(), Error> {
        let rotation_tai_nanoseconds = self.epoch.tai_nanoseconds_since_1900();
        let value_tai_nanoseconds = value.tai_nanoseconds_since_1900();
        if rotation_tai_nanoseconds == value_tai_nanoseconds {
            Ok(())
        } else {
            Err(Error::epoch_mismatch(
                rotation_tai_nanoseconds,
                value_tai_nanoseconds,
            ))
        }
    }
}
