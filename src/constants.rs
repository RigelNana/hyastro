//! Crate-internal sources of truth for physical, astronomical, and unit constants.
//!
//! Public quantity types may expose associated aliases, but every shared
//! numerical value is defined only here. Typed values such as frame metadata
//! and built-in data records remain with the types that own their invariants.

pub(crate) mod angle {
    use core::f64::consts::PI;

    /// Angular hours in one complete turn, exact by definition.
    pub(crate) const HOURS_PER_TURN: f64 = 24.0;
    /// Degrees in one angular hour, exact by definition.
    pub(crate) const DEGREES_PER_HOUR: f64 = 15.0;
    /// Sexagesimal minutes in one whole unit, exact by definition.
    pub(crate) const SEXAGESIMAL_MINUTES_PER_UNIT: f64 = 60.0;
    /// Sexagesimal seconds in one whole unit, exact by definition.
    pub(crate) const SEXAGESIMAL_SECONDS_PER_UNIT: f64 =
        SEXAGESIMAL_MINUTES_PER_UNIT * SEXAGESIMAL_MINUTES_PER_UNIT;
    /// Number of arcseconds in one degree, exact by definition.
    pub(crate) const ARCSECONDS_PER_DEGREE: f64 = 3_600.0;
    /// Number of milliarcseconds in one arcsecond, exact by definition.
    pub(crate) const MILLIARCSECONDS_PER_ARCSECOND: f64 = 1_000.0;
    /// Radians in one arcsecond, derived exactly from the radian-degree relation.
    pub(crate) const RADIANS_PER_ARCSECOND: f64 = PI / (180.0 * ARCSECONDS_PER_DEGREE);
}

pub(crate) mod length {
    /// Metres in one kilometre, exact by SI prefix definition.
    pub(crate) const METRES_PER_KILOMETRE: f64 = 1_000.0;
    /// Metres in one astronomical unit, exact under IAU 2012 Resolution B2.
    pub(crate) const METRES_PER_ASTRONOMICAL_UNIT: f64 = 149_597_870_700.0;
    /// Metres in one light-second, exact from the SI speed of light.
    pub(crate) const METRES_PER_LIGHT_SECOND: f64 = 299_792_458.0;
    /// Metres in one parsec, using the IAU astronomical-unit definition.
    pub(crate) const METRES_PER_PARSEC: f64 = 3.085_677_581_491_367e16;
}

pub(crate) mod body {
    /// Exact nominal solar radius from IAU 2015 Resolution B3.
    pub(crate) const IAU_2015_NOMINAL_SOLAR_RADIUS_METRES: f64 = 6.957e8;
    /// Lunar reference-sphere radius from the IAU WGCCRE 2015 report.
    pub(crate) const IAU_WGCCRE_2015_LUNAR_RADIUS_METRES: f64 = 1_737_400.0;
}

pub(crate) mod time {
    /// Nanoseconds in one SI microsecond, exact.
    pub(crate) const NANOSECONDS_PER_MICROSECOND: i128 = 1_000;
    /// Nanoseconds in one SI millisecond, exact.
    pub(crate) const NANOSECONDS_PER_MILLISECOND: i128 = 1_000_000;
    /// Nanoseconds in one SI second, exact.
    pub(crate) const NANOSECONDS_PER_SECOND: i128 = 1_000_000_000;
    /// SI seconds in one minute, exact.
    pub(crate) const SECONDS_PER_MINUTE: i128 = 60;
    /// Minutes in one hour, exact.
    pub(crate) const MINUTES_PER_HOUR: i128 = 60;
    /// Hours in one nominal day, exact.
    pub(crate) const HOURS_PER_DAY: i128 = 24;
    /// Milliseconds in one SI second, exact.
    pub(crate) const MILLISECONDS_PER_SECOND: f64 = 1_000.0;
    /// SI seconds in one hour, exact.
    pub(crate) const SECONDS_PER_HOUR: i128 = SECONDS_PER_MINUTE * MINUTES_PER_HOUR;
    /// SI seconds in one nominal Julian day, exact.
    pub(crate) const SECONDS_PER_DAY: i128 = HOURS_PER_DAY * SECONDS_PER_HOUR;
    /// Nanoseconds in one nominal Julian day, exact.
    pub(crate) const NANOSECONDS_PER_DAY: i128 = SECONDS_PER_DAY * NANOSECONDS_PER_SECOND;
    /// Nanoseconds in one 365.25-day Julian year, exact by definition.
    pub(crate) const NANOSECONDS_PER_JULIAN_YEAR: i128 = 36_525 * NANOSECONDS_PER_DAY / 100;
    /// Julian Date of J2000.0 in TT, exact by convention.
    pub(crate) const J2000_JULIAN_DATE: f64 = 2_451_545.0;
    #[cfg(feature = "hifitime")]
    /// IAU 1977 reference epoch, 1977-01-01T00:00:00 TAI, as a Julian Date.
    pub(crate) const IAU_1977_REFERENCE_JULIAN_DATE: f64 = 2_443_144.5;
    /// Exact coordinate offset TT−TAI in nanoseconds.
    pub(crate) const TT_MINUS_TAI_NANOSECONDS: i128 = 32_184_000_000;
    /// Exact coordinate offset GPS−TAI in nanoseconds.
    pub(crate) const GPS_MINUS_TAI_NANOSECONDS: i128 = -19_000_000_000;
    /// Largest integer exactly representable by binary64.
    pub(crate) const MAX_EXACT_BINARY64_INTEGER: i128 = 9_007_199_254_740_992;
}

#[cfg(feature = "std")]
pub(crate) mod earth {
    /// Conventional nominal Earth angular speed in radians per SI second.
    ///
    /// This is the IERS conventional value used with excess length of day, not
    /// a measured instantaneous rotation rate.
    pub(crate) const NOMINAL_ANGULAR_SPEED_RADIANS_PER_SECOND: f64 = 7.292_115_0e-5;
    /// Orthogonality tolerance for matrices returned by Earth-orientation models.
    pub(crate) const ROTATION_ORTHOGONALITY_TOLERANCE: f64 = 1.0e-12;
    /// Determinant tolerance for matrices returned by Earth-orientation models.
    pub(crate) const ROTATION_DETERMINANT_TOLERANCE: f64 = 1.0e-12;
    /// Half-width of the finite-difference stencil used for frame rates, in seconds.
    pub(crate) const ROTATION_RATE_DIFFERENCE_STEP_SECONDS: f64 = 3_600.0;
    /// Maximum accepted frame-rate extrapolation residual, in radians per second.
    pub(crate) const ROTATION_RATE_CONVERGENCE_TOLERANCE_RADIANS_PER_SECOND: f64 = 5.0e-16;
}
