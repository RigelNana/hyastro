use crate::uncertainty::{StandardUncertainty, UncertaintyOrigin};

use super::{Duration, Error, Instant, LeapSeconds, Tai, TimeScale, Ut1MinusUtc, Utc};

/// One `UT1−UTC` observation at an exact UTC-tagged physical instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarthRotationSample {
    epoch: Instant<Utc>,
    ut1_minus_utc: Ut1MinusUtc,
    ut1_minus_utc_standard_uncertainty: Option<StandardUncertainty<Duration>>,
}

impl EarthRotationSample {
    /// Constructs one Earth-rotation sample from validated semantic values.
    pub const fn new(epoch: Instant<Utc>, ut1_minus_utc: Ut1MinusUtc) -> Self {
        Self {
            epoch,
            ut1_minus_utc,
            ut1_minus_utc_standard_uncertainty: None,
        }
    }
    /// Associates a source-reported `UT1−UTC` standard uncertainty.
    #[must_use]
    pub const fn with_standard_uncertainty(
        mut self,
        standard_uncertainty: StandardUncertainty<Duration>,
    ) -> Self {
        self.ut1_minus_utc_standard_uncertainty = Some(standard_uncertainty);
        self
    }

    /// Returns the sample epoch.
    pub const fn epoch(self) -> Instant<Utc> {
        self.epoch
    }

    /// Returns the observed `UT1−UTC` value.
    pub const fn ut1_minus_utc(self) -> Ut1MinusUtc {
        self.ut1_minus_utc
    }
    /// Returns the source-reported `UT1−UTC` standard uncertainty, when supplied.
    pub const fn standard_uncertainty(self) -> Option<StandardUncertainty<Duration>> {
        self.ut1_minus_utc_standard_uncertainty
    }
}

/// Validated Earth-rotation observations used to interpolate `UT1−UTC`.
///
/// Unlike [`EarthOrientationTable`](super::EarthOrientationTable), this table
/// deliberately requires no polar motion, celestial-pole correction, or
/// length-of-day value. It is sufficient for UT1, ERA, and sidereal-time
/// calculations, but cannot drive terrestrial frame or state transforms.
#[derive(Debug, Clone, Copy)]
pub struct EarthRotationTable<'a> {
    samples: &'a [EarthRotationSample],
    version: &'a str,
    expires: Instant<Utc>,
}

