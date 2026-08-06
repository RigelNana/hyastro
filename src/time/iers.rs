use core::f64::consts::PI;
use std::{str::SplitWhitespace, vec::Vec};

use crate::math::{Angle, AngularSpeed};

use super::{
    CelestialPoleOffsetX, CelestialPoleOffsetY, DateTime, Duration, EarthOrientationSample, Error,
    ExcessLengthOfDay, Gregorian, ModifiedJulianDate, PolarMotionX, PolarMotionY, TimeContext,
    Ut1MinusUtc, Utc,
};

const RADIANS_PER_ARCSECOND: f64 = PI / (180.0 * 3_600.0);
const SECONDS_PER_DAY: f64 = 86_400.0;

/// An IERS Earth-orientation product understood by the built-in text adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EarthOrientationProduct {
    /// IERS EOP 20u24 C04 with IAU 2000A celestial-pole offsets.
    IersC04,
    /// IERS `finals.all` using the IAU 2000A columns.
    IersFinals2000A,
}

/// Provenance class attached to one group of EOP values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EarthOrientationValueKind {
    /// A final value from the IERS C04 or Bulletin B series.
    Final,
    /// A measured rapid-service value marked `I` by Bulletin A.
    Observed,
    /// A predicted rapid-service value marked `P` by Bulletin A.
    Predicted,
}

/// Per-group provenance for a normalized EOP record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarthOrientationQuality {
    polar_motion: Option<EarthOrientationValueKind>,
    ut1: Option<EarthOrientationValueKind>,
    length_of_day: Option<EarthOrientationValueKind>,
    celestial_pole: Option<EarthOrientationValueKind>,
}

impl EarthOrientationQuality {
    /// Returns the provenance of `xp/yp`, when those values are present.
    pub const fn polar_motion(self) -> Option<EarthOrientationValueKind> {
        self.polar_motion
    }

    /// Returns the provenance of `UT1−UTC`, when present.
    pub const fn ut1(self) -> Option<EarthOrientationValueKind> {
        self.ut1
    }

    /// Returns the provenance of LOD, when present.
    pub const fn length_of_day(self) -> Option<EarthOrientationValueKind> {
        self.length_of_day
    }

    /// Returns the provenance of `dX/dY`, when those values are present.
    pub const fn celestial_pole(self) -> Option<EarthOrientationValueKind> {
        self.celestial_pole
    }
}

/// Source uncertainties associated with a normalized EOP record.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EarthOrientationUncertainty {
    polar_motion_x: Option<Angle>,
    polar_motion_y: Option<Angle>,
    ut1_minus_utc: Option<Duration>,
    excess_length_of_day: Option<Duration>,
    celestial_pole_offset_x: Option<Angle>,
    celestial_pole_offset_y: Option<Angle>,
    polar_motion_rate_x: Option<AngularSpeed>,
    polar_motion_rate_y: Option<AngularSpeed>,
}

impl EarthOrientationUncertainty {
    /// Returns the nonnegative `xp` uncertainty.
    pub const fn polar_motion_x(self) -> Option<Angle> {
        self.polar_motion_x
    }

    /// Returns the nonnegative `yp` uncertainty.
    pub const fn polar_motion_y(self) -> Option<Angle> {
        self.polar_motion_y
    }

    /// Returns the nonnegative `UT1−UTC` uncertainty.
    pub const fn ut1_minus_utc(self) -> Option<Duration> {
        self.ut1_minus_utc
    }

    /// Returns the nonnegative LOD uncertainty.
    pub const fn excess_length_of_day(self) -> Option<Duration> {
        self.excess_length_of_day
    }

    /// Returns the nonnegative `dX` uncertainty.
    pub const fn celestial_pole_offset_x(self) -> Option<Angle> {
        self.celestial_pole_offset_x
    }

    /// Returns the nonnegative `dY` uncertainty.
    pub const fn celestial_pole_offset_y(self) -> Option<Angle> {
        self.celestial_pole_offset_y
    }

    /// Returns the nonnegative `xp` rate uncertainty.
    pub const fn polar_motion_rate_x(self) -> Option<AngularSpeed> {
        self.polar_motion_rate_x
    }

