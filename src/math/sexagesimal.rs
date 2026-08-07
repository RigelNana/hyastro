use core::{fmt, str::FromStr};

use libm::{floor, round};

use crate::constants::angle::{
    HOURS_PER_TURN, SEXAGESIMAL_MINUTES_PER_UNIT, SEXAGESIMAL_SECONDS_PER_UNIT,
};

use super::Error;

const HOURS_MINUTES_SECONDS_FORMAT: &str = "hours-minutes-seconds";
const DEGREES_MINUTES_SECONDS_FORMAT: &str = "degrees-minutes-seconds";

/// The explicit sign of a sexagesimal angle.
///
/// Keeping the sign separate preserves negative zero, for example `-00°00′00″`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SexagesimalSign {
    /// A positive value, including positive zero.
    Positive,
    /// A negative value, including negative zero.
    Negative,
}

impl SexagesimalSign {
    /// Returns whether this is the negative sign.
    pub const fn is_negative(self) -> bool {
        matches!(self, Self::Negative)
    }

    const fn character(self) -> char {
        match self {
            Self::Positive => '+',
            Self::Negative => '-',
        }
    }
}

/// A canonical cyclic angle represented as hours, minutes, and seconds.
///
/// Values are restricted to `[0h, 24h)`. This is an angular representation:
/// it has no relationship to civil time or leap seconds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct HoursMinutesSeconds {
    hours: u8,
    minutes: u8,
    seconds: f64,
}

impl HoursMinutesSeconds {
    /// Constructs a canonical HMS angle in `[0h, 24h)`.
    pub fn new(hours: u8, minutes: u8, seconds: f64) -> Result<Self, Error> {
        if f64::from(hours) >= HOURS_PER_TURN {
            return Err(Error::OutOfRange {
                field: "sexagesimal hours",
                value: f64::from(hours),
                interval: "[0, 24)",
                unit: "h",
            });
        }
        Self::validate_subunit("sexagesimal minutes", minutes)?;
        Error::ensure_finite("sexagesimal seconds", seconds)?;
        if !(0.0..SEXAGESIMAL_MINUTES_PER_UNIT).contains(&seconds) {
            return Err(Error::OutOfRange {
                field: "sexagesimal seconds",
                value: seconds,
                interval: "[0, 60)",
                unit: "s",
            });
        }
        Ok(Self {
            hours,
            minutes,
            seconds: if seconds == 0.0 { 0.0 } else { seconds },
        })
    }

    /// Constructs canonical HMS from decimal angular hours without wrapping.
    pub fn from_decimal_hours(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("decimal angular hours", value)?;
        if !(0.0..HOURS_PER_TURN).contains(&value) {
            return Err(Error::OutOfRange {
                field: "decimal angular hours",
                value,
                interval: "[0, 24)",
                unit: "h",
            });
        }
        Ok(Self::from_valid_decimal_hours(value))
    }

    /// Parses colon-separated, unit-suffixed, or whitespace-separated HMS.
    ///
    /// Accepted examples include `12:34:56.7`, `12h34m56.7s`, and
    /// `12 34 56.7`. A leading `+` is accepted; negative HMS is rejected.
    pub fn parse(input: &str) -> Result<Self, Error> {
        let input = input.trim();
        let input = input.strip_prefix('+').unwrap_or(input);
        if input.is_empty() || input.starts_with('-') || input.starts_with('−') {
            return Err(Self::syntax_error());
        }
        let (hours, minutes, seconds) = SexagesimalParser::hms_components(input)?;
        Self::new(hours, minutes, seconds)
    }

    /// Returns the whole angular hours in `0..=23`.
    pub const fn hours(self) -> u8 {
        self.hours
    }

    /// Returns the whole sexagesimal minutes in `0..=59`.
    pub const fn minutes(self) -> u8 {
        self.minutes
    }

    /// Returns the fractional sexagesimal seconds in `[0, 60)`.
    pub const fn seconds(self) -> f64 {
        self.seconds
    }

    /// Returns the represented value as decimal angular hours.
    pub fn as_decimal_hours(self) -> f64 {
        f64::from(self.hours)
            + f64::from(self.minutes) / SEXAGESIMAL_MINUTES_PER_UNIT
            + self.seconds / SEXAGESIMAL_SECONDS_PER_UNIT
    }

