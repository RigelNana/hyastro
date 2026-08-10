use crate::{math::Angle, uncertainty::StandardUncertainty};

use super::{
    CelestialPoleOffsetX, CelestialPoleOffsetY, DateTime, DeltaT, Duration,
    EarthAttitudeStandardUncertainties, Error, Gregorian, Instant, JulianDate, PolarMotionX,
    PolarMotionY, TimeContext, TimeInterval, TimeScale, Tt,
};

/// Whether a future Earth-attitude quantity is a prediction or an explicit assumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PredictionDisposition {
    /// A named model predicts the quantity.
    Predicted,
    /// The caller deliberately assumes the quantity rather than predicting it.
    Assumed,
}

/// One evaluated `TT−UT1` value and its optional one-sigma uncertainty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeltaTEstimate {
    value: Duration,
    standard_uncertainty: Option<StandardUncertainty<Duration>>,
}

impl DeltaTEstimate {
    /// Constructs an evaluated Delta T estimate.
    pub const fn new(
        value: Duration,
        standard_uncertainty: Option<StandardUncertainty<Duration>>,
    ) -> Self {
        Self {
            value,
            standard_uncertainty,
        }
    }

    /// Returns the estimated `TT−UT1` value.
    pub const fn value(self) -> Duration {
        self.value
    }

    /// Returns the model-supplied one-sigma uncertainty, when available.
    pub const fn standard_uncertainty(self) -> Option<StandardUncertainty<Duration>> {
        self.standard_uncertainty
    }
}

type DeltaTEvaluator = fn(JulianDate<Tt>) -> Result<DeltaTEstimate, Error>;

#[derive(Debug, Clone, Copy)]
enum DeltaTModelKind {
    Constant(DeltaTEstimate),
    EspenakMeeus2006,
    Custom(DeltaTEvaluator),
}

/// A named, validity-bounded model for future or historical `TT−UT1`.
///
/// Evaluation is performed from TT and therefore does not require a UTC label
/// or a future leap-second scenario. A constant model is useful for reproducing
/// a published eclipse scenario; custom models use a plain function pointer so
/// the value remains allocation-free and immutable.
#[derive(Debug, Clone, Copy)]
pub struct DeltaTModel<'a> {
    identifier: &'a str,
    validity: TimeInterval<Tt>,
    disposition: PredictionDisposition,
    kind: DeltaTModelKind,
}

impl<'a> DeltaTModel<'a> {
    /// Constructs a constant Delta T scenario over an explicit closed interval.
    pub fn constant(
        identifier: &'a str,
        validity: TimeInterval<Tt>,
        estimate: DeltaTEstimate,
    ) -> Result<Self, Error> {
        Self::new(
            identifier,
            validity,
            PredictionDisposition::Assumed,
            DeltaTModelKind::Constant(estimate),
        )
    }

    /// Constructs a caller-defined predictive Delta T model.
    pub fn custom(
        identifier: &'a str,
        validity: TimeInterval<Tt>,
        evaluator: DeltaTEvaluator,
    ) -> Result<Self, Error> {
        Self::new(
            identifier,
            validity,
            PredictionDisposition::Predicted,
            DeltaTModelKind::Custom(evaluator),
        )
    }