    /// Returns the nonnegative `yp` rate uncertainty.
    pub const fn polar_motion_rate_y(self) -> Option<AngularSpeed> {
        self.polar_motion_rate_y
    }
}

/// One normalized row from an IERS Earth-orientation product.
///
/// Optional fields remain absent when the upstream fixed-width column is
/// blank. In particular, this type never converts a missing prediction to a
/// numeric zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthOrientationRecord {
    source_line: usize,
    datetime: DateTime<Gregorian, Utc>,
    modified_julian_date: ModifiedJulianDate<Utc>,
    ut1_minus_utc: Option<Ut1MinusUtc>,
    excess_length_of_day: Option<ExcessLengthOfDay>,
    polar_motion_x: Option<PolarMotionX>,
    polar_motion_y: Option<PolarMotionY>,
    celestial_pole_offset_x: Option<CelestialPoleOffsetX>,
    celestial_pole_offset_y: Option<CelestialPoleOffsetY>,
    polar_motion_rate_x: Option<AngularSpeed>,
    polar_motion_rate_y: Option<AngularSpeed>,
    quality: EarthOrientationQuality,
    uncertainty: EarthOrientationUncertainty,
}

impl EarthOrientationRecord {
    /// Returns the one-based line number in the source text.
    pub const fn source_line(self) -> usize {
        self.source_line
    }

    /// Returns the UTC calendar label carried by the row.
    pub const fn datetime(self) -> DateTime<Gregorian, Utc> {
        self.datetime
    }

    /// Returns the UTC Modified Julian Date carried by the row.
    pub const fn modified_julian_date(self) -> ModifiedJulianDate<Utc> {
        self.modified_julian_date
    }

    /// Returns `UT1−UTC`, preserving an empty source field as `None`.
    pub const fn ut1_minus_utc(self) -> Option<Ut1MinusUtc> {
        self.ut1_minus_utc
    }

    /// Returns excess length of day, preserving an empty source field as `None`.
    pub const fn excess_length_of_day(self) -> Option<ExcessLengthOfDay> {
        self.excess_length_of_day
    }

    /// Returns polar motion `xp`, preserving an empty source field as `None`.
    pub const fn polar_motion_x(self) -> Option<PolarMotionX> {
        self.polar_motion_x
    }

    /// Returns polar motion `yp`, preserving an empty source field as `None`.
    pub const fn polar_motion_y(self) -> Option<PolarMotionY> {
        self.polar_motion_y
    }

    /// Returns celestial-pole correction `dX`, preserving an empty source field as `None`.
    pub const fn celestial_pole_offset_x(self) -> Option<CelestialPoleOffsetX> {
        self.celestial_pole_offset_x
    }

    /// Returns celestial-pole correction `dY`, preserving an empty source field as `None`.
    pub const fn celestial_pole_offset_y(self) -> Option<CelestialPoleOffsetY> {
        self.celestial_pole_offset_y
    }

    /// Returns the source-provided polar-motion rate `xrt`, when present.
    pub const fn polar_motion_rate_x(self) -> Option<AngularSpeed> {
        self.polar_motion_rate_x
    }

    /// Returns the source-provided polar-motion rate `yrt`, when present.
    pub const fn polar_motion_rate_y(self) -> Option<AngularSpeed> {
        self.polar_motion_rate_y
    }

    /// Returns per-value-group provenance.
    pub const fn quality(self) -> EarthOrientationQuality {
        self.quality
    }

    /// Returns source uncertainties without synthesizing missing values.
    pub const fn uncertainty(self) -> EarthOrientationUncertainty {
        self.uncertainty
    }

