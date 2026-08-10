use crate::uncertainty::{StandardUncertainty, UncertaintyOrigin};

use crate::math::Angle;

use super::{
    CelestialPoleOffsetX, CelestialPoleOffsetY, DeltaT, Duration, EarthAttitudeModelProvenance,
    EarthOrientationTable, Error, Instant, JulianDate, LeapSeconds, PolarMotionX, PolarMotionY,
    PredictedEarthOrientation, Tai, TimeScale, Tt, Ut1MinusUtc, Utc,
};

/// Source standard uncertainties associated with Earth-attitude values.
///
/// Missing fields stay explicit. The values describe the upstream estimates
/// only; they do not include interpolation error or model discrepancy.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EarthAttitudeStandardUncertainties {
    ut1_minus_utc: Option<StandardUncertainty<Duration>>,
    polar_motion_x: Option<StandardUncertainty<Angle>>,
    polar_motion_y: Option<StandardUncertainty<Angle>>,
    celestial_pole_offset_x: Option<StandardUncertainty<Angle>>,
    celestial_pole_offset_y: Option<StandardUncertainty<Angle>>,
}

impl EarthAttitudeStandardUncertainties {
    /// Constructs a bundle from independently optional source uncertainties.
    pub const fn new(
        ut1_minus_utc: Option<StandardUncertainty<Duration>>,
        polar_motion_x: Option<StandardUncertainty<Angle>>,
        polar_motion_y: Option<StandardUncertainty<Angle>>,
        celestial_pole_offset_x: Option<StandardUncertainty<Angle>>,
        celestial_pole_offset_y: Option<StandardUncertainty<Angle>>,
    ) -> Self {
        Self {
            ut1_minus_utc,
            polar_motion_x,
            polar_motion_y,
            celestial_pole_offset_x,
            celestial_pole_offset_y,
        }
    }

    /// Returns a bundle with no reported uncertainties.
    pub const fn none() -> Self {
        Self::new(None, None, None, None, None)
    }

    /// Returns the `UT1−UTC` standard uncertainty.
    pub const fn ut1_minus_utc(self) -> Option<StandardUncertainty<Duration>> {
        self.ut1_minus_utc
    }

    /// Returns the polar-motion $x_p$ standard uncertainty.
    pub const fn polar_motion_x(self) -> Option<StandardUncertainty<Angle>> {
        self.polar_motion_x
    }

    /// Returns the polar-motion $y_p$ standard uncertainty.
    pub const fn polar_motion_y(self) -> Option<StandardUncertainty<Angle>> {
        self.polar_motion_y
    }

    /// Returns the celestial-pole $dX$ standard uncertainty.
    pub const fn celestial_pole_offset_x(self) -> Option<StandardUncertainty<Angle>> {
        self.celestial_pole_offset_x
    }

    /// Returns the celestial-pole $dY$ standard uncertainty.
    pub const fn celestial_pole_offset_y(self) -> Option<StandardUncertainty<Angle>> {
        self.celestial_pole_offset_y
    }

    /// Returns whether no field carries a standard uncertainty.
    pub const fn is_empty(self) -> bool {
        self.ut1_minus_utc.is_none()
            && self.polar_motion_x.is_none()
            && self.polar_motion_y.is_none()
            && self.celestial_pole_offset_x.is_none()
            && self.celestial_pole_offset_y.is_none()
    }
}

/// One Earth-attitude observation at an exact UTC-tagged physical instant.
///
/// Attitude samples contain every value needed for observed celestial-to-
/// terrestrial direction rotations, but deliberately do not require length of
/// day or angular rates needed by position-velocity state transforms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthAttitudeSample {
    epoch: Instant<Utc>,
    ut1_minus_utc: Ut1MinusUtc,
    polar_motion_x: PolarMotionX,
    polar_motion_y: PolarMotionY,
    celestial_pole_offset_x: CelestialPoleOffsetX,
    celestial_pole_offset_y: CelestialPoleOffsetY,
    standard_uncertainties: EarthAttitudeStandardUncertainties,
}