    pub(crate) fn from_valid_decimal_hours(value: f64) -> Self {
        let value = if value == 0.0 { 0.0 } else { value };
        let mut hours = floor(value) as u8;
        let fractional_hours = value - f64::from(hours);
        let decimal_minutes = fractional_hours * SEXAGESIMAL_MINUTES_PER_UNIT;
        let mut minutes = floor(decimal_minutes) as u8;
        let mut seconds = (decimal_minutes - f64::from(minutes)) * SEXAGESIMAL_MINUTES_PER_UNIT;
        if seconds >= SEXAGESIMAL_MINUTES_PER_UNIT {
            seconds = 0.0;
            minutes += 1;
        }
        if f64::from(minutes) >= SEXAGESIMAL_MINUTES_PER_UNIT {
            minutes = 0;
            hours += 1;
        }
        Self {
            hours,
            minutes,
            seconds: if seconds == 0.0 { 0.0 } else { seconds },
        }
    }

    fn validate_subunit(field: &'static str, value: u8) -> Result<(), Error> {
        if f64::from(value) < SEXAGESIMAL_MINUTES_PER_UNIT {
            Ok(())
        } else {
            Err(Error::OutOfRange {
                field,
                value: f64::from(value),
                interval: "[0, 60)",
                unit: "min",
            })
        }
    }

    const fn syntax_error() -> Error {
        Error::InvalidSexagesimalSyntax {
            format: HOURS_MINUTES_SECONDS_FORMAT,
        }
    }
}

impl FromStr for HoursMinutesSeconds {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

impl fmt::Display for HoursMinutesSeconds {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(precision) = formatter.precision() {
            let (hours, minutes, seconds) = SexagesimalFormatter::rounded_hms(*self, precision);
            write!(formatter, "{hours:02}h{minutes:02}m")?;
            SexagesimalFormatter::write_seconds(formatter, seconds, Some(precision))?;
            formatter.write_str("s")
        } else {
            write!(formatter, "{:02}h{:02}m", self.hours, self.minutes)?;
            SexagesimalFormatter::write_seconds(formatter, self.seconds, None)?;
            formatter.write_str("s")
        }
    }
}

/// A signed angle represented as degrees, arcminutes, and arcseconds.
///
/// The explicit sign preserves negative zero. Semantic angle types impose their
/// own degree limits when converting from this representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DegreesMinutesSeconds {
    sign: SexagesimalSign,
    degrees: u16,
    minutes: u8,
    seconds: f64,
}

impl DegreesMinutesSeconds {
    /// Constructs a DMS representation with canonical minute and second fields.
    pub fn new(
        sign: SexagesimalSign,
        degrees: u16,
        minutes: u8,
        seconds: f64,
    ) -> Result<Self, Error> {
        HoursMinutesSeconds::validate_subunit("sexagesimal arcminutes", minutes)?;
        Error::ensure_finite("sexagesimal arcseconds", seconds)?;
        if !(0.0..SEXAGESIMAL_MINUTES_PER_UNIT).contains(&seconds) {
            return Err(Error::OutOfRange {
                field: "sexagesimal arcseconds",
                value: seconds,
                interval: "[0, 60)",
                unit: "arcsec",
            });
        }
        Ok(Self {
            sign,
            degrees,
            minutes,
            seconds: if seconds == 0.0 { 0.0 } else { seconds },
        })
    }

    /// Constructs DMS from finite decimal degrees.
    pub fn from_decimal_degrees(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("decimal degrees", value)?;
        let magnitude = value.abs();
        if magnitude >= f64::from(u16::MAX) + 1.0 {
            return Err(Error::OutOfRange {
                field: "decimal degrees magnitude",
                value: magnitude,
                interval: "[0, 65536)",
                unit: "deg",
            });
        }
        let sign = if value.is_sign_negative() {
            SexagesimalSign::Negative
        } else {
            SexagesimalSign::Positive
        };
        Ok(Self::from_valid_decimal_degrees(sign, magnitude))
    }

    /// Parses colon-separated, unit-suffixed, or whitespace-separated DMS.
    ///
    /// Accepted examples include `-12:34:56.7`, `-12°34′56.7″`,
    /// `-12d34m56.7s`, and `-12 34 56.7`. Both `-` and Unicode `−` are
    /// accepted, and an omitted sign is positive.
    pub fn parse(input: &str) -> Result<Self, Error> {
        let input = input.trim();
        let (sign, unsigned) = if let Some(rest) = input.strip_prefix('-') {
            (SexagesimalSign::Negative, rest)
        } else if let Some(rest) = input.strip_prefix('−') {
            (SexagesimalSign::Negative, rest)
        } else if let Some(rest) = input.strip_prefix('+') {
            (SexagesimalSign::Positive, rest)
        } else {
            (SexagesimalSign::Positive, input)
        };
        if unsigned.is_empty() {
            return Err(Self::syntax_error());
        }
        let (degrees, minutes, seconds) = SexagesimalParser::dms_components(unsigned)?;
        Self::new(sign, degrees, minutes, seconds)
    }

