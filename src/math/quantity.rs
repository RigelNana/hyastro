use core::{fmt::Debug, marker::PhantomData};

use crate::constants::{
    length::{
        METRES_PER_ASTRONOMICAL_UNIT, METRES_PER_KILOMETRE, METRES_PER_LIGHT_SECOND,
        METRES_PER_PARSEC,
    },
    time::SECONDS_PER_DAY,
};

use super::Error;

mod sealed {
    pub trait Sealed {}
}

/// A finite scalar that may be stored in a [`Vector3`](super::Vector3).
///
/// This trait is sealed so the crate controls dimensional multiplication.
pub trait Coordinate: sealed::Sealed + Copy + Clone + Debug + PartialEq {
    /// Quantity produced by multiplying two coordinates of this type.
    type Product: Coordinate;

    /// Returns the value in the type's canonical unit.
    #[doc(hidden)]
    fn canonical(self) -> f64;

    /// Constructs a coordinate from its canonical unit.
    #[doc(hidden)]
    fn try_from_canonical(value: f64) -> Result<Self, Error>;
}

/// A finite unitless scalar.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Dimensionless(f64);

impl Dimensionless {
    /// Constructs a finite unitless value.
    pub fn new(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("dimensionless value", value).map(Self)
    }
    pub(crate) const fn from_finite(value: f64) -> Self {
        Self(value)
    }

    /// Returns the unitless value.
    pub const fn value(self) -> f64 {
        self.0
    }

    /// Adds another value while preserving the finite invariant.
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        Self::new(self.0 + rhs.0)
    }

    /// Subtracts another value while preserving the finite invariant.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        Self::new(self.0 - rhs.0)
    }

    /// Scales the value while preserving the finite invariant.
    pub fn checked_scale(self, factor: f64) -> Result<Self, Error> {
        Error::ensure_finite("scale factor", factor)?;
        Self::new(self.0 * factor)
    }
}

impl sealed::Sealed for Dimensionless {}

impl Coordinate for Dimensionless {
    type Product = Dimensionless;

    fn canonical(self) -> f64 {
        self.0
    }

    fn try_from_canonical(value: f64) -> Result<Self, Error> {
        Self::new(value)
    }
}

/// A finite length stored canonically in metres.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Length(f64);

impl Length {
    /// Number of metres in one astronomical unit.
    pub const METRES_PER_AU: f64 = METRES_PER_ASTRONOMICAL_UNIT;
    /// Number of metres in one light-second.
    pub const METRES_PER_LIGHT_SECOND: f64 = METRES_PER_LIGHT_SECOND;
    /// Number of metres in one parsec.
    pub const METRES_PER_PARSEC: f64 = METRES_PER_PARSEC;

    pub(crate) const fn from_finite(value: f64) -> Self {
        Self(value)
    }

    /// Constructs a length in metres.
    pub fn from_metres(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("length", value).map(Self)
    }

    /// Constructs a length in kilometres.
    pub fn from_kilometres(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("kilometres", value)?;
        Self::from_metres(value * METRES_PER_KILOMETRE)
    }

    /// Constructs a length in astronomical units.
    pub fn from_astronomical_units(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("astronomical units", value)?;
        Self::from_metres(value * Self::METRES_PER_AU)
    }

    /// Constructs a length in light-seconds.
    pub fn from_light_seconds(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("light-seconds", value)?;
        Self::from_metres(value * Self::METRES_PER_LIGHT_SECOND)
    }

    /// Constructs a length in parsecs.
    pub fn from_parsecs(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("parsecs", value)?;
        Self::from_metres(value * Self::METRES_PER_PARSEC)
    }

    /// Returns the length in metres.
    pub const fn as_metres(self) -> f64 {
        self.0
    }

    /// Returns the length in kilometres.
    pub fn as_kilometres(self) -> f64 {
        self.0 / METRES_PER_KILOMETRE
    }

    /// Returns the length in astronomical units.
    pub fn as_astronomical_units(self) -> f64 {
        self.0 / Self::METRES_PER_AU
    }

    /// Returns the length in light-seconds.
    pub fn as_light_seconds(self) -> f64 {
        self.0 / Self::METRES_PER_LIGHT_SECOND
    }

    /// Returns the length in parsecs.
    pub fn as_parsecs(self) -> f64 {
        self.0 / Self::METRES_PER_PARSEC
    }

    /// Adds another length while preserving the finite invariant.
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        Self::from_metres(self.0 + rhs.0)
    }

    /// Subtracts another length while preserving the finite invariant.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        Self::from_metres(self.0 - rhs.0)
    }

    /// Scales the length while preserving the finite invariant.
    pub fn checked_scale(self, factor: f64) -> Result<Self, Error> {
        Error::ensure_finite("length scale factor", factor)?;
        Self::from_metres(self.0 * factor)
    }
}

impl sealed::Sealed for Length {}

impl Coordinate for Length {
    type Product = Squared<Length>;

    fn canonical(self) -> f64 {
        self.0
    }

    fn try_from_canonical(value: f64) -> Result<Self, Error> {
        Self::from_metres(value)
    }
}

/// A finite speed stored canonically in metres per second.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Speed(f64);

impl Speed {
    #[cfg(feature = "std")]
    pub(crate) const fn from_finite(value: f64) -> Self {
        Self(value)
    }

    /// Constructs a speed in metres per second.
    pub fn from_metres_per_second(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("speed", value).map(Self)
    }