impl EarthAttitudeSample {
    /// Constructs a sample from validated Earth-attitude values.
    pub const fn new(
        epoch: Instant<Utc>,
        ut1_minus_utc: Ut1MinusUtc,
        polar_motion_x: PolarMotionX,
        polar_motion_y: PolarMotionY,
        celestial_pole_offset_x: CelestialPoleOffsetX,
        celestial_pole_offset_y: CelestialPoleOffsetY,
    ) -> Self {
        Self {
            epoch,
            ut1_minus_utc,
            polar_motion_x,
            polar_motion_y,
            celestial_pole_offset_x,
            standard_uncertainties: EarthAttitudeStandardUncertainties::none(),
            celestial_pole_offset_y,
        }
    }
    /// Associates source-reported standard uncertainties with this sample.
    #[must_use]
    pub const fn with_standard_uncertainties(
        mut self,
        standard_uncertainties: EarthAttitudeStandardUncertainties,
    ) -> Self {
        self.standard_uncertainties = standard_uncertainties;
        self
    }

    /// Returns the sample epoch.
    pub const fn epoch(self) -> Instant<Utc> {
        self.epoch
    }

    /// Returns observed `UT1−UTC`.
    pub const fn ut1_minus_utc(self) -> Ut1MinusUtc {
        self.ut1_minus_utc
    }

    /// Returns polar motion $x_p$.
    pub const fn polar_motion_x(self) -> PolarMotionX {
        self.polar_motion_x
    }

    /// Returns polar motion $y_p$.
    pub const fn polar_motion_y(self) -> PolarMotionY {
        self.polar_motion_y
    }

    /// Returns celestial-pole correction $dX$.
    pub const fn celestial_pole_offset_x(self) -> CelestialPoleOffsetX {
        self.celestial_pole_offset_x
    }

    /// Returns celestial-pole correction $dY$.
    pub const fn celestial_pole_offset_y(self) -> CelestialPoleOffsetY {
        self.celestial_pole_offset_y
    }

    /// Returns the source-reported standard uncertainties.
    pub const fn standard_uncertainties(self) -> EarthAttitudeStandardUncertainties {
        self.standard_uncertainties
    }
}

/// Validated Earth-attitude observations used for direction-frame rotations.
///
/// Unlike [`EarthOrientationTable`](super::EarthOrientationTable), this table
/// does not require length of day or angular rates. It can drive observed
/// GCRS/CIRS/TIRS/ITRS direction rotations, but cannot prove a full state
/// transform with measured frame velocity.
#[derive(Debug, Clone, Copy)]
pub struct EarthAttitudeTable<'a> {
    samples: &'a [EarthAttitudeSample],
    version: &'a str,
    expires: Instant<Utc>,
}

