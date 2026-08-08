use core::{fmt, marker::PhantomData};

use crate::{
    math::{Length, Speed, Vector3},
    time::{Instant, TimeScale},
};

use super::{CelestialBody, Error};

/// A request for one target relative to one centre at a physical epoch in frame `F`.
pub struct EphemerisQuery<F, S: TimeScale> {
    target: CelestialBody,
    center: CelestialBody,
    epoch: Instant<S>,
    frame: PhantomData<F>,
}

impl<F, S: TimeScale> EphemerisQuery<F, S> {
    /// Constructs a state query.
    pub const fn new(target: CelestialBody, center: CelestialBody, epoch: Instant<S>) -> Self {
        Self {
            target,
            center,
            epoch,
            frame: PhantomData,
        }
    }

    /// Returns the requested target.
    pub const fn target(self) -> CelestialBody {
        self.target
    }

    /// Returns the requested centre.
    pub const fn center(self) -> CelestialBody {
        self.center
    }

    /// Returns the requested physical epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }
}

impl<F, S: TimeScale> Copy for EphemerisQuery<F, S> {}

impl<F, S: TimeScale> Clone for EphemerisQuery<F, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F, S: TimeScale> PartialEq for EphemerisQuery<F, S> {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target && self.center == other.center && self.epoch == other.epoch
    }
}

impl<F, S: TimeScale> Eq for EphemerisQuery<F, S> {}

impl<F, S: TimeScale> fmt::Debug for EphemerisQuery<F, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EphemerisQuery")
            .field("target", &self.target)
            .field("center", &self.center)
            .field("epoch", &self.epoch)
            .finish()
    }
}

/// A target-centre Cartesian state whose vector axes are encoded by frame `F`.
///
/// The centre is carried explicitly rather than being implied by `F`: a relative state is a pair
/// of free vectors, not a [`Point3`](crate::math::Point3) tied to a fixed frame origin.
pub struct RelativeState<F, S: TimeScale> {
    target: CelestialBody,
    center: CelestialBody,
    position: Vector3<F, Length>,
    velocity: Vector3<F, Speed>,
    epoch: Instant<S>,
}

impl<F, S: TimeScale> RelativeState<F, S> {
    /// Constructs a finite relative state and enforces the zero identity-state invariant.
    pub fn try_new(
        target: CelestialBody,
        center: CelestialBody,
        position: Vector3<F, Length>,
        velocity: Vector3<F, Speed>,
        epoch: Instant<S>,
    ) -> Result<Self, Error> {
        if target == center && !Self::vectors_are_zero(position, velocity) {
            return Err(Error::NonZeroIdentityState { body: target });
        }
        Ok(Self {
            target,
            center,
            position,
            velocity,
            epoch,
        })
    }

    /// Constructs the exact zero state of a body relative to itself.
    pub fn zero(body: CelestialBody, epoch: Instant<S>) -> Result<Self, Error> {
        Self::try_new(
            body,
            body,
            Vector3::new(
                Length::from_metres(0.0)?,
                Length::from_metres(0.0)?,
                Length::from_metres(0.0)?,
            ),
            Vector3::new(
                Speed::from_metres_per_second(0.0)?,
                Speed::from_metres_per_second(0.0)?,
                Speed::from_metres_per_second(0.0)?,
            ),
            epoch,
        )
    }

    /// Returns the target body.
    pub const fn target(self) -> CelestialBody {
        self.target
    }

    /// Returns the centre body.
    pub const fn center(self) -> CelestialBody {
        self.center
    }

    /// Returns target position relative to the centre.
    pub const fn position(self) -> Vector3<F, Length> {
        self.position
    }

    /// Returns target velocity relative to the centre.
    pub const fn velocity(self) -> Vector3<F, Speed> {
        self.velocity
    }

    /// Returns the physical evaluation epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Reverses the target-centre direction and negates both vectors.
    pub fn checked_reversed(self) -> Result<Self, Error> {
        Self::try_new(
            self.center,
            self.target,
            self.position.checked_scale(-1.0)?,
            self.velocity.checked_scale(-1.0)?,
            self.epoch,
        )
    }