    /// Constructs the Espenak–Meeus polynomial model published by NASA in 2006.
    ///
    /// The model is accepted from astronomical year −1999 through +3000. NASA
    /// does not publish one uniform uncertainty polynomial for the full range,
    /// so evaluations retain `None` rather than inventing an error estimate.
    pub fn espenak_meeus_2006() -> Result<DeltaTModel<'static>, Error> {
        let time = TimeContext::builtin();
        let start = time.resolve(DateTime::<Gregorian, Tt>::from_components(
            -1999, 1, 1, 0, 0, 0, 0,
        )?)?;
        let end = time.resolve(DateTime::<Gregorian, Tt>::from_components(
            3000,
            12,
            31,
            23,
            59,
            59,
            999_999_999,
        )?)?;
        DeltaTModel::<'static>::new(
            "NASA Espenak-Meeus 2006 Delta T polynomials",
            TimeInterval::new(start, end)?,
            PredictionDisposition::Predicted,
            DeltaTModelKind::EspenakMeeus2006,
        )
    }

    fn new(
        identifier: &'a str,
        validity: TimeInterval<Tt>,
        disposition: PredictionDisposition,
        kind: DeltaTModelKind,
    ) -> Result<Self, Error> {
        if identifier.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "Delta T model identifier must not be empty",
            });
        }
        Ok(Self {
            identifier,
            validity,
            disposition,
            kind,
        })
    }

    /// Returns the stable model identifier.
    pub const fn identifier(self) -> &'a str {
        self.identifier
    }

    /// Returns the model's closed physical validity interval in TT.
    pub const fn validity(self) -> TimeInterval<Tt> {
        self.validity
    }

    /// Returns whether the model value is predicted or assumed.
    pub const fn disposition(self) -> PredictionDisposition {
        self.disposition
    }

    /// Evaluates the model at a TT Julian date inside its declared validity.
    pub fn evaluate(self, terrestrial_time: JulianDate<Tt>) -> Result<DeltaTEstimate, Error> {
        let time = TimeContext::builtin();
        let epoch = time.resolve(terrestrial_time.to_datetime::<Gregorian>()?)?;
        if !self.validity.contains(epoch) {
            return Err(Error::EarthOrientationPredictionOutsideValidity {
                requested: epoch.tai_nanoseconds_since_1900(),
                validity_start: self.validity.start().tai_nanoseconds_since_1900(),
                validity_end: self.validity.end().tai_nanoseconds_since_1900(),
            });
        }
        match self.kind {
            DeltaTModelKind::Constant(estimate) => Ok(estimate),
            DeltaTModelKind::EspenakMeeus2006 => Self::evaluate_espenak_meeus(terrestrial_time),
            DeltaTModelKind::Custom(evaluator) => evaluator(terrestrial_time),
        }
    }

    fn evaluate_espenak_meeus(terrestrial_time: JulianDate<Tt>) -> Result<DeltaTEstimate, Error> {
        let datetime = terrestrial_time.to_datetime::<Gregorian>()?;
        let year = f64::from(datetime.date().year());
        let y = year + (f64::from(datetime.date().month()) - 0.5) / 12.0;
        let seconds = if y < -500.0 {
            let u = (y - 1820.0) / 100.0;
            -20.0 + 32.0 * u * u
        } else if y < 500.0 {
            let u = y / 100.0;
            (((((0.009_031_652_1 * u + 0.022_174_192) * u - 0.179_845_2) * u - 5.952_053) * u
                + 33.783_11)
                * u
                - 1014.41)
                * u
                + 10_583.6
        } else if y < 1600.0 {
            let u = (y - 1000.0) / 100.0;
            (((((0.008_357_207_3 * u - 0.005_050_998) * u - 0.850_346_3) * u + 0.319_781) * u
                + 71.234_72)
                * u
                - 556.01)
                * u
                + 1574.2
        } else if y < 1700.0 {
            let t = y - 1600.0;
            ((t / 7129.0 - 0.015_32) * t - 0.9808) * t + 120.0
        } else if y < 1800.0 {
            let t = y - 1700.0;
            (((-t / 1_174_000.0 + 0.000_133_36) * t - 0.005_928_5) * t + 0.1603) * t + 8.83
        } else if y < 1860.0 {
            let t = y - 1800.0;
            let mut value = 0.000_000_000_875 * t - 0.000_000_169_9;
            value = value * t + 0.000_012_127_2;
            value = value * t - 0.000_374_36;
            value = value * t + 0.004_111_6;
            value = value * t + 0.006_861_2;
            value = value * t - 0.332_447;
            value * t + 13.72
        } else if y < 1900.0 {
            let t = y - 1860.0;
            ((((t / 233_174.0 - 0.000_447_362_4) * t + 0.016_806_68) * t - 0.251_754) * t + 0.5737)
                * t
                + 7.62
        } else if y < 1920.0 {
            let t = y - 1900.0;
            (((-0.000_197 * t + 0.006_196_6) * t - 0.059_893_9) * t + 1.494_119) * t - 2.79
        } else if y < 1941.0 {
            let t = y - 1920.0;
            ((0.002_093_6 * t - 0.076_100) * t + 0.844_93) * t + 21.20
        } else if y < 1961.0 {
            let t = y - 1950.0;
            (t / 2547.0 - 1.0 / 233.0) * t * t + 0.407 * t + 29.07
        } else if y < 1986.0 {
            let t = y - 1975.0;
            (-t / 718.0 - 1.0 / 260.0) * t * t + 1.067 * t + 45.45
        } else if y < 2005.0 {
            let t = y - 2000.0;
            ((((0.000_023_735_99 * t + 0.000_651_814) * t + 0.001_727_5) * t - 0.060_374) * t
                + 0.3345)
                * t
                + 63.86
        } else if y < 2050.0 {
            let t = y - 2000.0;
            (0.005_589 * t + 0.322_17) * t + 62.92
        } else if y < 2150.0 {
            let u = (y - 1820.0) / 100.0;
            -20.0 + 32.0 * u * u - 0.5628 * (2150.0 - y)
        } else {
            let u = (y - 1820.0) / 100.0;
            -20.0 + 32.0 * u * u
        };
        Ok(DeltaTEstimate::new(
            Duration::from_seconds_f64(seconds)?,
            None,
        ))
    }
}

