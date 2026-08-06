use core::{fmt, marker::PhantomData};

use super::{Duration, Error, Tai, TimeScale, TimeScaleModel};

/// A physical instant tagged with the scale used to represent it.
///
/// Internally every instant is stored as exact TAI nanoseconds since
/// 1900-01-01T00:00:00 TAI. Scale conversion therefore never compounds
/// rounding error.
pub struct Instant<S: TimeScale> {
    tai_nanoseconds_since_1900: i128,
    scale: PhantomData<S>,
}

impl<S: TimeScale> Instant<S> {
    /// Returns the exact internal TAI nanoseconds since 1900-01-01 TAI.
    pub const fn tai_nanoseconds_since_1900(self) -> i128 {
        self.tai_nanoseconds_since_1900
    }
    /// Converts a physical instant to target scale `S` through an explicit model.
    ///
    /// The internal TAI coordinate remains exact; the model is consulted to
    /// prove that it can represent this instant in `S`.
    pub fn from_instant<From, Model>(instant: Instant<From>, model: &Model) -> Result<Self, Error>
    where
        From: TimeScale,
        Model: TimeScaleModel<S>,
    {
        model.validate_instant(instant)?;
        Ok(instant.retag())
    }

    /// Adds a physical duration with overflow checking.
    pub fn checked_add(self, duration: Duration) -> Result<Self, Error> {
        self.tai_nanoseconds_since_1900
            .checked_add(duration.as_nanoseconds())
            .map(Self::from_tai_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "adding duration to instant",
            })
    }

    /// Subtracts a physical duration with overflow checking.
    pub fn checked_sub(self, duration: Duration) -> Result<Self, Error> {
        self.tai_nanoseconds_since_1900
            .checked_sub(duration.as_nanoseconds())
            .map(Self::from_tai_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "subtracting duration from instant",
            })
    }

    /// Returns the physical duration since an instant in the same scale.
    pub fn duration_since(self, earlier: Self) -> Result<Duration, Error> {
        self.tai_nanoseconds_since_1900
            .checked_sub(earlier.tai_nanoseconds_since_1900)
            .map(Duration::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "subtracting instants",
            })
    }

    /// Wraps the instant as a reference epoch.
    pub const fn as_epoch(self) -> Epoch<S> {
        Epoch { instant: self }
    }

    pub(crate) const fn from_tai_nanoseconds(tai_nanoseconds_since_1900: i128) -> Self {
        Self {
            tai_nanoseconds_since_1900,
            scale: PhantomData,
        }
    }

    pub(crate) const fn retag<T: TimeScale>(self) -> Instant<T> {
        Instant::from_tai_nanoseconds(self.tai_nanoseconds_since_1900)
    }
}

impl Instant<Tai> {
    /// Constructs an instant from exact TAI nanoseconds since 1900-01-01 TAI.
    pub const fn from_tai_nanoseconds_since_1900(nanoseconds: i128) -> Self {
        Self::from_tai_nanoseconds(nanoseconds)
    }
}

impl<S: TimeScale> Copy for Instant<S> {}

impl<S: TimeScale> Clone for Instant<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for Instant<S> {
    fn eq(&self, other: &Self) -> bool {
        self.tai_nanoseconds_since_1900 == other.tai_nanoseconds_since_1900
    }
}

impl<S: TimeScale> Eq for Instant<S> {}

impl<S: TimeScale> PartialOrd for Instant<S> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: TimeScale> Ord for Instant<S> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.tai_nanoseconds_since_1900
            .cmp(&other.tai_nanoseconds_since_1900)
    }
}

impl<S: TimeScale> fmt::Debug for Instant<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Instant")
            .field("scale", &S::NAME)
            .field(
                "tai_nanoseconds_since_1900",
                &self.tai_nanoseconds_since_1900,
            )
            .finish()
    }
}

/// A reference epoch carrying the scale of its underlying instant.
pub struct Epoch<S: TimeScale> {
    instant: Instant<S>,
}

impl<S: TimeScale> Epoch<S> {
    /// Constructs a reference epoch from a typed instant.
    pub const fn new(instant: Instant<S>) -> Self {
        Self { instant }
    }

    /// Returns the underlying instant.
    pub const fn instant(self) -> Instant<S> {
        self.instant
    }
}

impl<S: TimeScale> Copy for Epoch<S> {}

impl<S: TimeScale> Clone for Epoch<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for Epoch<S> {
    fn eq(&self, other: &Self) -> bool {
        self.instant == other.instant
    }
}

impl<S: TimeScale> Eq for Epoch<S> {}

impl<S: TimeScale> fmt::Debug for Epoch<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Epoch").field(&self.instant).finish()
    }
}

/// An exact POSIX timestamp that deliberately ignores leap seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixTimestamp {
    nanoseconds_since_1970: i128,
}

impl UnixTimestamp {
    /// The Unix epoch at 1970-01-01T00:00:00Z.
    pub const EPOCH: Self = Self {
        nanoseconds_since_1970: 0,
    };

    /// Constructs a POSIX timestamp from exact nanoseconds since the Unix epoch.
    pub const fn from_nanoseconds(nanoseconds: i128) -> Self {
        Self {
            nanoseconds_since_1970: nanoseconds,
        }
    }

    /// Returns exact nanoseconds since the Unix epoch.
    pub const fn as_nanoseconds(self) -> i128 {
        self.nanoseconds_since_1970
    }

    /// Adds a physical duration with overflow checking.
    pub fn checked_add(self, duration: Duration) -> Result<Self, Error> {
        self.nanoseconds_since_1970
            .checked_add(duration.as_nanoseconds())
            .map(Self::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "adding duration to Unix timestamp",
            })
    }

    /// Returns the nominal POSIX duration since another timestamp.
    pub fn duration_since(self, earlier: Self) -> Result<Duration, Error> {
        self.nanoseconds_since_1970
            .checked_sub(earlier.nanoseconds_since_1970)
            .map(Duration::from_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "subtracting Unix timestamps",
            })
    }
}
