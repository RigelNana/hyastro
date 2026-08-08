//! Strongly typed calendars, time scales, instants, durations, and library adapters.

mod calendar;
mod civil;
mod context;
mod delta_t;
mod duration;
mod earth_attitude;
mod earth_rotation;
mod eop;
mod error;
mod fixed_offset;
#[cfg(feature = "hifitime")]
mod hifitime;
#[cfg(feature = "std")]
mod iers;
mod instant;
mod interval;
#[cfg(feature = "jiff")]
mod jiff;
mod julian;
mod leap;
mod scale;
#[cfg(feature = "std")]
mod tdb;

pub use calendar::{Calendar, Date, Gregorian, Julian, JulianDayNumber, Weekday};
pub use civil::{DateTime, TimeOfDay};
pub use context::{NoEarthOrientation, TimeContext, TimeScaleModel};
pub use delta_t::DeltaT;
pub use duration::Duration;
pub use earth_attitude::{EarthAttitude, EarthAttitudeSample, EarthAttitudeTable};
pub use earth_rotation::{EarthRotation, EarthRotationSample, EarthRotationTable};
pub use eop::{
    CelestialPoleOffsetX, CelestialPoleOffsetY, EarthOrientation, EarthOrientationSample,
    EarthOrientationTable, ExcessLengthOfDay, PolarMotionX, PolarMotionY, Ut1MinusUtc,
};
pub use error::Error;
pub use fixed_offset::{CivilDateTime, FixedUtcOffset};
#[cfg(feature = "hifitime")]
pub use hifitime::{Hifitime, HifitimeScale};
#[cfg(feature = "std")]
pub use iers::{
    EarthOrientationAcceptance, EarthOrientationData, EarthOrientationProduct,
    EarthOrientationRecord, IersC04, IersFinals2000A,
};
pub use instant::{Epoch, Instant, UnixTimestamp};
pub use interval::TimeInterval;
#[cfg(feature = "jiff")]
pub use jiff::Jiff;
pub use julian::{BesselianEpoch, JulianDate, JulianEpoch, ModifiedJulianDate};
pub use leap::{LeapKind, LeapSecond, LeapSeconds};
pub use scale::{Gps, Posix, Tai, Tcb, Tcg, Tdb, TimeScale, Tt, Ut1, Utc};
#[cfg(feature = "std")]
pub use tdb::{GeocentricTdb, TdbSolution};