    /// Returns the explicit sign.
    pub const fn sign(self) -> SexagesimalSign {
        self.sign
    }

    /// Returns the whole degree magnitude.
    pub const fn degrees(self) -> u16 {
        self.degrees
    }

    /// Returns the whole arcminutes in `0..=59`.
    pub const fn minutes(self) -> u8 {
        self.minutes
    }

    /// Returns the fractional arcseconds in `[0, 60)`.
    pub const fn seconds(self) -> f64 {
        self.seconds
    }

    /// Returns the represented value as signed decimal degrees.
    pub fn as_decimal_degrees(self) -> f64 {
        let magnitude = f64::from(self.degrees)
            + f64::from(self.minutes) / SEXAGESIMAL_MINUTES_PER_UNIT
            + self.seconds / SEXAGESIMAL_SECONDS_PER_UNIT;
        if self.sign.is_negative() {
            -magnitude
        } else {
            magnitude
        }
    }

    pub(crate) fn from_valid_decimal_degrees(sign: SexagesimalSign, magnitude: f64) -> Self {
        let mut degrees = floor(magnitude) as u16;
        let fractional_degrees = magnitude - f64::from(degrees);
        let decimal_minutes = fractional_degrees * SEXAGESIMAL_MINUTES_PER_UNIT;
        let mut minutes = floor(decimal_minutes) as u8;
        let mut seconds = (decimal_minutes - f64::from(minutes)) * SEXAGESIMAL_MINUTES_PER_UNIT;
        if seconds >= SEXAGESIMAL_MINUTES_PER_UNIT {
            seconds = 0.0;
            minutes += 1;
        }
        if f64::from(minutes) >= SEXAGESIMAL_MINUTES_PER_UNIT {
            minutes = 0;
            degrees = degrees.saturating_add(1);
        }
        Self {
            sign,
            degrees,
            minutes,
            seconds: if seconds == 0.0 { 0.0 } else { seconds },
        }
    }

    pub(crate) fn from_semantic_decimal_degrees(value: f64) -> Self {
        let sign = if value.is_sign_negative() {
            SexagesimalSign::Negative
        } else {
            SexagesimalSign::Positive
        };
        Self::from_valid_decimal_degrees(sign, value.abs())
    }

    const fn syntax_error() -> Error {
        Error::InvalidSexagesimalSyntax {
            format: DEGREES_MINUTES_SECONDS_FORMAT,
        }
    }
}

impl FromStr for DegreesMinutesSeconds {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

impl fmt::Display for DegreesMinutesSeconds {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (degrees, minutes, seconds, precision) = if let Some(precision) = formatter.precision()
        {
            let (degrees, minutes, seconds) = SexagesimalFormatter::rounded_dms(*self, precision);
            (degrees, minutes, seconds, Some(precision))
        } else {
            (u32::from(self.degrees), self.minutes, self.seconds, None)
        };
        write!(
            formatter,
            "{}{degrees:02}°{minutes:02}′",
            self.sign.character()
        )?;
        SexagesimalFormatter::write_seconds(formatter, seconds, precision)?;
        formatter.write_str("″")
    }
}

struct SexagesimalParser;

impl SexagesimalParser {
    fn hms_components(input: &str) -> Result<(u8, u8, f64), Error> {
        let components = if input.contains(':') {
            Self::split_three(input, ':', HoursMinutesSeconds::syntax_error())?
        } else if let Some((hours, remainder)) = Self::split_at_any(input, &['h', 'H']) {
            let (minutes, seconds) = Self::split_at_any(remainder, &['m', 'M'])
                .ok_or_else(HoursMinutesSeconds::syntax_error)?;
            let seconds = Self::strip_any_suffix(seconds, &['s', 'S'])
                .ok_or_else(HoursMinutesSeconds::syntax_error)?;
            (hours.trim(), minutes.trim(), seconds.trim())
        } else {
            Self::split_whitespace_three(input, HoursMinutesSeconds::syntax_error())?
        };
        Ok((
            components
                .0
                .parse()
                .map_err(|_| HoursMinutesSeconds::syntax_error())?,
            components
                .1
                .parse()
                .map_err(|_| HoursMinutesSeconds::syntax_error())?,
            components
                .2
                .parse()
                .map_err(|_| HoursMinutesSeconds::syntax_error())?,
        ))
    }