/// Standard uncertainties for predicted or assumed polar-motion and pole-offset values.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EarthAttitudeOffsetUncertainties {
    polar_motion_x: Option<StandardUncertainty<Angle>>,
    polar_motion_y: Option<StandardUncertainty<Angle>>,
    celestial_pole_offset_x: Option<StandardUncertainty<Angle>>,
    celestial_pole_offset_y: Option<StandardUncertainty<Angle>>,
}

impl EarthAttitudeOffsetUncertainties {
    /// Constructs the four independently reported angular uncertainties.
    pub const fn new(
        polar_motion_x: Option<StandardUncertainty<Angle>>,
        polar_motion_y: Option<StandardUncertainty<Angle>>,
        celestial_pole_offset_x: Option<StandardUncertainty<Angle>>,
        celestial_pole_offset_y: Option<StandardUncertainty<Angle>>,
    ) -> Self {
        Self {
            polar_motion_x,
            polar_motion_y,
            celestial_pole_offset_x,
            celestial_pole_offset_y,
        }
    }

    /// Returns the $x_p$ standard uncertainty.
    pub const fn polar_motion_x(self) -> Option<StandardUncertainty<Angle>> {
        self.polar_motion_x
    }

    /// Returns the $y_p$ standard uncertainty.
    pub const fn polar_motion_y(self) -> Option<StandardUncertainty<Angle>> {
        self.polar_motion_y
    }

    /// Returns the $dX$ standard uncertainty.
    pub const fn celestial_pole_offset_x(self) -> Option<StandardUncertainty<Angle>> {
        self.celestial_pole_offset_x
    }

    /// Returns the $dY$ standard uncertainty.
    pub const fn celestial_pole_offset_y(self) -> Option<StandardUncertainty<Angle>> {
        self.celestial_pole_offset_y
    }
}

/// A named prediction or assumption for polar motion and celestial-pole offsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthAttitudeOffsetModel<'a> {
    identifier: &'a str,
    disposition: PredictionDisposition,
    polar_motion_x: PolarMotionX,
    polar_motion_y: PolarMotionY,
    celestial_pole_offset_x: CelestialPoleOffsetX,
    celestial_pole_offset_y: CelestialPoleOffsetY,
    standard_uncertainties: EarthAttitudeOffsetUncertainties,
}