impl<'a> EarthAttitudeTable<'a> {
    /// Validates a non-empty, strictly ordered Earth-attitude table.
    pub fn new(
        samples: &'a [EarthAttitudeSample],
        version: &'a str,
        expires: Instant<Utc>,
    ) -> Result<Self, Error> {
        if samples.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "at least one Earth-attitude sample is required",
            });
        }
        if version.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "Earth-attitude version must not be empty",
            });
        }
        for (index, pair) in samples.windows(2).enumerate() {
            if pair[0].epoch.tai_nanoseconds_since_1900()
                >= pair[1].epoch.tai_nanoseconds_since_1900()
            {
                return Err(Error::InvalidEarthOrientationSample {
                    index: index + 1,
                    reason: "Earth-attitude sample epochs must be strictly increasing",
                });
            }
        }
        let last = samples[samples.len() - 1]
            .epoch
            .tai_nanoseconds_since_1900();
        if expires.tai_nanoseconds_since_1900() <= last {
            return Err(Error::InvalidEarthOrientationData {
                reason: "Earth-attitude expiration must follow the final sample",
            });
        }
        Ok(Self {
            samples,
            version,
            expires,
        })
    }

    /// Returns the original validated samples.
    pub const fn samples(self) -> &'a [EarthAttitudeSample] {
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
    ) -> Result<EarthAttitude<S>, Error> {
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
            return Ok(EarthAttitude::from_sample(epoch, self.samples[0]));
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
            operation: "interpolating Earth-attitude epoch",
        })?;
        let span = right_epoch.checked_sub(left_epoch).ok_or(Error::Overflow {
            operation: "interpolating Earth-attitude span",
        })?;
        let fraction = elapsed as f64 / span as f64;
        Error::ensure_finite("Earth-attitude interpolation fraction", fraction)?;

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

        let (standard_uncertainties, standard_uncertainty_origin) =
            Self::interpolate_standard_uncertainties(
                left.standard_uncertainties,
                right.standard_uncertainties,
                fraction,
            )?;

        Ok(EarthAttitude {
            epoch,
            ut1_minus_utc,
            polar_motion_x: PolarMotionX::from_angle(Self::interpolate_angle(
                "interpolated polar motion x",
                left.polar_motion_x.as_angle(),
                right.polar_motion_x.as_angle(),
                fraction,
            )?),
            polar_motion_y: PolarMotionY::from_angle(Self::interpolate_angle(
                "interpolated polar motion y",
                left.polar_motion_y.as_angle(),
                right.polar_motion_y.as_angle(),
                fraction,
            )?),
            celestial_pole_offset_x: CelestialPoleOffsetX::from_angle(Self::interpolate_angle(
                "interpolated celestial-pole offset x",
                left.celestial_pole_offset_x.as_angle(),
                right.celestial_pole_offset_x.as_angle(),
                fraction,
            )?),
            celestial_pole_offset_y: CelestialPoleOffsetY::from_angle(Self::interpolate_angle(
                "interpolated celestial-pole offset y",
                left.celestial_pole_offset_y.as_angle(),
                right.celestial_pole_offset_y.as_angle(),
                fraction,
            )?),
            standard_uncertainties,
            standard_uncertainty_origin,
        })
    }

    fn interpolate_angle(
        field: &'static str,
        left: Angle,
        right: Angle,
        fraction: f64,
    ) -> Result<Angle, Error> {
        let value = (1.0 - fraction) * left.as_radians() + fraction * right.as_radians();
        Error::ensure_finite(field, value)?;
        Angle::from_radians(value).map_err(|_| Error::NonFinite { field, value })
    }

    fn interpolate_standard_uncertainties(
        left: EarthAttitudeStandardUncertainties,
        right: EarthAttitudeStandardUncertainties,
        fraction: f64,
    ) -> Result<
        (
            EarthAttitudeStandardUncertainties,
            Option<UncertaintyOrigin>,
        ),
        Error,
    > {
        if fraction == 0.0 {
            return Ok((
                left,
                (!left.is_empty()).then_some(UncertaintyOrigin::SourceReported),
            ));
        }
        if fraction == 1.0 {
            return Ok((
                right,
                (!right.is_empty()).then_some(UncertaintyOrigin::SourceReported),
            ));
        }
        let uncertainties = EarthAttitudeStandardUncertainties::new(
            Self::interpolate_duration_uncertainty(
                left.ut1_minus_utc,
                right.ut1_minus_utc,
                fraction,
            )?,
            Self::interpolate_angle_uncertainty(
                left.polar_motion_x,
                right.polar_motion_x,
                fraction,
            )?,
            Self::interpolate_angle_uncertainty(
                left.polar_motion_y,
                right.polar_motion_y,
                fraction,
            )?,
            Self::interpolate_angle_uncertainty(
                left.celestial_pole_offset_x,
                right.celestial_pole_offset_x,
                fraction,
            )?,
            Self::interpolate_angle_uncertainty(
                left.celestial_pole_offset_y,
                right.celestial_pole_offset_y,
                fraction,
            )?,
        );
        let origin = (!uncertainties.is_empty())
            .then_some(UncertaintyOrigin::CorrelationAgnosticLinearInterpolation);
        Ok((uncertainties, origin))
    }

    fn interpolate_duration_uncertainty(
        left: Option<StandardUncertainty<Duration>>,
        right: Option<StandardUncertainty<Duration>>,
        fraction: f64,
    ) -> Result<Option<StandardUncertainty<Duration>>, Error> {
        let (Some(left), Some(right)) = (left, right) else {
            return Ok(None);
        };
        let value = (1.0 - fraction) * left.value().as_seconds_f64()
            + fraction * right.value().as_seconds_f64();
        Ok(Some(StandardUncertainty::from_validated(
            Duration::from_seconds_f64(value)?,
        )))
    }

    fn interpolate_angle_uncertainty(
        left: Option<StandardUncertainty<Angle>>,
        right: Option<StandardUncertainty<Angle>>,
        fraction: f64,
    ) -> Result<Option<StandardUncertainty<Angle>>, Error> {
        let (Some(left), Some(right)) = (left, right) else {
            return Ok(None);
        };
        let value =
            (1.0 - fraction) * left.value().as_radians() + fraction * right.value().as_radians();
        let angle = Angle::from_radians(value).map_err(|_| Error::NonFinite {
            field: "interpolated Earth-attitude standard uncertainty",
            value,
        })?;
        Ok(Some(StandardUncertainty::from_validated(angle)))
    }
}