    /// Resolves this row to a complete interpolation sample.
    ///
    /// The method fails if the supplied leap-second context cannot resolve the
    /// UTC label or if any algorithm-required field is absent.
    pub fn try_into_sample<E>(
        self,
        time: &TimeContext<'_, E>,
    ) -> Result<EarthOrientationSample, Error> {
        let epoch = time.resolve(self.datetime)?;
        let epoch_tai_nanoseconds = epoch.tai_nanoseconds_since_1900();
        let mut sample = EarthOrientationSample::new(
            epoch,
            Self::required(self.ut1_minus_utc, "UT1−UTC", epoch_tai_nanoseconds)?,
            Self::required(
                self.excess_length_of_day,
                "length of day",
                epoch_tai_nanoseconds,
            )?,
            Self::required(
                self.polar_motion_x,
                "polar motion xp",
                epoch_tai_nanoseconds,
            )?,
            Self::required(
                self.polar_motion_y,
                "polar motion yp",
                epoch_tai_nanoseconds,
            )?,
            Self::required(
                self.celestial_pole_offset_x,
                "celestial-pole offset dX",
                epoch_tai_nanoseconds,
            )?,
            Self::required(
                self.celestial_pole_offset_y,
                "celestial-pole offset dY",
                epoch_tai_nanoseconds,
            )?,
        );
        match (self.polar_motion_rate_x, self.polar_motion_rate_y) {
            (Some(rate_x), Some(rate_y)) => {
                sample = sample.with_polar_motion_rates(rate_x, rate_y);
            }
            (None, None) => {}
            _ => {
                return Err(Error::InvalidEarthOrientationText {
                    line: self.source_line,
                    field: "paired polar-motion rates",
                });
            }
        }
        Ok(sample)
    }

    fn required<T>(
        value: Option<T>,
        field: &'static str,
        epoch_tai_nanoseconds: i128,
    ) -> Result<T, Error> {
        value.ok_or(Error::MissingEarthOrientationValue {
            field,
            epoch_tai_nanoseconds,
        })
    }
}

/// Owned normalized records parsed from one IERS text product.
#[derive(Debug, Clone, PartialEq)]
pub struct EarthOrientationData {
    product: EarthOrientationProduct,
    records: Vec<EarthOrientationRecord>,
}

impl EarthOrientationData {
    /// Returns the source product represented by this data.
    pub const fn product(&self) -> EarthOrientationProduct {
        self.product
    }

    /// Returns all normalized records in source order.
    pub fn records(&self) -> &[EarthOrientationRecord] {
        &self.records
    }

    /// Resolves every record as a complete EOP interpolation sample.
    ///
    /// This is strict: one missing upstream value fails the conversion rather
    /// than dropping the row or substituting zero.
    pub fn try_samples<E>(
        &self,
        time: &TimeContext<'_, E>,
    ) -> Result<Vec<EarthOrientationSample>, Error> {
        self.records
            .iter()
            .copied()
            .map(|record| record.try_into_sample(time))
            .collect()
    }

    /// Resolves complete samples inside an inclusive typed MJD interval.
    ///
    /// This permits callers to intersect a long EOP product with the explicit
    /// coverage of their leap-second model. Records inside the requested
    /// interval remain strict: a missing value still fails conversion.
    pub fn try_samples_in<E>(
        &self,
        time: &TimeContext<'_, E>,
        start: ModifiedJulianDate<Utc>,
        end: ModifiedJulianDate<Utc>,
    ) -> Result<Vec<EarthOrientationSample>, Error> {
        let start = start.as_f64_lossy();
        let end = end.as_f64_lossy();
        if start > end {
            return Err(Error::InvalidEarthOrientationData {
                reason: "sample MJD interval start must not follow its end",
            });
        }
        let samples = self
            .records
            .iter()
            .copied()
            .filter(|record| {
                let mjd = record.modified_julian_date.as_f64_lossy();
                (start..=end).contains(&mjd)
            })
            .map(|record| record.try_into_sample(time))
            .collect::<Result<Vec<_>, _>>()?;
        if samples.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "sample MJD interval contains no records",
            });
        }
        Ok(samples)
    }
}

/// Parser for the IERS EOP 20u24 C04 whitespace-delimited text product.
#[derive(Debug, Clone, Copy, Default)]
pub struct IersC04;