    /// Chains `target → centre` with `centre → next centre` at the same epoch.
    pub fn checked_chain(self, next: Self) -> Result<Self, Error> {
        if self.center != next.target {
            return Err(Error::DisconnectedChain {
                left_center: self.center,
                right_target: next.target,
            });
        }
        Self::ensure_same_epoch(self.epoch, next.epoch)?;
        Self::try_new(
            self.target,
            next.center,
            self.position.checked_add(next.position)?,
            self.velocity.checked_add(next.velocity)?,
            self.epoch,
        )
    }

    fn ensure_same_epoch(left: Instant<S>, right: Instant<S>) -> Result<(), Error> {
        if left == right {
            Ok(())
        } else {
            Err(Error::EpochMismatch {
                left_tai_nanoseconds: left.tai_nanoseconds_since_1900(),
                right_tai_nanoseconds: right.tai_nanoseconds_since_1900(),
            })
        }
    }

    fn vectors_are_zero(position: Vector3<F, Length>, velocity: Vector3<F, Speed>) -> bool {
        let [x, y, z] = position.components();
        let [vx, vy, vz] = velocity.components();
        x.as_metres() == 0.0
            && y.as_metres() == 0.0
            && z.as_metres() == 0.0
            && vx.as_metres_per_second() == 0.0
            && vy.as_metres_per_second() == 0.0
            && vz.as_metres_per_second() == 0.0
    }
}

impl<F, S: TimeScale> Copy for RelativeState<F, S> {}

impl<F, S: TimeScale> Clone for RelativeState<F, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F, S: TimeScale> PartialEq for RelativeState<F, S> {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target
            && self.center == other.center
            && self.position == other.position
            && self.velocity == other.velocity
            && self.epoch == other.epoch
    }
}

impl<F, S: TimeScale> fmt::Debug for RelativeState<F, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RelativeState")
            .field("target", &self.target)
            .field("center", &self.center)
            .field("position", &self.position)
            .field("velocity", &self.velocity)
            .field("epoch", &self.epoch)
            .finish()
    }
}

/// A closed physical-time interval over which one target-centre query is available.
pub struct Coverage<F, S: TimeScale> {
    target: CelestialBody,
    center: CelestialBody,
    start: Instant<S>,
    end: Instant<S>,
    frame: PhantomData<F>,
}

impl<F, S: TimeScale> Coverage<F, S> {
    #[cfg(feature = "anise")]
    pub(crate) const fn from_ordered(
        target: CelestialBody,
        center: CelestialBody,
        start: Instant<S>,
        end: Instant<S>,
    ) -> Self {
        Self {
            target,
            center,
            start,
            end,
            frame: PhantomData,
        }
    }

    /// Returns the covered target.
    pub const fn target(self) -> CelestialBody {
        self.target
    }

    /// Returns the covered centre.
    pub const fn center(self) -> CelestialBody {
        self.center
    }

    /// Returns the inclusive coverage start.
    pub const fn start(self) -> Instant<S> {
        self.start
    }

    /// Returns the inclusive coverage end.
    pub const fn end(self) -> Instant<S> {
        self.end
    }

    /// Reports whether the interval contains a physical instant.
    pub fn contains(self, epoch: Instant<S>) -> bool {
        self.start <= epoch && epoch <= self.end
    }
}

impl<F, S: TimeScale> Copy for Coverage<F, S> {}

impl<F, S: TimeScale> Clone for Coverage<F, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F, S: TimeScale> PartialEq for Coverage<F, S> {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target
            && self.center == other.center
            && self.start == other.start
            && self.end == other.end
    }
}

impl<F, S: TimeScale> Eq for Coverage<F, S> {}

impl<F, S: TimeScale> fmt::Debug for Coverage<F, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Coverage")
            .field("target", &self.target)
            .field("center", &self.center)
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}