    fn dms_components(input: &str) -> Result<(u16, u8, f64), Error> {
        let components = if input.contains(':') {
            Self::split_three(input, ':', DegreesMinutesSeconds::syntax_error())?
        } else if let Some((degrees, remainder)) = Self::split_at_any(input, &['°', 'd', 'D']) {
            let (minutes, seconds) = Self::split_at_any(remainder, &['′', '\'', 'm', 'M'])
                .ok_or_else(DegreesMinutesSeconds::syntax_error)?;
            let seconds = Self::strip_any_suffix(seconds, &['″', '"', 's', 'S'])
                .ok_or_else(DegreesMinutesSeconds::syntax_error)?;
            (degrees.trim(), minutes.trim(), seconds.trim())
        } else {
            Self::split_whitespace_three(input, DegreesMinutesSeconds::syntax_error())?
        };
        Ok((
            components
                .0
                .parse()
                .map_err(|_| DegreesMinutesSeconds::syntax_error())?,
            components
                .1
                .parse()
                .map_err(|_| DegreesMinutesSeconds::syntax_error())?,
            components
                .2
                .parse()
                .map_err(|_| DegreesMinutesSeconds::syntax_error())?,
        ))
    }

    fn split_three(
        input: &str,
        delimiter: char,
        error: Error,
    ) -> Result<(&str, &str, &str), Error> {
        let mut parts = input.split(delimiter);
        let first = parts.next().ok_or(error)?;
        let second = parts.next().ok_or(error)?;
        let third = parts.next().ok_or(error)?;
        if parts.next().is_some() || first.is_empty() || second.is_empty() || third.is_empty() {
            return Err(error);
        }
        Ok((first.trim(), second.trim(), third.trim()))
    }

    fn split_whitespace_three(input: &str, error: Error) -> Result<(&str, &str, &str), Error> {
        let mut parts = input.split_whitespace();
        let first = parts.next().ok_or(error)?;
        let second = parts.next().ok_or(error)?;
        let third = parts.next().ok_or(error)?;
        if parts.next().is_some() {
            return Err(error);
        }
        Ok((first, second, third))
    }

    fn split_at_any<'input>(
        input: &'input str,
        delimiters: &[char],
    ) -> Option<(&'input str, &'input str)> {
        let (index, delimiter) = input
            .char_indices()
            .find(|(_, candidate)| delimiters.contains(candidate))?;
        let remainder = &input[index + delimiter.len_utf8()..];
        Some((&input[..index], remainder))
    }

    fn strip_any_suffix<'input>(input: &'input str, suffixes: &[char]) -> Option<&'input str> {
        let input = input.trim();
        let suffix = input.chars().next_back()?;
        if suffixes.contains(&suffix) {
            Some(input[..input.len() - suffix.len_utf8()].trim())
        } else {
            None
        }
    }
}

struct SexagesimalFormatter;

impl SexagesimalFormatter {
    fn rounded_hms(value: HoursMinutesSeconds, precision: usize) -> (u8, u8, f64) {
        let (mut hours, mut minutes, mut seconds) = (
            value.hours,
            value.minutes,
            Self::round_seconds(value.seconds, precision),
        );
        if seconds >= SEXAGESIMAL_MINUTES_PER_UNIT {
            seconds = 0.0;
            minutes += 1;
        }
        if f64::from(minutes) >= SEXAGESIMAL_MINUTES_PER_UNIT {
            minutes = 0;
            hours += 1;
        }
        if f64::from(hours) >= HOURS_PER_TURN {
            hours = 0;
        }
        (hours, minutes, seconds)
    }

    fn rounded_dms(value: DegreesMinutesSeconds, precision: usize) -> (u32, u8, f64) {
        let (mut degrees, mut minutes, mut seconds) = (
            u32::from(value.degrees),
            value.minutes,
            Self::round_seconds(value.seconds, precision),
        );
        if seconds >= SEXAGESIMAL_MINUTES_PER_UNIT {
            seconds = 0.0;
            minutes += 1;
        }
        if f64::from(minutes) >= SEXAGESIMAL_MINUTES_PER_UNIT {
            minutes = 0;
            degrees += 1;
        }
        (degrees, minutes, seconds)
    }

    fn round_seconds(seconds: f64, precision: usize) -> f64 {
        let mut factor = 1.0;
        for _ in 0..precision.min(15) {
            factor *= 10.0;
        }
        round(seconds * factor) / factor
    }

    fn write_seconds(
        formatter: &mut fmt::Formatter<'_>,
        seconds: f64,
        precision: Option<usize>,
    ) -> fmt::Result {
        if let Some(precision) = precision {
            let width = precision.saturating_add(3);
            write!(
                formatter,
                "{seconds:0width$.precision$}",
                width = width,
                precision = precision
            )
        } else if seconds < 10.0 {
            write!(formatter, "0{seconds}")
        } else {
            write!(formatter, "{seconds}")
        }
    }
}