impl IersC04 {
    /// Parses an entire C04 text snapshot into normalized, fully populated records.
    pub fn parse(source: &str) -> Result<EarthOrientationData, Error> {
        let mut records = Vec::new();
        for (index, line) in source.lines().enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            records.push(Self::parse_line(line, line_number)?);
        }
        if records.is_empty() {
            return Err(IersText::invalid(0, "C04 data rows"));
        }
        Ok(EarthOrientationData {
            product: EarthOrientationProduct::IersC04,
            records,
        })
    }

    fn parse_line(line: &str, line_number: usize) -> Result<EarthOrientationRecord, Error> {
        let mut parts = line.split_whitespace();
        let year = IersText::next_u16(&mut parts, line_number, "year")?;
        let month = IersText::next_u8(&mut parts, line_number, "month")?;
        let day = IersText::next_u8(&mut parts, line_number, "day")?;
        let hour = IersText::next_u8(&mut parts, line_number, "hour")?;
        let mjd = IersText::next_f64(&mut parts, line_number, "MJD")?;
        let xp = IersText::next_f64(&mut parts, line_number, "xp")?;
        let yp = IersText::next_f64(&mut parts, line_number, "yp")?;
        let dut1 = IersText::next_f64(&mut parts, line_number, "UT1−UTC")?;
        let dx = IersText::next_f64(&mut parts, line_number, "dX")?;
        let dy = IersText::next_f64(&mut parts, line_number, "dY")?;
        let xrt = IersText::next_f64(&mut parts, line_number, "xrt")?;
        let yrt = IersText::next_f64(&mut parts, line_number, "yrt")?;
        let lod = IersText::next_f64(&mut parts, line_number, "LOD")?;
        let xp_error = IersText::next_f64(&mut parts, line_number, "xp error")?;
        let yp_error = IersText::next_f64(&mut parts, line_number, "yp error")?;
        let dut1_error = IersText::next_f64(&mut parts, line_number, "UT1−UTC error")?;
        let dx_error = IersText::next_f64(&mut parts, line_number, "dX error")?;
        let dy_error = IersText::next_f64(&mut parts, line_number, "dY error")?;
        let xrt_error = IersText::next_f64(&mut parts, line_number, "xrt error")?;
        let yrt_error = IersText::next_f64(&mut parts, line_number, "yrt error")?;
        let lod_error = IersText::next_f64(&mut parts, line_number, "LOD error")?;
        if parts.next().is_some() {
            return Err(IersText::invalid(line_number, "C04 column count"));
        }

        let (datetime, modified_julian_date) =
            IersText::epoch(year, month, day, hour, mjd, line_number)?;
        let polar_motion_rate_x =
            IersText::angular_speed_arcseconds_per_day(xrt, line_number, "xrt")?;
        let polar_motion_rate_y =
            IersText::angular_speed_arcseconds_per_day(yrt, line_number, "yrt")?;
        Ok(EarthOrientationRecord {
            source_line: line_number,
            datetime,
            modified_julian_date,
            ut1_minus_utc: Some(Ut1MinusUtc::from_seconds(dut1)?),
            excess_length_of_day: Some(ExcessLengthOfDay::from_duration(
                Duration::from_seconds_f64(lod)?,
            )?),
            polar_motion_x: Some(IersText::polar_motion_x(xp, line_number)?),
            polar_motion_y: Some(IersText::polar_motion_y(yp, line_number)?),
            celestial_pole_offset_x: Some(IersText::celestial_offset_x_arcseconds(
                dx,
                line_number,
            )?),
            celestial_pole_offset_y: Some(IersText::celestial_offset_y_arcseconds(
                dy,
                line_number,
            )?),
            polar_motion_rate_x: Some(polar_motion_rate_x),
            polar_motion_rate_y: Some(polar_motion_rate_y),
            quality: EarthOrientationQuality {
                polar_motion: Some(EarthOrientationValueKind::Final),
                ut1: Some(EarthOrientationValueKind::Final),
                length_of_day: Some(EarthOrientationValueKind::Final),
                celestial_pole: Some(EarthOrientationValueKind::Final),
            },
            uncertainty: EarthOrientationUncertainty {
                polar_motion_x: Some(IersText::uncertainty_angle_arcseconds(
                    xp_error,
                    line_number,
                    "xp error",
                )?),
                polar_motion_y: Some(IersText::uncertainty_angle_arcseconds(
                    yp_error,
                    line_number,
                    "yp error",
                )?),
                ut1_minus_utc: Some(IersText::uncertainty_duration_seconds(
                    dut1_error,
                    line_number,
                    "UT1−UTC error",
                )?),
                excess_length_of_day: Some(IersText::uncertainty_duration_seconds(
                    lod_error,
                    line_number,
                    "LOD error",
                )?),
                celestial_pole_offset_x: Some(IersText::uncertainty_angle_arcseconds(
                    dx_error,
                    line_number,
                    "dX error",
                )?),
                celestial_pole_offset_y: Some(IersText::uncertainty_angle_arcseconds(
                    dy_error,
                    line_number,
                    "dY error",
                )?),
                polar_motion_rate_x: Some(IersText::uncertainty_rate_arcseconds_per_day(
                    xrt_error,
                    line_number,
                    "xrt error",
                )?),
                polar_motion_rate_y: Some(IersText::uncertainty_rate_arcseconds_per_day(
                    yrt_error,
                    line_number,
                    "yrt error",
                )?),
            },
        })
    }
}