impl<'a> EarthAttitudeOffsetModel<'a> {
    /// Constructs a validated offset prediction or assumption.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identifier: &'a str,
        disposition: PredictionDisposition,
        polar_motion_x: PolarMotionX,
        polar_motion_y: PolarMotionY,
        celestial_pole_offset_x: CelestialPoleOffsetX,
        celestial_pole_offset_y: CelestialPoleOffsetY,
        standard_uncertainties: EarthAttitudeOffsetUncertainties,
    ) -> Result<Self, Error> {
        if identifier.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "Earth-attitude offset model identifier must not be empty",
            });
        }
        Ok(Self {
            identifier,
            disposition,
            polar_motion_x,
            polar_motion_y,
            celestial_pole_offset_x,
            celestial_pole_offset_y,
            standard_uncertainties,
        })
    }

    /// Constructs an explicit zero-polar-motion, zero-pole-offset assumption.
    pub const fn assumed_zero() -> Self {
        let zero = Angle::from_finite(0.0);
        Self {
            identifier: "zero polar motion and celestial-pole offsets",
            disposition: PredictionDisposition::Assumed,
            polar_motion_x: PolarMotionX::from_angle(zero),
            polar_motion_y: PolarMotionY::from_angle(zero),
            celestial_pole_offset_x: CelestialPoleOffsetX::from_angle(zero),
            celestial_pole_offset_y: CelestialPoleOffsetY::from_angle(zero),
            standard_uncertainties: EarthAttitudeOffsetUncertainties {
                polar_motion_x: None,
                polar_motion_y: None,
                celestial_pole_offset_x: None,
                celestial_pole_offset_y: None,
            },
        }
    }

    /// Returns the stable model or assumption identifier.
    pub const fn identifier(self) -> &'a str {
        self.identifier
    }

    /// Returns whether the offsets are predicted or assumed.
    pub const fn disposition(self) -> PredictionDisposition {
        self.disposition
    }

    /// Returns $x_p$.
    pub const fn polar_motion_x(self) -> PolarMotionX {
        self.polar_motion_x
    }

    /// Returns $y_p$.
    pub const fn polar_motion_y(self) -> PolarMotionY {
        self.polar_motion_y
    }

    /// Returns $dX$.
    pub const fn celestial_pole_offset_x(self) -> CelestialPoleOffsetX {
        self.celestial_pole_offset_x
    }

    /// Returns $dY$.
    pub const fn celestial_pole_offset_y(self) -> CelestialPoleOffsetY {
        self.celestial_pole_offset_y
    }

    /// Returns the supplied offset uncertainties.
    pub const fn standard_uncertainties(self) -> EarthAttitudeOffsetUncertainties {
        self.standard_uncertainties
    }
}

/// Provenance of an Earth-attitude model used for terrestrial direction rotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarthAttitudeModelProvenance<'a> {
    source: &'a str,
    predicted: bool,
    delta_t_model: Option<&'a str>,
    delta_t_disposition: Option<PredictionDisposition>,
    offset_model: Option<&'a str>,
    offset_disposition: Option<PredictionDisposition>,
}

impl<'a> EarthAttitudeModelProvenance<'a> {
    pub(crate) const fn tabulated(source: &'a str) -> Self {
        Self {
            source,
            predicted: false,
            delta_t_model: None,
            delta_t_disposition: None,
            offset_model: None,
            offset_disposition: None,
        }
    }

    const fn predicted(
        source: &'a str,
        delta_t: DeltaTModel<'a>,
        offsets: EarthAttitudeOffsetModel<'a>,
    ) -> Self {
        Self {
            source,
            predicted: true,
            delta_t_model: Some(delta_t.identifier()),
            delta_t_disposition: Some(delta_t.disposition()),
            offset_model: Some(offsets.identifier()),
            offset_disposition: Some(offsets.disposition()),
        }
    }