/// Earth-attitude values resolved at one physical instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthAttitude<S: TimeScale> {
    epoch: Instant<S>,
    ut1_minus_utc: Ut1MinusUtc,
    polar_motion_x: PolarMotionX,
    polar_motion_y: PolarMotionY,
    celestial_pole_offset_x: CelestialPoleOffsetX,
    celestial_pole_offset_y: CelestialPoleOffsetY,
    standard_uncertainties: EarthAttitudeStandardUncertainties,
    standard_uncertainty_origin: Option<UncertaintyOrigin>,
}

impl<S: TimeScale> EarthAttitude<S> {
    fn from_sample(epoch: Instant<S>, sample: EarthAttitudeSample) -> Self {
        Self {
            epoch,
            ut1_minus_utc: sample.ut1_minus_utc,
            polar_motion_x: sample.polar_motion_x,
            polar_motion_y: sample.polar_motion_y,
            celestial_pole_offset_x: sample.celestial_pole_offset_x,
            celestial_pole_offset_y: sample.celestial_pole_offset_y,
            standard_uncertainties: sample.standard_uncertainties,
            standard_uncertainty_origin: (!sample.standard_uncertainties.is_empty())
                .then_some(UncertaintyOrigin::SourceReported),
        }
    }

    /// Returns the resolved physical epoch.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns interpolated `UT1−UTC`.
    pub const fn ut1_minus_utc(self) -> Ut1MinusUtc {
        self.ut1_minus_utc
    }

    /// Returns interpolated polar motion $x_p$.
    pub const fn polar_motion_x(self) -> PolarMotionX {
        self.polar_motion_x
    }

    /// Returns interpolated polar motion $y_p$.
    pub const fn polar_motion_y(self) -> PolarMotionY {
        self.polar_motion_y
    }

    /// Returns interpolated celestial-pole correction $dX$.
    pub const fn celestial_pole_offset_x(self) -> CelestialPoleOffsetX {
        self.celestial_pole_offset_x
    }

    /// Returns interpolated celestial-pole correction $dY$.
    pub const fn celestial_pole_offset_y(self) -> CelestialPoleOffsetY {
        self.celestial_pole_offset_y
    }

    /// Returns available standard uncertainties for the resolved observations.
    ///
    /// Exact sample queries preserve source-reported values. Between samples
    /// each available field is a correlation-agnostic linear upper bound. The
    /// bundle excludes EOP interpolation error and model discrepancy.
    pub const fn standard_uncertainties(self) -> EarthAttitudeStandardUncertainties {
        self.standard_uncertainties
    }