/// Parser for the IERS `finals.all` IAU 2000A fixed-width product.
#[derive(Debug, Clone, Copy, Default)]
pub struct IersFinals2000A;

impl IersFinals2000A {
    /// Parses `finals.all`, preferring final Bulletin B columns when present.
    ///
    /// Calendar-only trailing rows are ignored. Empty LOD and celestial-pole
    /// fields in otherwise populated prediction rows remain `None`.
    pub fn parse(source: &str) -> Result<EarthOrientationData, Error> {
        let mut records = Vec::new();
        for (index, line) in source.lines().enumerate() {
            let line_number = index + 1;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(record) = Self::parse_line(line, line_number)? {
                records.push(record);
            }
        }
        if records.is_empty() {
            return Err(IersText::invalid(0, "finals2000A data rows"));
        }
        Ok(EarthOrientationData {
            product: EarthOrientationProduct::IersFinals2000A,
            records,
        })
    }

    #[allow(clippy::too_many_lines)]
    fn parse_line(line: &str, line_number: usize) -> Result<Option<EarthOrientationRecord>, Error> {
        let short_year = IersText::required_u8(line, 0, 2, line_number, "year")?;
        let month = IersText::required_u8(line, 2, 4, line_number, "month")?;
        let day = IersText::required_u8(line, 4, 6, line_number, "day")?;
        let mjd = IersText::required_f64(line, 7, 15, line_number, "MJD")?;

        let a_xp = IersText::optional_f64(line, 18, 27, line_number, "Bulletin A xp")?;
        let a_xp_error = IersText::optional_f64(line, 27, 36, line_number, "Bulletin A xp error")?;
        let a_yp = IersText::optional_f64(line, 37, 46, line_number, "Bulletin A yp")?;
        let a_yp_error = IersText::optional_f64(line, 46, 55, line_number, "Bulletin A yp error")?;
        IersText::paired(a_xp, a_yp, line_number, "Bulletin A xp/yp")?;

        let a_dut1 = IersText::optional_f64(line, 58, 68, line_number, "Bulletin A UT1−UTC")?;
        let a_dut1_error =
            IersText::optional_f64(line, 68, 78, line_number, "Bulletin A UT1−UTC error")?;
        let a_lod = IersText::optional_f64(line, 79, 86, line_number, "Bulletin A LOD")?;
        let a_lod_error =
            IersText::optional_f64(line, 86, 93, line_number, "Bulletin A LOD error")?;

        let a_dx = IersText::optional_f64(line, 97, 106, line_number, "Bulletin A dX")?;
        let a_dx_error =
            IersText::optional_f64(line, 106, 115, line_number, "Bulletin A dX error")?;
        let a_dy = IersText::optional_f64(line, 116, 125, line_number, "Bulletin A dY")?;
        let a_dy_error =
            IersText::optional_f64(line, 125, 134, line_number, "Bulletin A dY error")?;
        IersText::paired(a_dx, a_dy, line_number, "Bulletin A dX/dY")?;

        let b_xp = IersText::optional_f64(line, 134, 144, line_number, "Bulletin B xp")?;
        let b_yp = IersText::optional_f64(line, 144, 154, line_number, "Bulletin B yp")?;
        IersText::paired(b_xp, b_yp, line_number, "Bulletin B xp/yp")?;
        let b_dut1 = IersText::optional_f64(line, 154, 165, line_number, "Bulletin B UT1−UTC")?;
        let b_dx = IersText::optional_f64(line, 165, 175, line_number, "Bulletin B dX")?;
        let b_dy = IersText::optional_f64(line, 175, 185, line_number, "Bulletin B dY")?;
        IersText::paired(b_dx, b_dy, line_number, "Bulletin B dX/dY")?;

        if b_xp.is_none()
            && a_xp.is_none()
            && b_dut1.is_none()
            && a_dut1.is_none()
            && a_lod.is_none()
            && b_dx.is_none()
            && a_dx.is_none()
        {
            return Ok(None);
        }

        let year = if mjd <= 51_543.0 {
            1_900 + u16::from(short_year)
        } else {
            2_000 + u16::from(short_year)
        };
        let (datetime, modified_julian_date) =
            IersText::epoch(year, month, day, 0, mjd, line_number)?;

        let pm_a_kind = IersText::flag(line, 16, line_number, "polar-motion flag")?;
        let ut1_a_kind = IersText::flag(line, 57, line_number, "UT1 flag")?;
        let celestial_a_kind = IersText::flag(line, 95, line_number, "nutation flag")?;

        let (xp, yp, polar_motion_kind, xp_error, yp_error) = if let (Some(xp), Some(yp)) =
            (b_xp, b_yp)
        {
            (
                Some(xp),
                Some(yp),
                Some(EarthOrientationValueKind::Final),
                None,
                None,
            )
        } else {
            (
                a_xp,
                a_yp,
                a_xp.map(|_| IersText::required_kind(pm_a_kind, line_number, "polar-motion flag"))
                    .transpose()?,
                a_xp_error,
                a_yp_error,
            )
        };
        let (dut1, ut1_kind, dut1_error) = if let Some(dut1) = b_dut1 {
            (Some(dut1), Some(EarthOrientationValueKind::Final), None)
        } else {
            (
                a_dut1,
                a_dut1
                    .map(|_| IersText::required_kind(ut1_a_kind, line_number, "UT1 flag"))
                    .transpose()?,
                a_dut1_error,
            )
        };
        let (dx, dy, celestial_kind, dx_error, dy_error) =
            if let (Some(dx), Some(dy)) = (b_dx, b_dy) {
                (
                    Some(dx),
                    Some(dy),
                    Some(EarthOrientationValueKind::Final),
                    None,
                    None,
                )
            } else {
                (
                    a_dx,
                    a_dy,
                    a_dx.map(|_| {
                        IersText::required_kind(celestial_a_kind, line_number, "nutation flag")
                    })
                    .transpose()?,
                    a_dx_error,
                    a_dy_error,
                )
            };
        let lod_kind = a_lod
            .map(|_| IersText::required_kind(ut1_a_kind, line_number, "UT1 flag"))
            .transpose()?;

        Ok(Some(EarthOrientationRecord {
            source_line: line_number,
            datetime,
            modified_julian_date,
            ut1_minus_utc: dut1.map(Ut1MinusUtc::from_seconds).transpose()?,
            excess_length_of_day: a_lod
                .map(ExcessLengthOfDay::from_milliseconds)
                .transpose()?,
            polar_motion_x: xp
                .map(|value| IersText::polar_motion_x(value, line_number))
                .transpose()?,
            polar_motion_y: yp
                .map(|value| IersText::polar_motion_y(value, line_number))
                .transpose()?,
            celestial_pole_offset_x: dx
                .map(|value| IersText::celestial_offset_x_milliarcseconds(value, line_number))
                .transpose()?,
            celestial_pole_offset_y: dy
                .map(|value| IersText::celestial_offset_y_milliarcseconds(value, line_number))
                .transpose()?,
            polar_motion_rate_x: None,
            polar_motion_rate_y: None,
            quality: EarthOrientationQuality {
                polar_motion: polar_motion_kind,
                ut1: ut1_kind,
                length_of_day: lod_kind,
                celestial_pole: celestial_kind,
            },
            uncertainty: EarthOrientationUncertainty {
                polar_motion_x: xp_error
                    .map(|value| {
                        IersText::uncertainty_angle_arcseconds(
                            value,
                            line_number,
                            "Bulletin A xp error",
                        )
                    })
                    .transpose()?,
                polar_motion_y: yp_error
                    .map(|value| {
                        IersText::uncertainty_angle_arcseconds(
                            value,
                            line_number,
                            "Bulletin A yp error",
                        )
                    })
                    .transpose()?,
                ut1_minus_utc: dut1_error
                    .map(|value| {
                        IersText::uncertainty_duration_seconds(
                            value,
                            line_number,
                            "Bulletin A UT1−UTC error",
                        )
                    })
                    .transpose()?,
                excess_length_of_day: a_lod_error
                    .map(|milliseconds| {
                        IersText::uncertainty_duration_seconds(
                            milliseconds / 1_000.0,
                            line_number,
                            "Bulletin A LOD error",
                        )
                    })
                    .transpose()?,
                celestial_pole_offset_x: dx_error
                    .map(|milliarcseconds| {
                        IersText::uncertainty_angle_milliarcseconds(
                            milliarcseconds,
                            line_number,
                            "Bulletin A dX error",
                        )
                    })
                    .transpose()?,
                celestial_pole_offset_y: dy_error
                    .map(|milliarcseconds| {
                        IersText::uncertainty_angle_milliarcseconds(
                            milliarcseconds,
                            line_number,
                            "Bulletin A dY error",
                        )
                    })
                    .transpose()?,
                polar_motion_rate_x: None,
                polar_motion_rate_y: None,
            },
        }))
    }
}