    /// Returns the table version or prediction-scenario identifier.
    pub const fn source(self) -> &'a str {
        self.source
    }

    /// Returns whether this source is a future/historical modeled scenario.
    pub const fn is_predicted(self) -> bool {
        self.predicted
    }

    /// Returns the explicit Delta T model identifier, when modeled directly.
    pub const fn delta_t_model(self) -> Option<&'a str> {
        self.delta_t_model
    }

    /// Returns whether the modeled Delta T is predicted or assumed.
    pub const fn delta_t_disposition(self) -> Option<PredictionDisposition> {
        self.delta_t_disposition
    }

    /// Returns the polar-motion and celestial-pole offset model identifier.
    pub const fn offset_model(self) -> Option<&'a str> {
        self.offset_model
    }

    /// Returns whether the modeled offsets are predicted or assumed.
    pub const fn offset_disposition(self) -> Option<PredictionDisposition> {
        self.offset_disposition
    }
}

/// Explicit future or historical Earth-attitude scenario driven by Delta T.
///
/// The scenario evaluates UT1 directly from TT, so it remains usable when no
/// future UTC/leap-second mapping exists. It supplies direction attitude plus a
/// nominal site-rotation velocity; it cannot drive a measured state transform
/// because it carries no observed length of day or angular rates.
#[derive(Debug, Clone, Copy)]
pub struct PredictedEarthOrientation<'a> {
    identifier: &'a str,
    delta_t: DeltaTModel<'a>,
    offsets: EarthAttitudeOffsetModel<'a>,
}

impl<'a> PredictedEarthOrientation<'a> {
    /// Constructs a named prediction scenario from explicit component models.
    pub fn new(
        identifier: &'a str,
        delta_t: DeltaTModel<'a>,
        offsets: EarthAttitudeOffsetModel<'a>,
    ) -> Result<Self, Error> {
        if identifier.is_empty() {
            return Err(Error::InvalidEarthOrientationData {
                reason: "predicted Earth-orientation identifier must not be empty",
            });
        }
        Ok(Self {
            identifier,
            delta_t,
            offsets,
        })
    }

    /// Returns the scenario identifier.
    pub const fn identifier(self) -> &'a str {
        self.identifier
    }

    /// Returns the Delta T model.
    pub const fn delta_t_model(self) -> DeltaTModel<'a> {
        self.delta_t
    }

    /// Returns the polar-motion and celestial-pole offset model.
    pub const fn offset_model(self) -> EarthAttitudeOffsetModel<'a> {
        self.offsets
    }

    /// Returns the scenario's closed physical coverage interval.
    pub const fn coverage(self) -> TimeInterval<Tt> {
        self.delta_t.validity()
    }

    /// Evaluates Delta T at one physical instant without consulting UTC.
    pub fn delta_t_at<S: TimeScale>(
        self,
        epoch: Instant<S>,
        terrestrial_time: JulianDate<Tt>,
    ) -> Result<(DeltaT<S>, Option<StandardUncertainty<Duration>>), Error> {
        let estimate = self.delta_t.evaluate(terrestrial_time)?;
        Ok((
            DeltaT::new(epoch, estimate.value()),
            estimate.standard_uncertainty(),
        ))
    }

    /// Returns complete component provenance for this scenario.
    pub const fn provenance(self) -> EarthAttitudeModelProvenance<'a> {
        EarthAttitudeModelProvenance::predicted(self.identifier, self.delta_t, self.offsets)
    }

    pub(crate) fn standard_uncertainties(self) -> EarthAttitudeStandardUncertainties {
        let offsets = self.offsets.standard_uncertainties();
        EarthAttitudeStandardUncertainties::new(
            None,
            offsets.polar_motion_x(),
            offsets.polar_motion_y(),
            offsets.celestial_pole_offset_x(),
            offsets.celestial_pole_offset_y(),
        )
    }
}