impl<'a> EarthRotationTable<'a> {
    /// Validates a non-empty, strictly ordered Earth-rotation table.
    ///
    /// `expires` is the exclusive metadata expiration instant and must follow
    /// the final sample. Numerical coverage remains the closed interval from
    /// the first through the last sample; the table never extrapolates.
    pub fn new(
        samples: &'a [EarthRotationSample],
        version: &'a str,
        expires: Instant<Utc>,
    ) -> Result<Self, Error> {
        if samples.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "at least one Earth-rotation sample is required",
            });
        }
        if version.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "Earth-rotation version must not be empty",
            });
        }
        for (index, pair) in samples.windows(2).enumerate() {
            if pair[0].epoch.tai_nanoseconds_since_1900()
                >= pair[1].epoch.tai_nanoseconds_since_1900()
            {
                return Err(Error::InvalidEarthOrientationSample {
                    index: index + 1,
                    reason: "Earth-rotation sample epochs must be strictly increasing",
                });
            }
        }
        let last = samples[samples.len() - 1]
            .epoch
            .tai_nanoseconds_since_1900();
        if expires.tai_nanoseconds_since_1900() <= last {
            return Err(Error::InvalidEarthOrientationData {
                reason: "Earth-rotation expiration must follow the final sample",
            });
        }
        Ok(Self {
            samples,
            version,
            expires,
        })
    }

    /// Returns the original validated samples.
    pub const fn samples(self) -> &'a [EarthRotationSample] {
        self.samples
    }

    /// Returns the provider's version identifier.
    pub const fn version(self) -> &'a str {
        self.version
    }

    /// Returns the closed physical interval covered by interpolation.
    pub fn coverage(self) -> (Instant<Utc>, Instant<Utc>) {
        (
            self.samples[0].epoch,
            self.samples[self.samples.len() - 1].epoch,
        )
    }

    /// Returns the exclusive metadata expiration instant.
    pub const fn expires(self) -> Instant<Utc> {
        self.expires
    }

    pub(crate) fn at<S: TimeScale>(
        self,
        epoch: Instant<S>,
        leap_seconds: LeapSeconds<'_>,
    ) -> Result<EarthRotation<S>, Error> {
        let requested = epoch.tai_nanoseconds_since_1900();
        let first = self.samples[0].epoch.tai_nanoseconds_since_1900();
        let last = self.samples[self.samples.len() - 1]
            .epoch
            .tai_nanoseconds_since_1900();
        let expires = self.expires.tai_nanoseconds_since_1900();
        if requested >= expires {
            return Err(Error::EarthOrientationExpired { requested, expires });
        }
        if requested < first || requested > last {
            return Err(Error::EarthOrientationUnavailable {
                requested,
                coverage_start: first,
                coverage_end: last,
            });
        }

        if self.samples.len() == 1 {
            return Ok(EarthRotation {
                epoch,
                ut1_minus_utc: self.samples[0].ut1_minus_utc,
                ut1_minus_utc_standard_uncertainty: self.samples[0]
                    .ut1_minus_utc_standard_uncertainty,
                standard_uncertainty_origin: self.samples[0]
                    .ut1_minus_utc_standard_uncertainty
                    .map(|_| UncertaintyOrigin::SourceReported),
            });
        }

        let (left_index, right_index) =
            match self.samples.binary_search_by_key(&requested, |sample| {
                sample.epoch.tai_nanoseconds_since_1900()
            }) {
                Ok(index) if index + 1 < self.samples.len() => (index, index + 1),
                Ok(index) => (index - 1, index),
                Err(right_index) => (right_index - 1, right_index),
            };
        let left = self.samples[left_index];
        let right = self.samples[right_index];
        let left_epoch = left.epoch.tai_nanoseconds_since_1900();
        let right_epoch = right.epoch.tai_nanoseconds_since_1900();
        let elapsed = requested.checked_sub(left_epoch).ok_or(Error::Overflow {
            operation: "interpolating Earth-rotation epoch",
        })?;
        let span = right_epoch.checked_sub(left_epoch).ok_or(Error::Overflow {
            operation: "interpolating Earth-rotation span",
        })?;
        let fraction = elapsed as f64 / span as f64;
        Error::ensure_finite("Earth-rotation interpolation fraction", fraction)?;

        let query_tai_minus_utc = leap_seconds.offset(epoch.retag::<Tai>())?;
        let left_tai_minus_utc = leap_seconds.offset(left.epoch.retag::<Tai>())?;
        let right_tai_minus_utc = leap_seconds.offset(right.epoch.retag::<Tai>())?;
        let left_ut1_minus_tai = left
            .ut1_minus_utc
            .as_duration()
            .checked_sub(left_tai_minus_utc)?;
        let right_ut1_minus_tai = right
            .ut1_minus_utc
            .as_duration()
            .checked_sub(right_tai_minus_utc)?;
        let interpolated = (1.0 - fraction) * left_ut1_minus_tai.as_nanoseconds() as f64
            + fraction * right_ut1_minus_tai.as_nanoseconds() as f64;
        Error::ensure_finite("interpolated UT1−TAI nanoseconds", interpolated)?;
        let ut1_minus_tai = Duration::from_nanoseconds(libm::round(interpolated) as i128);
        let ut1_minus_utc =
            Ut1MinusUtc::from_duration(ut1_minus_tai.checked_add(query_tai_minus_utc)?)?;
        let (ut1_minus_utc_standard_uncertainty, standard_uncertainty_origin) =
            Self::interpolate_standard_uncertainty(
                left.ut1_minus_utc_standard_uncertainty,
                right.ut1_minus_utc_standard_uncertainty,
                fraction,
            )?;

        Ok(EarthRotation {
            epoch,
            ut1_minus_utc,
            ut1_minus_utc_standard_uncertainty,
            standard_uncertainty_origin,
        })
    }

    fn interpolate_standard_uncertainty(
        left: Option<StandardUncertainty<Duration>>,
        right: Option<StandardUncertainty<Duration>>,
        fraction: f64,
    ) -> Result<
        (
            Option<StandardUncertainty<Duration>>,
            Option<UncertaintyOrigin>,
        ),
        Error,
    > {
        if fraction == 0.0 {
            return Ok((left, left.map(|_| UncertaintyOrigin::SourceReported)));
        }
        if fraction == 1.0 {
            return Ok((right, right.map(|_| UncertaintyOrigin::SourceReported)));
        }
        let (Some(left), Some(right)) = (left, right) else {
            return Ok((None, None));
        };
        let seconds = (1.0 - fraction) * left.value().as_seconds_f64()
            + fraction * right.value().as_seconds_f64();
        let duration = Duration::from_seconds_f64(seconds)?;
        Ok((
            Some(StandardUncertainty::from_validated(duration)),
            Some(UncertaintyOrigin::CorrelationAgnosticLinearInterpolation),
        ))
    }
}

/// Earth-rotation values resolved at one physical instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarthRotation<S: TimeScale> {
    epoch: Instant<S>,
    ut1_minus_utc: Ut1MinusUtc,
    ut1_minus_utc_standard_uncertainty: Option<StandardUncertainty<Duration>>,
    standard_uncertainty_origin: Option<UncertaintyOrigin>,
}

impl<S: TimeScale> EarthRotation<S> {
    /// Returns the resolved physical epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns interpolated `UT1−UTC`.
    pub const fn ut1_minus_utc(self) -> Ut1MinusUtc {
        self.ut1_minus_utc
    }

    /// Returns the `UT1−UTC` standard uncertainty, when source errors support it.
    ///
    /// An exact sample preserves the reported value. Between samples this is a
    /// correlation-agnostic linear upper bound; it excludes EOP interpolation
    /// error and model discrepancy.
    pub const fn standard_uncertainty(self) -> Option<StandardUncertainty<Duration>> {
        self.ut1_minus_utc_standard_uncertainty
    }

    /// Returns how the available standard uncertainty was obtained.
    pub const fn standard_uncertainty_origin(self) -> Option<UncertaintyOrigin> {
        self.standard_uncertainty_origin
    }
}