    /// Constructs a speed in kilometres per second.
    pub fn from_kilometres_per_second(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("kilometres per second", value)?;
        Self::from_metres_per_second(value * METRES_PER_KILOMETRE)
    }

    /// Constructs a speed in astronomical units per day.
    pub fn from_astronomical_units_per_day(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("astronomical units per day", value)?;
        Self::from_metres_per_second(value * METRES_PER_ASTRONOMICAL_UNIT / SECONDS_PER_DAY as f64)
    }

    /// Returns the speed in metres per second.
    pub const fn as_metres_per_second(self) -> f64 {
        self.0
    }

    /// Returns the speed in kilometres per second.
    pub fn as_kilometres_per_second(self) -> f64 {
        self.0 / METRES_PER_KILOMETRE
    }

    /// Returns the speed in astronomical units per day.
    pub fn as_astronomical_units_per_day(self) -> f64 {
        self.0 * SECONDS_PER_DAY as f64 / METRES_PER_ASTRONOMICAL_UNIT
    }

    /// Adds another speed while preserving the finite invariant.
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        Self::from_metres_per_second(self.0 + rhs.0)
    }

    /// Subtracts another speed while preserving the finite invariant.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        Self::from_metres_per_second(self.0 - rhs.0)
    }

    /// Scales the speed while preserving the finite invariant.
    pub fn checked_scale(self, factor: f64) -> Result<Self, Error> {
        Error::ensure_finite("speed scale factor", factor)?;
        Self::from_metres_per_second(self.0 * factor)
    }
}

impl sealed::Sealed for Speed {}

impl Coordinate for Speed {
    type Product = Squared<Speed>;

    fn canonical(self) -> f64 {
        self.0
    }

    fn try_from_canonical(value: f64) -> Result<Self, Error> {
        Self::from_metres_per_second(value)
    }
}

/// A finite acceleration stored canonically in metres per second squared.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Acceleration(f64);

impl Acceleration {
    /// Constructs an acceleration in metres per second squared.
    pub fn from_metres_per_second_squared(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("acceleration", value).map(Self)
    }

    /// Returns the acceleration in metres per second squared.
    pub const fn as_metres_per_second_squared(self) -> f64 {
        self.0
    }

    /// Adds another acceleration while preserving the finite invariant.
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        Self::from_metres_per_second_squared(self.0 + rhs.0)
    }

    /// Subtracts another acceleration while preserving the finite invariant.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        Self::from_metres_per_second_squared(self.0 - rhs.0)
    }

    /// Scales the acceleration while preserving the finite invariant.
    pub fn checked_scale(self, factor: f64) -> Result<Self, Error> {
        Error::ensure_finite("acceleration scale factor", factor)?;
        Self::from_metres_per_second_squared(self.0 * factor)
    }
}

impl sealed::Sealed for Acceleration {}

impl Coordinate for Acceleration {
    type Product = Squared<Acceleration>;

    fn canonical(self) -> f64 {
        self.0
    }

    fn try_from_canonical(value: f64) -> Result<Self, Error> {
        Self::from_metres_per_second_squared(value)
    }
}

/// A finite angular speed stored canonically in radians per second.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AngularSpeed(f64);

impl AngularSpeed {
    /// Constructs an angular speed in radians per second.
    pub fn from_radians_per_second(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("angular speed", value).map(Self)
    }

    /// Returns the angular speed in radians per second.
    pub const fn as_radians_per_second(self) -> f64 {
        self.0
    }

    /// Adds another angular speed while preserving the finite invariant.
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        Self::from_radians_per_second(self.0 + rhs.0)
    }

    /// Subtracts another angular speed while preserving the finite invariant.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        Self::from_radians_per_second(self.0 - rhs.0)
    }

    /// Scales the angular speed while preserving the finite invariant.
    pub fn checked_scale(self, factor: f64) -> Result<Self, Error> {
        Error::ensure_finite("angular speed scale factor", factor)?;
        Self::from_radians_per_second(self.0 * factor)
    }
}

impl sealed::Sealed for AngularSpeed {}

impl Coordinate for AngularSpeed {
    type Product = Squared<AngularSpeed>;

    fn canonical(self) -> f64 {
        self.0
    }

    fn try_from_canonical(value: f64) -> Result<Self, Error> {
        Self::from_radians_per_second(value)
    }
}

/// A finite scalar representing the product of two values of the same quantity.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Squared<Q: Coordinate> {
    value: f64,
    quantity: PhantomData<Q>,
}

impl<Q: Coordinate> Squared<Q> {
    /// Constructs a squared quantity from its canonical product unit.
    pub fn new(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("squared quantity", value)?;
        Ok(Self {
            value,
            quantity: PhantomData,
        })
    }

    /// Returns the value in the canonical product unit.
    pub const fn value(self) -> f64 {
        self.value
    }

    /// Scales the value while preserving the finite invariant.
    pub fn checked_scale(self, factor: f64) -> Result<Self, Error> {
        Error::ensure_finite("squared quantity scale factor", factor)?;
        Self::new(self.value * factor)
    }
}

impl<Q: Coordinate> sealed::Sealed for Squared<Q> {}

impl<Q: Coordinate> Coordinate for Squared<Q> {
    type Product = Squared<Squared<Q>>;

    fn canonical(self) -> f64 {
        self.value
    }

    fn try_from_canonical(value: f64) -> Result<Self, Error> {
        Self::new(value)
    }
}