struct IersText;

impl IersText {
    const fn invalid(line: usize, field: &'static str) -> Error {
        Error::InvalidEarthOrientationText { line, field }
    }

    fn next<'a>(
        parts: &mut SplitWhitespace<'a>,
        line: usize,
        field: &'static str,
    ) -> Result<&'a str, Error> {
        parts.next().ok_or(Self::invalid(line, field))
    }

    fn next_u8(
        parts: &mut SplitWhitespace<'_>,
        line: usize,
        field: &'static str,
    ) -> Result<u8, Error> {
        Self::next(parts, line, field)?
            .parse()
            .map_err(|_| Self::invalid(line, field))
    }

    fn next_u16(
        parts: &mut SplitWhitespace<'_>,
        line: usize,
        field: &'static str,
    ) -> Result<u16, Error> {
        Self::next(parts, line, field)?
            .parse()
            .map_err(|_| Self::invalid(line, field))
    }

    fn next_f64(
        parts: &mut SplitWhitespace<'_>,
        line: usize,
        field: &'static str,
    ) -> Result<f64, Error> {
        Self::next(parts, line, field)?
            .parse()
            .map_err(|_| Self::invalid(line, field))
    }

    fn field(source: &str, start: usize, end: usize) -> &str {
        source.get(start..end).unwrap_or("").trim()
    }

    fn required_u8(
        source: &str,
        start: usize,
        end: usize,
        line: usize,
        field: &'static str,
    ) -> Result<u8, Error> {
        Self::field(source, start, end)
            .parse()
            .map_err(|_| Self::invalid(line, field))
    }

    fn required_f64(
        source: &str,
        start: usize,
        end: usize,
        line: usize,
        field: &'static str,
    ) -> Result<f64, Error> {
        Self::field(source, start, end)
            .parse()
            .map_err(|_| Self::invalid(line, field))
    }

    fn optional_f64(
        source: &str,
        start: usize,
        end: usize,
        line: usize,
        field: &'static str,
    ) -> Result<Option<f64>, Error> {
        let value = Self::field(source, start, end);
        if value.is_empty() {
            Ok(None)
        } else {
            value
                .parse()
                .map(Some)
                .map_err(|_| Self::invalid(line, field))
        }
    }

    fn flag(
        source: &str,
        index: usize,
        line: usize,
        field: &'static str,
    ) -> Result<Option<EarthOrientationValueKind>, Error> {
        match Self::field(source, index, index + 1) {
            "I" => Ok(Some(EarthOrientationValueKind::Observed)),
            "P" => Ok(Some(EarthOrientationValueKind::Predicted)),
            "" => Ok(None),
            _ => Err(Self::invalid(line, field)),
        }
    }

    fn required_kind(
        value: Option<EarthOrientationValueKind>,
        line: usize,
        field: &'static str,
    ) -> Result<EarthOrientationValueKind, Error> {
        value.ok_or(Self::invalid(line, field))
    }

    fn paired(
        left: Option<f64>,
        right: Option<f64>,
        line: usize,
        field: &'static str,
    ) -> Result<(), Error> {
        if left.is_some() == right.is_some() {
            Ok(())
        } else {
            Err(Self::invalid(line, field))
        }
    }

    fn epoch(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        mjd: f64,
        line: usize,
    ) -> Result<(DateTime<Gregorian, Utc>, ModifiedJulianDate<Utc>), Error> {
        let datetime = DateTime::from_components(year.into(), month, day, hour, 0, 0, 0)?;
        let expected = datetime.date().to_julian_day_number().value() as f64 - 2_400_001.0
            + f64::from(hour) / 24.0;
        if (mjd - expected).abs() > 5.0e-7 {
            return Err(Error::EarthOrientationMjdMismatch {
                line,
                expected,
                actual: mjd,
            });
        }
        Ok((datetime, ModifiedJulianDate::from_parts(mjd, 0.0)?))
    }

    fn polar_motion_x(value: f64, line: usize) -> Result<PolarMotionX, Error> {
        PolarMotionX::from_arcseconds(value).map_err(|_| Self::invalid(line, "xp"))
    }

    fn polar_motion_y(value: f64, line: usize) -> Result<PolarMotionY, Error> {
        PolarMotionY::from_arcseconds(value).map_err(|_| Self::invalid(line, "yp"))
    }

    fn celestial_offset_x_arcseconds(
        value: f64,
        line: usize,
    ) -> Result<CelestialPoleOffsetX, Error> {
        Self::celestial_offset_x_milliarcseconds(value * 1_000.0, line)
    }

    fn celestial_offset_y_arcseconds(
        value: f64,
        line: usize,
    ) -> Result<CelestialPoleOffsetY, Error> {
        Self::celestial_offset_y_milliarcseconds(value * 1_000.0, line)
    }

    fn celestial_offset_x_milliarcseconds(
        value: f64,
        line: usize,
    ) -> Result<CelestialPoleOffsetX, Error> {
        CelestialPoleOffsetX::from_milliarcseconds(value).map_err(|_| Self::invalid(line, "dX"))
    }

    fn celestial_offset_y_milliarcseconds(
        value: f64,
        line: usize,
    ) -> Result<CelestialPoleOffsetY, Error> {
        CelestialPoleOffsetY::from_milliarcseconds(value).map_err(|_| Self::invalid(line, "dY"))
    }

    fn angular_speed_arcseconds_per_day(
        value: f64,
        line: usize,
        field: &'static str,
    ) -> Result<AngularSpeed, Error> {
        AngularSpeed::from_radians_per_second(value * RADIANS_PER_ARCSECOND / SECONDS_PER_DAY)
            .map_err(|_| Self::invalid(line, field))
    }

    fn uncertainty_angle_arcseconds(
        value: f64,
        line: usize,
        field: &'static str,
    ) -> Result<Angle, Error> {
        if value < 0.0 {
            return Err(Self::invalid(line, field));
        }
        Angle::from_radians(value * RADIANS_PER_ARCSECOND).map_err(|_| Self::invalid(line, field))
    }

    fn uncertainty_angle_milliarcseconds(
        value: f64,
        line: usize,
        field: &'static str,
    ) -> Result<Angle, Error> {
        Self::uncertainty_angle_arcseconds(value / 1_000.0, line, field)
    }

    fn uncertainty_duration_seconds(
        value: f64,
        line: usize,
        field: &'static str,
    ) -> Result<Duration, Error> {
        if value < 0.0 {
            return Err(Self::invalid(line, field));
        }
        Duration::from_seconds_f64(value).map_err(|_| Self::invalid(line, field))
    }

    fn uncertainty_rate_arcseconds_per_day(
        value: f64,
        line: usize,
        field: &'static str,
    ) -> Result<AngularSpeed, Error> {
        if value < 0.0 {
            return Err(Self::invalid(line, field));
        }
        Self::angular_speed_arcseconds_per_day(value, line, field)
    }
}
