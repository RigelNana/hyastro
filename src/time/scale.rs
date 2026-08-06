use core::fmt::Debug;

mod sealed {
    pub trait Sealed {}
}

/// A sealed marker describing a time scale carried by time values.
pub trait TimeScale: sealed::Sealed + Copy + Clone + Debug + Eq {
    /// Conventional short name of the scale.
    const NAME: &'static str;

    /// Whether every labeled day contains exactly 86,400 SI seconds.
    #[doc(hidden)]
    const UNIFORM_DAYS: bool;

    /// Whether civil labels may contain `23:59:60`.
    #[doc(hidden)]
    const LEAP_SECOND_LABELS: bool = false;

    /// Scale coordinate minus TAI, in exact nanoseconds, when fixed.
    #[doc(hidden)]
    const TAI_OFFSET_NANOSECONDS: Option<i128> = None;
}

/// International Atomic Time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tai;

impl sealed::Sealed for Tai {}

impl TimeScale for Tai {
    const NAME: &'static str = "TAI";
    const UNIFORM_DAYS: bool = true;
    const TAI_OFFSET_NANOSECONDS: Option<i128> = Some(0);
}

/// Coordinated Universal Time with leap-second labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Utc;

impl sealed::Sealed for Utc {}

impl TimeScale for Utc {
    const NAME: &'static str = "UTC";
    const UNIFORM_DAYS: bool = false;
    const LEAP_SECOND_LABELS: bool = true;
}

/// Terrestrial Time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tt;

impl sealed::Sealed for Tt {}

impl TimeScale for Tt {
    const NAME: &'static str = "TT";
    const UNIFORM_DAYS: bool = true;
    const TAI_OFFSET_NANOSECONDS: Option<i128> = Some(32_184_000_000);
}

/// Barycentric Dynamical Time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tdb;

impl sealed::Sealed for Tdb {}

impl TimeScale for Tdb {
    const NAME: &'static str = "TDB";
    const UNIFORM_DAYS: bool = true;
}

/// Geocentric Coordinate Time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tcg;

impl sealed::Sealed for Tcg {}

impl TimeScale for Tcg {
    const NAME: &'static str = "TCG";
    const UNIFORM_DAYS: bool = true;
}

/// Barycentric Coordinate Time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tcb;

impl sealed::Sealed for Tcb {}

impl TimeScale for Tcb {
    const NAME: &'static str = "TCB";
    const UNIFORM_DAYS: bool = true;
}

/// Universal Time 1 derived from Earth rotation data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ut1;

impl sealed::Sealed for Ut1 {}

impl TimeScale for Ut1 {
    const NAME: &'static str = "UT1";
    const UNIFORM_DAYS: bool = false;
}

/// GPS system time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gps;

impl sealed::Sealed for Gps {}

impl TimeScale for Gps {
    const NAME: &'static str = "GPS";
    const UNIFORM_DAYS: bool = true;
    const TAI_OFFSET_NANOSECONDS: Option<i128> = Some(-19_000_000_000);
}

/// POSIX/Unix time, which deliberately ignores leap seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Posix;

impl sealed::Sealed for Posix {}

impl TimeScale for Posix {
    const NAME: &'static str = "POSIX";
    const UNIFORM_DAYS: bool = true;
}