    /// Returns how the available standard uncertainties were obtained.
    pub const fn standard_uncertainty_origin(self) -> Option<UncertaintyOrigin> {
        self.standard_uncertainty_origin
    }
}

/// Earth-attitude quantities resolved from either tabulated data or an explicit model.
///
/// Delta T is always available and is sufficient to derive UT1 from TT. `UT1−UTC`
/// is present only when the source is a UTC-tagged table with a valid leap-second
/// mapping at the requested instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthAttitudeState<S: TimeScale> {
    epoch: Instant<S>,
    delta_t: DeltaT<S>,
    delta_t_standard_uncertainty: Option<StandardUncertainty<Duration>>,
    ut1_minus_utc: Option<Ut1MinusUtc>,
    polar_motion_x: PolarMotionX,
    polar_motion_y: PolarMotionY,
    celestial_pole_offset_x: CelestialPoleOffsetX,
    celestial_pole_offset_y: CelestialPoleOffsetY,
    standard_uncertainties: EarthAttitudeStandardUncertainties,
}

impl<S: TimeScale> EarthAttitudeState<S> {
    fn from_attitude(
        attitude: EarthAttitude<S>,
        leap_seconds: LeapSeconds<'_>,
    ) -> Result<Self, Error> {
        Ok(Self {
            epoch: attitude.epoch(),
            delta_t: DeltaT::from_ut1_minus_utc(
                attitude.epoch(),
                attitude.ut1_minus_utc(),
                leap_seconds,
            )?,
            delta_t_standard_uncertainty: attitude.standard_uncertainties().ut1_minus_utc(),
            ut1_minus_utc: Some(attitude.ut1_minus_utc()),
            polar_motion_x: attitude.polar_motion_x(),
            polar_motion_y: attitude.polar_motion_y(),
            celestial_pole_offset_x: attitude.celestial_pole_offset_x(),
            celestial_pole_offset_y: attitude.celestial_pole_offset_y(),
            standard_uncertainties: attitude.standard_uncertainties(),
        })
    }

    fn from_orientation(
        orientation: super::EarthOrientation<S>,
        leap_seconds: LeapSeconds<'_>,
    ) -> Result<Self, Error> {
        Ok(Self {
            epoch: orientation.epoch(),
            delta_t: DeltaT::from_ut1_minus_utc(
                orientation.epoch(),
                orientation.ut1_minus_utc(),
                leap_seconds,
            )?,
            delta_t_standard_uncertainty: None,
            ut1_minus_utc: Some(orientation.ut1_minus_utc()),
            polar_motion_x: orientation.polar_motion_x(),
            polar_motion_y: orientation.polar_motion_y(),
            celestial_pole_offset_x: orientation.celestial_pole_offset_x(),
            celestial_pole_offset_y: orientation.celestial_pole_offset_y(),
            standard_uncertainties: EarthAttitudeStandardUncertainties::new(
                None, None, None, None, None,
            ),
        })
    }

    fn from_prediction(
        prediction: PredictedEarthOrientation<'_>,
        epoch: Instant<S>,
        terrestrial_time: JulianDate<Tt>,
    ) -> Result<Self, Error> {
        let (delta_t, delta_t_standard_uncertainty) =
            prediction.delta_t_at(epoch, terrestrial_time)?;
        let offsets = prediction.offset_model();
        Ok(Self {
            epoch,
            delta_t,
            delta_t_standard_uncertainty,
            ut1_minus_utc: None,
            polar_motion_x: offsets.polar_motion_x(),
            polar_motion_y: offsets.polar_motion_y(),
            celestial_pole_offset_x: offsets.celestial_pole_offset_x(),
            celestial_pole_offset_y: offsets.celestial_pole_offset_y(),
            standard_uncertainties: prediction.standard_uncertainties(),
        })
    }

    /// Returns the physical epoch shared by every quantity.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns `TT−UT1` at the resolved epoch.
    pub const fn delta_t(self) -> DeltaT<S> {
        self.delta_t
    }

    /// Returns the model-supplied Delta T standard uncertainty, when available.
    pub const fn delta_t_standard_uncertainty(self) -> Option<StandardUncertainty<Duration>> {
        self.delta_t_standard_uncertainty
    }

    /// Returns `UT1−UTC` only for a UTC-tagged tabulated source.
    pub const fn ut1_minus_utc(self) -> Option<Ut1MinusUtc> {
        self.ut1_minus_utc
    }

    /// Returns polar motion $x_p$.
    pub const fn polar_motion_x(self) -> PolarMotionX {
        self.polar_motion_x
    }

    /// Returns polar motion $y_p$.
    pub const fn polar_motion_y(self) -> PolarMotionY {
        self.polar_motion_y
    }

    /// Returns celestial-pole correction $dX$.
    pub const fn celestial_pole_offset_x(self) -> CelestialPoleOffsetX {
        self.celestial_pole_offset_x
    }

    /// Returns celestial-pole correction $dY$.
    pub const fn celestial_pole_offset_y(self) -> CelestialPoleOffsetY {
        self.celestial_pole_offset_y
    }

    /// Returns the source-supplied angular and tabulated-UT1 uncertainties.
    pub const fn standard_uncertainties(self) -> EarthAttitudeStandardUncertainties {
        self.standard_uncertainties
    }
}

pub(crate) mod model {
    use super::*;

    pub trait Sealed {
        fn earth_attitude_state_at<S: TimeScale>(
            self,
            epoch: Instant<S>,
            terrestrial_time: JulianDate<Tt>,
            leap_seconds: LeapSeconds<'_>,
        ) -> Result<EarthAttitudeState<S>, Error>;

        fn earth_attitude_provenance(&self) -> EarthAttitudeModelProvenance<'_>;
    }
}

/// A sealed source of complete direction-level Earth-attitude quantities.
///
/// Built-in implementations cover tabulated full EOP, tabulated attitude
/// without length of day, and explicit [`PredictedEarthOrientation`] scenarios.
/// This capability can drive UT1 and terrestrial direction rotations but does
/// not by itself promise measured angular rates for state transforms.
pub trait EarthAttitudeModel: model::Sealed + Copy {}

impl<T: model::Sealed + Copy> EarthAttitudeModel for T {}

impl model::Sealed for EarthAttitudeTable<'_> {
    fn earth_attitude_state_at<S: TimeScale>(
        self,
        epoch: Instant<S>,
        _terrestrial_time: JulianDate<Tt>,
        leap_seconds: LeapSeconds<'_>,
    ) -> Result<EarthAttitudeState<S>, Error> {
        EarthAttitudeState::from_attitude(self.at(epoch, leap_seconds)?, leap_seconds)
    }

    fn earth_attitude_provenance(&self) -> EarthAttitudeModelProvenance<'_> {
        EarthAttitudeModelProvenance::tabulated(self.version)
    }
}

impl model::Sealed for EarthOrientationTable<'_> {
    fn earth_attitude_state_at<S: TimeScale>(
        self,
        epoch: Instant<S>,
        _terrestrial_time: JulianDate<Tt>,
        leap_seconds: LeapSeconds<'_>,
    ) -> Result<EarthAttitudeState<S>, Error> {
        EarthAttitudeState::from_orientation(self.at(epoch, leap_seconds)?, leap_seconds)
    }

    fn earth_attitude_provenance(&self) -> EarthAttitudeModelProvenance<'_> {
        EarthAttitudeModelProvenance::tabulated(self.version())
    }
}

impl model::Sealed for PredictedEarthOrientation<'_> {
    fn earth_attitude_state_at<S: TimeScale>(
        self,
        epoch: Instant<S>,
        terrestrial_time: JulianDate<Tt>,
        _leap_seconds: LeapSeconds<'_>,
    ) -> Result<EarthAttitudeState<S>, Error> {
        EarthAttitudeState::from_prediction(self, epoch, terrestrial_time)
    }

    fn earth_attitude_provenance(&self) -> EarthAttitudeModelProvenance<'_> {
        self.provenance()
    }
}
