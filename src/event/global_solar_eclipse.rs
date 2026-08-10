use core::f64::consts::TAU;
use std::vec::Vec;

use crate::{
    astro::{Astrometry, GeocentricApparentPlace, ReceptionLightTimeOptions, SolarApparentPlace},
    earth::{Earth, ReferenceEllipsoid},
    ephem::{CelestialBody, EphemerisProvenance, EphemerisProvider},
    frame::{
        EclipticDirection, EclipticDirectionAt, EclipticLatitude, EclipticLongitude,
        EquatorialDirection, EquatorialDirectionAt, Frames, TrueEclipticEquinoxOfDate,
        TrueEquatorEquinoxOfDate,
    },
    math::{Angle, AngularSpeed, Declination, Dimensionless, HourAngle, Length, RightAscension},
    time::{Duration, Instant, JulianDate, TimeInterval, TimeScale, Tt},
};
use libm::{asin, atan2, cos, sin, sqrt};

use super::{
    Error, Events, ExtremumEvidence, MoonPhase, SolarEclipseModel, SolarEclipseSearchOptions,
    search::{BracketedExtremumSearch, BracketedRootSearch},
};

/// Signed distance of the lunar shadow axis from Earth's centre at greatest eclipse.
///
/// The magnitude is expressed in equatorial Earth radii. The sign is positive when the closest
/// point of the axis lies north of Earth's true equator of date and negative when it lies south.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SolarEclipseGamma(f64);

impl SolarEclipseGamma {
    fn from_geometry(axis_offset: ShadowVector, equatorial_radius_metres: f64) -> Self {
        let magnitude = axis_offset.norm() / equatorial_radius_metres;
        Self(magnitude.copysign(axis_offset.component(2)))
    }

    /// Returns the signed distance in equatorial Earth radii.
    pub const fn as_equatorial_radii(self) -> f64 {
        self.0
    }

    /// Returns the unsigned distance in equatorial Earth radii.
    pub fn absolute(self) -> f64 {
        self.0.abs()
    }
}

/// Signed radius of the core lunar shadow on a plane normal to its axis.
///
/// Positive values denote the umbra, negative values the antumbra, and zero is the shadow-cone
/// vertex. The magnitude is the radius of the corresponding total or annular shadow cross-section.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SolarShadowRadius(f64);

impl SolarShadowRadius {
    fn from_metres(value: f64) -> Result<Self, Error> {
        Length::from_metres(value)?;
        Ok(Self(value))
    }

    /// Returns the signed shadow radius in metres.
    pub const fn as_metres(self) -> f64 {
        self.0
    }

    /// Returns whether the section lies before the cone vertex and therefore carries umbra.
    pub const fn is_umbral(self) -> bool {
        self.0 > 0.0
    }

    /// Returns whether the section lies beyond the cone vertex and therefore carries antumbra.
    pub const fn is_antumbral(self) -> bool {
        self.0 < 0.0
    }
}

/// One rectangular coordinate on the Besselian fundamental plane.
///
/// The value is expressed in units of the selected Earth's equatorial radius. Positive `x` points
/// east and positive `y` points north on the true equator and equinox of date axes.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct BesselianPlaneCoordinate(f64);

impl BesselianPlaneCoordinate {
    fn from_equatorial_radii(value: f64) -> Result<Self, Error> {
        Dimensionless::new(value)?;
        Ok(Self(value))
    }

    /// Returns the coordinate in equatorial Earth radii.
    pub const fn as_equatorial_radii(self) -> f64 {
        self.0
    }
}

/// Conventional signed Besselian shadow radius on the fundamental plane.
///
/// `l1` is the positive penumbral radius. For `l2`, negative values denote an umbral section and
/// positive values denote an antumbral section. This sign convention is the opposite of
/// [`SolarShadowRadius`].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct BesselianShadowRadius(f64);

impl BesselianShadowRadius {
    fn from_equatorial_radii(value: f64) -> Result<Self, Error> {
        Dimensionless::new(value)?;
        Ok(Self(value))
    }

    /// Returns the signed radius in equatorial Earth radii.
    pub const fn as_equatorial_radii(self) -> f64 {
        self.0
    }
}

/// Positive lunar-radius ratio used by one Besselian shadow cone.
///
/// The value is measured in units of the selected Earth's equatorial radius. Separate penumbral
/// and umbral values preserve published `k1/k2` conventions without pretending that they describe
/// one physical lunar sphere.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct BesselianLunarRadiusRatio(f64);

impl BesselianLunarRadiusRatio {
    /// Constructs a positive lunar-radius ratio.
    pub fn try_from_equatorial_radii(value: f64) -> Result<Self, Error> {
        Dimensionless::new(value)?;
        if value <= 0.0 {
            return Err(Error::InvalidBesselianLimbModelValue {
                field: "lunar radius in equatorial Earth radii",
                value,
            });
        }
        Ok(Self(value))
    }

    /// Returns the radius in equatorial Earth radii.
    pub const fn as_equatorial_radii(self) -> f64 {
        self.0
    }
}

/// Explicit solar and lunar radius convention for Besselian elements.
///
/// This model is independent of the physical [`SolarEclipseModel`] used by the exact global shadow
/// classifier. Published eclipse tables commonly use distinct empirical lunar ratios `k1` and
/// `k2` plus ecliptic-latitude/longitude corrections `Δb/Δl`; physical common-tangent geometry
/// uses one spherical lunar radius and zero positional correction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesselianLimbModel {
    identifier: &'static str,
    source: &'static str,
    solar_radius: Length,
    penumbral_lunar_radius: BesselianLunarRadiusRatio,
    umbral_lunar_radius: BesselianLunarRadiusRatio,
    lunar_latitude_correction: Angle,
    lunar_longitude_correction: Angle,
    physical_model: Option<SolarEclipseModel>,
}

impl BesselianLimbModel {
    /// Stable identifier for the NASA Five Millennium Canon radius convention.
    pub const NASA_FIVE_MILLENNIUM_IDENTIFIER: &'static str =
        "NASA Five Millennium Canon solar-eclipse Besselian k1/k2 convention";

    /// Stable source citation for the NASA Five Millennium Canon convention.
    pub const NASA_FIVE_MILLENNIUM_SOURCE: &'static str =
        "https://eclipse.gsfc.nasa.gov/SEbeselm/SEbeselm2001/SE2024Apr08Tbeselm.html";

    /// Constructs a validated explicit Besselian radius convention.
    pub fn new(
        identifier: &'static str,
        source: &'static str,
        solar_radius: Length,
        penumbral_lunar_radius: BesselianLunarRadiusRatio,
        umbral_lunar_radius: BesselianLunarRadiusRatio,
        lunar_latitude_correction: Angle,
        lunar_longitude_correction: Angle,
    ) -> Result<Self, Error> {
        if identifier.trim().is_empty() {
            return Err(Error::EmptyBesselianLimbModelIdentifier);
        }
        if source.trim().is_empty() {
            return Err(Error::EmptyBesselianLimbModelSource);
        }
        if solar_radius.as_metres() <= 0.0 {
            return Err(Error::InvalidBesselianLimbModelValue {
                field: "solar radius in metres",
                value: solar_radius.as_metres(),
            });
        }
        if penumbral_lunar_radius.as_equatorial_radii() < umbral_lunar_radius.as_equatorial_radii()
        {
            return Err(Error::InvalidBesselianLimbModelValue {
                field: "k1 minus k2",
                value: penumbral_lunar_radius.as_equatorial_radii()
                    - umbral_lunar_radius.as_equatorial_radii(),
            });
        }
        Ok(Self {
            identifier,
            solar_radius,
            penumbral_lunar_radius,
            umbral_lunar_radius,
            lunar_latitude_correction,
            source,
            lunar_longitude_correction,
            physical_model: None,
        })
    }

    /// Constructs the NASA Five Millennium Canon `k1/k2` convention with its zero `Δb/Δl`.
    pub const fn nasa_five_millennium() -> Self {
        Self {
            identifier: Self::NASA_FIVE_MILLENNIUM_IDENTIFIER,
            solar_radius: Length::from_finite(696_000_000.0),
            penumbral_lunar_radius: BesselianLunarRadiusRatio(0.272_488),
            source: Self::NASA_FIVE_MILLENNIUM_SOURCE,
            umbral_lunar_radius: BesselianLunarRadiusRatio(0.272_281),
            lunar_latitude_correction: Angle::from_finite(0.0),
            lunar_longitude_correction: Angle::from_finite(0.0),
            physical_model: None,
        }
    }

    /// Derives one physical common-tangent convention from spherical Sun and Moon figures.
    pub fn physical(earth: Earth, model: SolarEclipseModel) -> Self {
        let equatorial_radius = earth.reference_ellipsoid().semi_major_axis().as_metres();
        let lunar_radius =
            BesselianLunarRadiusRatio(model.moon().radius().as_metres() / equatorial_radius);
        Self {
            identifier: "physical common-tangent spherical Sun/Moon figures",
            solar_radius: model.sun().radius(),
            penumbral_lunar_radius: lunar_radius,
            umbral_lunar_radius: lunar_radius,
            source: "caller-supplied SolarEclipseModel figure identifiers",
            lunar_latitude_correction: Angle::from_finite(0.0),
            lunar_longitude_correction: Angle::from_finite(0.0),
            physical_model: Some(model),
        }
    }

    /// Returns the stable model identifier.
    pub const fn identifier(self) -> &'static str {
        self.identifier
    }

    /// Returns the stable source citation.
    pub const fn source(self) -> &'static str {
        self.source
    }

    /// Returns the adopted solar radius.
    pub const fn solar_radius(self) -> Length {
        self.solar_radius
    }

    /// Returns the penumbral lunar-radius ratio `k1`.
    pub const fn penumbral_lunar_radius(self) -> BesselianLunarRadiusRatio {
        self.penumbral_lunar_radius
    }

    /// Returns the umbral lunar-radius ratio `k2`.
    pub const fn umbral_lunar_radius(self) -> BesselianLunarRadiusRatio {
        self.umbral_lunar_radius
    }

    /// Returns the additive true-ecliptic lunar-latitude correction `Δb`.
    pub const fn lunar_latitude_correction(self) -> Angle {
        self.lunar_latitude_correction
    }

    /// Returns the additive true-ecliptic lunar-longitude correction `Δl`.
    pub const fn lunar_longitude_correction(self) -> Angle {
        self.lunar_longitude_correction
    }

    /// Returns the source physical spherical model, when this convention was derived from one.
    pub const fn physical_model(self) -> Option<SolarEclipseModel> {
        self.physical_model
    }
}

/// Light-time controls and explicit limb convention for Besselian-element evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesselianElementsOptions {
    limb_model: BesselianLimbModel,
    light_time: ReceptionLightTimeOptions,
}

impl BesselianElementsOptions {
    /// Combines one explicit limb convention with reception light-time controls.
    pub const fn new(
        limb_model: BesselianLimbModel,
        light_time: ReceptionLightTimeOptions,
    ) -> Self {
        Self {
            limb_model,
            light_time,
        }
    }

    /// Uses the NASA Five Millennium Canon radius convention and standard light-time controls.
    pub const fn nasa_five_millennium() -> Self {
        Self::new(
            BesselianLimbModel::nasa_five_millennium(),
            ReceptionLightTimeOptions::standard(),
        )
    }

    /// Derives a physical Besselian convention from existing global-eclipse search options.
    pub fn physical(earth: Earth, options: SolarEclipseSearchOptions) -> Self {
        Self::new(
            BesselianLimbModel::physical(earth, options.model()),
            options.angular_search().light_time(),
        )
    }

    /// Returns the selected Besselian limb convention.
    pub const fn limb_model(self) -> BesselianLimbModel {
        self.limb_model
    }

    /// Returns the reception light-time controls.
    pub const fn light_time(self) -> ReceptionLightTimeOptions {
        self.light_time
    }
}

/// Fit controls for a short-lived Besselian-element polynomial.
///
/// Five evenly spaced apparent-place samples cover the closed validity interval. The standard
/// six-hour publication window is centred on the caller's reference epoch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesselianPolynomialOptions {
    elements: BesselianElementsOptions,
    validity_half_span: Duration,
}

impl BesselianPolynomialOptions {
    const MAXIMUM_HALF_SPAN: Duration =
        Duration::from_nanoseconds(6 * 3_600 * Duration::NANOSECONDS_PER_SECOND);

    /// Constructs validated fit controls.
    pub fn new(
        elements: BesselianElementsOptions,
        validity_half_span: Duration,
    ) -> Result<Self, Error> {
        if validity_half_span <= Duration::ZERO || validity_half_span > Self::MAXIMUM_HALF_SPAN {
            return Err(Error::InvalidSearchDuration {
                field: "Besselian polynomial validity half-span",
                nanoseconds: validity_half_span.as_nanoseconds(),
                maximum_nanoseconds: Self::MAXIMUM_HALF_SPAN.as_nanoseconds(),
            });
        }
        Ok(Self {
            elements,
            validity_half_span,
        })
    }

    /// Uses a six-hour window and the NASA Five Millennium Canon radius convention.
    pub const fn nasa_six_hour() -> Self {
        Self {
            elements: BesselianElementsOptions::nasa_five_millennium(),
            validity_half_span: Duration::from_nanoseconds(
                3 * 3_600 * Duration::NANOSECONDS_PER_SECOND,
            ),
        }
    }

    /// Uses a six-hour window and physical spherical figures from global-eclipse search options.
    pub fn physical_six_hour(earth: Earth, options: SolarEclipseSearchOptions) -> Self {
        Self {
            elements: BesselianElementsOptions::physical(earth, options),
            validity_half_span: Duration::from_nanoseconds(
                3 * 3_600 * Duration::NANOSECONDS_PER_SECOND,
            ),
        }
    }

    /// Returns the instantaneous-element controls sampled by the fit.
    pub const fn elements(self) -> BesselianElementsOptions {
        self.elements
    }

    /// Returns half the closed validity interval.
    pub const fn validity_half_span(self) -> Duration {
        self.validity_half_span
    }
}

/// First derivative of a dimensionless Besselian coordinate or radius.
///
/// The canonical value is per SI second. Published eclipse tables normally use the equivalent
/// value per TT hour.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct BesselianElementRate(f64);

impl BesselianElementRate {
    fn from_per_second(value: f64) -> Result<Self, Error> {
        Dimensionless::new(value)?;
        Ok(Self(value))
    }

    /// Returns the derivative per SI second.
    pub const fn as_per_second(self) -> f64 {
        self.0
    }

    /// Returns the derivative per TT hour.
    pub const fn as_per_tt_hour(self) -> f64 {
        self.0 * 3_600.0
    }
}

/// Method used to derive one set of Besselian-element rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BesselianDerivativeMethod {
    /// Symmetric numerical difference with the stated half-step.
    SymmetricDifference {
        /// Time from the centre sample to either side sample.
        half_step: Duration,
    },
    /// Exact derivative of a retained Besselian polynomial.
    AnalyticPolynomial,
}

/// First derivatives of the six time-varying Besselian elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesselianElementDerivatives {
    x: BesselianElementRate,
    y: BesselianElementRate,
    d: AngularSpeed,
    mu: AngularSpeed,
    l1: BesselianElementRate,
    l2: BesselianElementRate,
    method: BesselianDerivativeMethod,
}

impl BesselianElementDerivatives {
    /// Returns `dx/dt`.
    pub const fn x(self) -> BesselianElementRate {
        self.x
    }

    /// Returns `dy/dt`.
    pub const fn y(self) -> BesselianElementRate {
        self.y
    }

    /// Returns the shadow-axis declination derivative `dd/dt`.
    pub const fn d(self) -> AngularSpeed {
        self.d
    }

    /// Returns `dd/dt` in degrees per TT hour.
    pub fn d_degrees_per_tt_hour(self) -> f64 {
        self.d.as_radians_per_second().to_degrees() * 3_600.0
    }

    /// Returns the ephemeris-hour-angle derivative `dμ/dt`.
    pub const fn mu(self) -> AngularSpeed {
        self.mu
    }

    /// Returns `dμ/dt` in degrees per TT hour.
    pub fn mu_degrees_per_tt_hour(self) -> f64 {
        self.mu.as_radians_per_second().to_degrees() * 3_600.0
    }

    /// Returns `dl1/dt`.
    pub const fn l1(self) -> BesselianElementRate {
        self.l1
    }

    /// Returns `dl2/dt`.
    pub const fn l2(self) -> BesselianElementRate {
        self.l2
    }

    /// Returns how the rates were derived.
    pub const fn method(self) -> BesselianDerivativeMethod {
        self.method
    }

    /// Returns the symmetric numerical half-step, or `None` for analytic polynomial derivatives.
    pub const fn numerical_time_step(self) -> Option<Duration> {
        match self.method {
            BesselianDerivativeMethod::SymmetricDifference { half_step } => Some(half_step),
            BesselianDerivativeMethod::AnalyticPolynomial => None,
        }
    }
}

/// Fundamental plane through Earth's centre and normal to the lunar shadow axis.
///
/// The retained shadow-axis direction is point `Z`, directed from the Moon towards the Sun on true
/// equator and equinox of date axes. The plane origin is the geocentre; positive `x` is east and
/// positive `y` is north.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesselianFundamentalPlane<S: TimeScale> {
    shadow_axis: EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>,
}

impl<S: TimeScale> BesselianFundamentalPlane<S> {
    /// Returns the physical epoch of the plane.
    pub const fn epoch(self) -> Instant<S> {
        self.shadow_axis.epoch()
    }

    /// Returns point `Z`, the Moon-to-Sun shadow-axis direction.
    pub const fn shadow_axis(self) -> EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S> {
        self.shadow_axis
    }
}

/// Instantaneous Besselian solar-eclipse elements and their first derivatives.
///
/// `x` and `y` locate the lunar shadow axis on the fundamental plane; `d` and `μ` orient that axis;
/// `l1` and `l2` are the penumbral and conventional signed umbral radii. Cone tangents are retained
/// because downstream contact and path algorithms require them. The selected Earth, explicit
/// Besselian limb convention, ephemeris provenance, and numerical derivative evidence remain
/// explicit.
#[derive(Debug, Clone, PartialEq)]
pub struct BesselianElements<S: TimeScale> {
    plane: BesselianFundamentalPlane<S>,
    x: BesselianPlaneCoordinate,
    y: BesselianPlaneCoordinate,
    mu: HourAngle,
    l1: BesselianShadowRadius,
    l2: BesselianShadowRadius,
    tan_f1: Dimensionless,
    tan_f2: Dimensionless,
    derivatives: BesselianElementDerivatives,
    earth: Earth,
    limb_model: BesselianLimbModel,
    ephemeris: EphemerisProvenance,
    astrometric_evaluations: u32,
}

impl<S: TimeScale> BesselianElements<S> {
    /// Returns the physical evaluation epoch.
    pub const fn epoch(&self) -> Instant<S> {
        self.plane.epoch()
    }

    /// Returns the fully oriented Besselian fundamental plane.
    pub const fn fundamental_plane(&self) -> BesselianFundamentalPlane<S> {
        self.plane
    }

    /// Returns the conventional fundamental-plane `x` coordinate.
    pub const fn x(&self) -> BesselianPlaneCoordinate {
        self.x
    }

    /// Returns the conventional fundamental-plane `y` coordinate.
    pub const fn y(&self) -> BesselianPlaneCoordinate {
        self.y
    }

    /// Returns the shadow-axis declination `d`.
    pub const fn d(&self) -> Declination {
        self.plane.shadow_axis().coordinates().declination()
    }

    /// Returns the Greenwich ephemeris hour angle `μ` of the shadow axis.
    pub const fn mu(&self) -> HourAngle {
        self.mu
    }

    /// Returns the penumbral radius `l1` on the fundamental plane.
    pub const fn l1(&self) -> BesselianShadowRadius {
        self.l1
    }

    /// Returns the conventional signed umbral radius `l2` on the fundamental plane.
    pub const fn l2(&self) -> BesselianShadowRadius {
        self.l2
    }

    /// Returns the contact-convention core-shadow radius on the fundamental plane.
    ///
    /// This is exactly `-l2 * a`, where `a` is the selected Earth's equatorial radius. It uses
    /// this element set's explicit [`BesselianLimbModel`] and is independent of the physical
    /// common-tangent cone used by [`GlobalSolarEclipseMaximum`].
    pub fn contact_core_shadow_radius_at_fundamental_plane(&self) -> SolarShadowRadius {
        let equatorial_radius = self
            .earth
            .reference_ellipsoid()
            .semi_major_axis()
            .as_metres();
        SolarShadowRadius(-self.l2.as_equatorial_radii() * equatorial_radius)
    }

    /// Returns the tangent of the penumbral cone half-angle `f1`.
    pub const fn tan_f1(&self) -> Dimensionless {
        self.tan_f1
    }

    /// Returns the tangent of the umbral cone half-angle `f2`.
    pub const fn tan_f2(&self) -> Dimensionless {
        self.tan_f2
    }

    /// Returns numerical first derivatives with respect to TT.
    pub const fn derivatives(&self) -> BesselianElementDerivatives {
        self.derivatives
    }

    /// Returns the selected Earth reference ellipsoid.
    pub const fn earth(&self) -> Earth {
        self.earth
    }

    /// Returns the explicit solar and lunar radius convention.
    pub const fn limb_model(&self) -> BesselianLimbModel {
        self.limb_model
    }

    /// Returns the ephemeris model and data provenance.
    pub const fn ephemeris(&self) -> &EphemerisProvenance {
        &self.ephemeris
    }

    /// Returns the number of individual geocentric apparent-place evaluations.
    pub const fn astrometric_evaluations(&self) -> u32 {
        self.astrometric_evaluations
    }
}

/// Polynomial for a dimensionless Besselian coordinate or shadow radius.
///
/// Coefficient index `n` is measured in equatorial Earth radii per TT-hour to power `n`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesselianScalarPolynomial<const N: usize> {
    coefficients: [f64; N],
}

impl<const N: usize> BesselianScalarPolynomial<N> {
    const fn new(coefficients: [f64; N]) -> Self {
        Self { coefficients }
    }

    /// Returns one coefficient, or `None` when `power` lies outside this polynomial's degree.
    pub fn coefficient(self, power: usize) -> Option<f64> {
        self.coefficients.get(power).copied()
    }

    /// Returns all coefficients in ascending power order.
    pub const fn coefficients(self) -> [f64; N] {
        self.coefficients
    }

    fn evaluate(self, tt_hours: f64) -> f64 {
        self.coefficients
            .iter()
            .rev()
            .fold(0.0, |value, coefficient| {
                value.mul_add(tt_hours, *coefficient)
            })
    }

    fn derivative_per_tt_hour(self, tt_hours: f64) -> f64 {
        self.coefficients
            .iter()
            .enumerate()
            .skip(1)
            .rev()
            .fold(0.0, |value, (power, coefficient)| {
                value.mul_add(tt_hours, *coefficient * power as f64)
            })
    }
}

/// Polynomial for a Besselian angular element.
///
/// Coefficients are stored canonically in radians per TT-hour to the coefficient's power.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesselianAnglePolynomial<const N: usize> {
    coefficients_radians: [f64; N],
}

impl<const N: usize> BesselianAnglePolynomial<N> {
    const fn new(coefficients_radians: [f64; N]) -> Self {
        Self {
            coefficients_radians,
        }
    }

    /// Returns one coefficient in radians per TT-hour to `power`.
    pub fn coefficient_radians(self, power: usize) -> Option<f64> {
        self.coefficients_radians.get(power).copied()
    }

    /// Returns one coefficient in degrees per TT-hour to `power`.
    pub fn coefficient_degrees(self, power: usize) -> Option<f64> {
        self.coefficient_radians(power).map(f64::to_degrees)
    }

    /// Returns all coefficients in radians and ascending power order.
    pub const fn coefficients_radians(self) -> [f64; N] {
        self.coefficients_radians
    }

    fn evaluate_radians(self, tt_hours: f64) -> f64 {
        self.coefficients_radians
            .iter()
            .rev()
            .fold(0.0, |value, coefficient| {
                value.mul_add(tt_hours, *coefficient)
            })
    }

    fn derivative_radians_per_tt_hour(self, tt_hours: f64) -> f64 {
        self.coefficients_radians
            .iter()
            .enumerate()
            .skip(1)
            .rev()
            .fold(0.0, |value, (power, coefficient)| {
                value.mul_add(tt_hours, *coefficient * power as f64)
            })
    }
}

/// Maximum sampled residuals of a Besselian polynomial fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesselianPolynomialResiduals {
    x: BesselianPlaneCoordinate,
    y: BesselianPlaneCoordinate,
    d: Angle,
    mu: Angle,
    l1: BesselianShadowRadius,
    l2: BesselianShadowRadius,
    axis_right_ascension: Angle,
    tan_f1: Dimensionless,
    tan_f2: Dimensionless,
}

impl BesselianPolynomialResiduals {
    /// Returns the maximum absolute `x` residual.
    pub const fn x(self) -> BesselianPlaneCoordinate {
        self.x
    }

    /// Returns the maximum absolute `y` residual.
    pub const fn y(self) -> BesselianPlaneCoordinate {
        self.y
    }

    /// Returns the maximum absolute shadow-axis declination residual.
    pub const fn d(self) -> Angle {
        self.d
    }

    /// Returns the maximum absolute ephemeris-hour-angle residual.
    pub const fn mu(self) -> Angle {
        self.mu
    }

    /// Returns the maximum absolute `l1` residual.
    pub const fn l1(self) -> BesselianShadowRadius {
        self.l1
    }

    /// Returns the maximum absolute `l2` residual.
    pub const fn l2(self) -> BesselianShadowRadius {
        self.l2
    }

    /// Returns the maximum residual of the retained internal shadow-axis right ascension.
    pub const fn axis_right_ascension(self) -> Angle {
        self.axis_right_ascension
    }

    /// Returns the maximum absolute `tan(f1)` residual.
    pub const fn tan_f1(self) -> Dimensionless {
        self.tan_f1
    }

    /// Returns the maximum absolute `tan(f2)` residual.
    pub const fn tan_f2(self) -> Dimensionless {
        self.tan_f2
    }
}

/// Short-lived Besselian solar-eclipse polynomial centred on one reference epoch.
///
/// `x` and `y` are cubic, `d`, `l1`, and `l2` are quadratic, and `μ` is linear in TT hours from
/// `reference_epoch`. A private cubic right-ascension fit keeps the returned fundamental plane
/// fully oriented without requiring a time context at evaluation. The fit is rejected outside its
/// explicit closed validity interval.
#[derive(Debug, Clone, PartialEq)]
pub struct BesselianElementsPolynomial<S: TimeScale> {
    reference_epoch: Instant<S>,
    validity: TimeInterval<S>,
    x: BesselianScalarPolynomial<4>,
    y: BesselianScalarPolynomial<4>,
    d: BesselianAnglePolynomial<3>,
    mu: BesselianAnglePolynomial<2>,
    l1: BesselianScalarPolynomial<3>,
    l2: BesselianScalarPolynomial<3>,
    axis_right_ascension: BesselianAnglePolynomial<4>,
    tan_f1: Dimensionless,
    tan_f2: Dimensionless,
    residuals: BesselianPolynomialResiduals,
    earth: Earth,
    limb_model: BesselianLimbModel,
    ephemeris: EphemerisProvenance,
    astrometric_evaluations: u32,
}

impl<S: TimeScale> BesselianElementsPolynomial<S> {
    /// Returns the epoch from which polynomial time is measured in TT hours.
    pub const fn reference_epoch(&self) -> Instant<S> {
        self.reference_epoch
    }

    /// Returns the closed interval on which evaluation is accepted.
    pub const fn validity(&self) -> TimeInterval<S> {
        self.validity
    }

    /// Returns the cubic `x` polynomial.
    pub const fn x(&self) -> BesselianScalarPolynomial<4> {
        self.x
    }

    /// Returns the cubic `y` polynomial.
    pub const fn y(&self) -> BesselianScalarPolynomial<4> {
        self.y
    }

    /// Returns the quadratic shadow-axis declination polynomial.
    pub const fn d(&self) -> BesselianAnglePolynomial<3> {
        self.d
    }

    /// Returns the linear ephemeris-hour-angle polynomial.
    pub const fn mu(&self) -> BesselianAnglePolynomial<2> {
        self.mu
    }

    /// Returns the quadratic `l1` polynomial.
    pub const fn l1(&self) -> BesselianScalarPolynomial<3> {
        self.l1
    }

    /// Returns the quadratic `l2` polynomial.
    pub const fn l2(&self) -> BesselianScalarPolynomial<3> {
        self.l2
    }

    /// Returns the fitted constant `tan(f1)`.
    pub const fn tan_f1(&self) -> Dimensionless {
        self.tan_f1
    }

    /// Returns the fitted constant `tan(f2)`.
    pub const fn tan_f2(&self) -> Dimensionless {
        self.tan_f2
    }

    /// Returns maximum residuals measured at all five fit samples.
    pub const fn residuals(&self) -> BesselianPolynomialResiduals {
        self.residuals
    }

    /// Returns the selected Earth reference ellipsoid.
    pub const fn earth(&self) -> Earth {
        self.earth
    }

    /// Returns the explicit solar and lunar radius convention.
    pub const fn limb_model(&self) -> BesselianLimbModel {
        self.limb_model
    }

    /// Returns the ephemeris model and data provenance used for the fit.
    pub const fn ephemeris(&self) -> &EphemerisProvenance {
        &self.ephemeris
    }

    /// Returns the number of apparent-place evaluations consumed by the fit.
    pub const fn astrometric_evaluations(&self) -> u32 {
        self.astrometric_evaluations
    }

    fn tt_hours_at(&self, epoch: Instant<S>) -> Result<f64, Error> {
        if !self.validity.contains(epoch) {
            return Err(Error::BesselianPolynomialOutsideValidity {
                epoch_tai_nanoseconds: epoch.tai_nanoseconds_since_1900(),
                start_tai_nanoseconds: self.validity.start().tai_nanoseconds_since_1900(),
                end_tai_nanoseconds: self.validity.end().tai_nanoseconds_since_1900(),
            });
        }
        Ok(epoch.duration_since(self.reference_epoch)?.as_seconds_f64() / 3_600.0)
    }

    fn derivatives_at_tt_hours(&self, tt_hours: f64) -> Result<BesselianElementDerivatives, Error> {
        Ok(BesselianElementDerivatives {
            x: BesselianElementRate::from_per_second(
                self.x.derivative_per_tt_hour(tt_hours) / 3_600.0,
            )?,
            y: BesselianElementRate::from_per_second(
                self.y.derivative_per_tt_hour(tt_hours) / 3_600.0,
            )?,
            d: AngularSpeed::from_radians_per_second(
                self.d.derivative_radians_per_tt_hour(tt_hours) / 3_600.0,
            )?,
            mu: AngularSpeed::from_radians_per_second(
                self.mu.derivative_radians_per_tt_hour(tt_hours) / 3_600.0,
            )?,
            l1: BesselianElementRate::from_per_second(
                self.l1.derivative_per_tt_hour(tt_hours) / 3_600.0,
            )?,
            l2: BesselianElementRate::from_per_second(
                self.l2.derivative_per_tt_hour(tt_hours) / 3_600.0,
            )?,
            method: BesselianDerivativeMethod::AnalyticPolynomial,
        })
    }

    /// Evaluates analytic first derivatives inside the retained validity interval.
    pub fn derivatives_at(&self, epoch: Instant<S>) -> Result<BesselianElementDerivatives, Error> {
        self.derivatives_at_tt_hours(self.tt_hours_at(epoch)?)
    }

    /// Evaluates elements and analytic first derivatives inside the retained validity interval.
    pub fn elements_at(&self, epoch: Instant<S>) -> Result<BesselianElements<S>, Error> {
        let tt_hours = self.tt_hours_at(epoch)?;
        let right_ascension =
            RightAscension::wrap_radians(self.axis_right_ascension.evaluate_radians(tt_hours))?;
        let declination = Declination::try_from_radians(self.d.evaluate_radians(tt_hours))?;
        let mu = HourAngle::wrap_radians(self.mu.evaluate_radians(tt_hours))?;
        let derivatives = self.derivatives_at_tt_hours(tt_hours)?;

        Ok(BesselianElements {
            plane: BesselianFundamentalPlane {
                shadow_axis: EquatorialDirectionAt::new(
                    epoch,
                    EquatorialDirection::new(right_ascension, declination),
                ),
            },
            x: BesselianPlaneCoordinate::from_equatorial_radii(self.x.evaluate(tt_hours))?,
            y: BesselianPlaneCoordinate::from_equatorial_radii(self.y.evaluate(tt_hours))?,
            mu,
            l1: BesselianShadowRadius::from_equatorial_radii(self.l1.evaluate(tt_hours))?,
            l2: BesselianShadowRadius::from_equatorial_radii(self.l2.evaluate(tt_hours))?,
            tan_f1: self.tan_f1,
            tan_f2: self.tan_f2,
            derivatives,
            earth: self.earth,
            limb_model: self.limb_model,
            ephemeris: self.ephemeris.clone(),
            astrometric_evaluations: 0,
        })
    }
}

/// Global solar-eclipse classification by the shadow that reaches Earth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GlobalSolarEclipseKind {
    /// Only the lunar penumbra reaches Earth.
    Partial,
    /// The antumbra reaches Earth throughout the central path.
    Annular,
    /// The umbra reaches Earth throughout the central path.
    Total,
    /// Both antumbra and umbra reach different portions of the central path.
    Hybrid,
}

/// Annular or total character at one point on a central eclipse path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CentralSolarEclipseCharacter {
    /// The shadow section is antumbral and produces an annular eclipse.
    Annular,
    /// The shadow section is umbral and produces a total eclipse.
    Total,
}

impl CentralSolarEclipseCharacter {
    fn from_radius(radius: SolarShadowRadius) -> Self {
        if radius.is_umbral() {
            Self::Total
        } else {
            Self::Annular
        }
    }
}

/// Identity of one endpoint of the interval in which the shadow axis intersects Earth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CentralSolarEclipsePathLimitKind {
    /// First instant at which the shadow axis touches the reference ellipsoid.
    Start,
    /// Last instant at which the shadow axis touches the reference ellipsoid.
    End,
}

/// One refined tangency between the lunar shadow axis and the reference ellipsoid.
pub struct CentralSolarEclipsePathLimit<S: TimeScale> {
    kind: CentralSolarEclipsePathLimitKind,
    instant: Instant<S>,
    bracket_start: Instant<S>,
    bracket_end: Instant<S>,
    time_uncertainty: Duration,
    ellipsoid_residual: f64,
    iterations: u32,
    evaluations: u32,
}

impl<S: TimeScale> CentralSolarEclipsePathLimit<S> {
    /// Returns whether this is the start or end of the central path.
    pub const fn kind(self) -> CentralSolarEclipsePathLimitKind {
        self.kind
    }

    /// Returns the refined physical instant.
    pub const fn instant(self) -> Instant<S> {
        self.instant
    }

    /// Returns the final inclusive root bracket start.
    pub const fn bracket_start(self) -> Instant<S> {
        self.bracket_start
    }

    /// Returns the final inclusive root bracket end.
    pub const fn bracket_end(self) -> Instant<S> {
        self.bracket_end
    }

    /// Returns half the final root-bracket width.
    pub const fn time_uncertainty(self) -> Duration {
        self.time_uncertainty
    }

    /// Returns the final dimensionless ellipsoid-intersection residual.
    pub const fn ellipsoid_residual(self) -> f64 {
        self.ellipsoid_residual
    }

    /// Returns the completed Brent iterations.
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns the cumulative astrometric evaluations consumed by this eclipse search.
    pub const fn evaluations(self) -> u32 {
        self.evaluations
    }
}

impl<S: TimeScale> Copy for CentralSolarEclipsePathLimit<S> {}

impl<S: TimeScale> Clone for CentralSolarEclipsePathLimit<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for CentralSolarEclipsePathLimit<S> {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.instant == other.instant
            && self.bracket_start == other.bracket_start
            && self.bracket_end == other.bracket_end
            && self.time_uncertainty == other.time_uncertainty
            && self.ellipsoid_residual == other.ellipsoid_residual
            && self.iterations == other.iterations
            && self.evaluations == other.evaluations
    }
}

impl<S: TimeScale> core::fmt::Debug for CentralSolarEclipsePathLimit<S> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("CentralSolarEclipsePathLimit")
            .field("kind", &self.kind)
            .field("instant", &self.instant)
            .field("bracket_start", &self.bracket_start)
            .field("bracket_end", &self.bracket_end)
            .field("time_uncertainty", &self.time_uncertainty)
            .field("ellipsoid_residual", &self.ellipsoid_residual)
            .field("iterations", &self.iterations)
            .field("evaluations", &self.evaluations)
            .finish()
    }
}

/// A refined annular/total change caused by the shadow-cone vertex crossing Earth's surface.
pub struct HybridSolarEclipseTransition<S: TimeScale> {
    instant: Instant<S>,
    before: CentralSolarEclipseCharacter,
    after: CentralSolarEclipseCharacter,
    bracket_start: Instant<S>,
    bracket_end: Instant<S>,
    time_uncertainty: Duration,
    shadow_radius: SolarShadowRadius,
    iterations: u32,
    evaluations: u32,
}

impl<S: TimeScale> HybridSolarEclipseTransition<S> {
    /// Returns the refined transition instant.
    pub const fn instant(self) -> Instant<S> {
        self.instant
    }

    /// Returns the central-path character immediately before the transition.
    pub const fn before(self) -> CentralSolarEclipseCharacter {
        self.before
    }

    /// Returns the central-path character immediately after the transition.
    pub const fn after(self) -> CentralSolarEclipseCharacter {
        self.after
    }

    /// Returns the final inclusive root bracket start.
    pub const fn bracket_start(self) -> Instant<S> {
        self.bracket_start
    }

    /// Returns the final inclusive root bracket end.
    pub const fn bracket_end(self) -> Instant<S> {
        self.bracket_end
    }

    /// Returns half the final root-bracket width.
    pub const fn time_uncertainty(self) -> Duration {
        self.time_uncertainty
    }

    /// Returns the signed core-shadow radius remaining at the refined transition.
    pub const fn shadow_radius(self) -> SolarShadowRadius {
        self.shadow_radius
    }

    /// Returns the completed Brent iterations.
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns the cumulative astrometric evaluations consumed by this eclipse search.
    pub const fn evaluations(self) -> u32 {
        self.evaluations
    }
}

impl<S: TimeScale> Copy for HybridSolarEclipseTransition<S> {}

impl<S: TimeScale> Clone for HybridSolarEclipseTransition<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for HybridSolarEclipseTransition<S> {
    fn eq(&self, other: &Self) -> bool {
        self.instant == other.instant
            && self.before == other.before
            && self.after == other.after
            && self.bracket_start == other.bracket_start
            && self.bracket_end == other.bracket_end
            && self.time_uncertainty == other.time_uncertainty
            && self.shadow_radius == other.shadow_radius
            && self.iterations == other.iterations
            && self.evaluations == other.evaluations
    }
}

impl<S: TimeScale> core::fmt::Debug for HybridSolarEclipseTransition<S> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("HybridSolarEclipseTransition")
            .field("instant", &self.instant)
            .field("before", &self.before)
            .field("after", &self.after)
            .field("bracket_start", &self.bracket_start)
            .field("bracket_end", &self.bracket_end)
            .field("time_uncertainty", &self.time_uncertainty)
            .field("shadow_radius", &self.shadow_radius)
            .field("iterations", &self.iterations)
            .field("evaluations", &self.evaluations)
            .finish()
    }
}

/// Complete interval and annular/total transitions of one central solar-eclipse path.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalSolarEclipseCentralPath<S: TimeScale> {
    start: CentralSolarEclipsePathLimit<S>,
    end: CentralSolarEclipsePathLimit<S>,
    start_character: CentralSolarEclipseCharacter,
    greatest_character: CentralSolarEclipseCharacter,
    end_character: CentralSolarEclipseCharacter,
    transitions: Vec<HybridSolarEclipseTransition<S>>,
}

impl<S: TimeScale> GlobalSolarEclipseCentralPath<S> {
    /// Returns the first shadow-axis tangency with the reference ellipsoid.
    pub const fn start(&self) -> CentralSolarEclipsePathLimit<S> {
        self.start
    }

    /// Returns the last shadow-axis tangency with the reference ellipsoid.
    pub const fn end(&self) -> CentralSolarEclipsePathLimit<S> {
        self.end
    }

    /// Returns the annular or total character immediately inside the start of the central path.
    pub const fn start_character(&self) -> CentralSolarEclipseCharacter {
        self.start_character
    }

    /// Returns the annular or total character at greatest eclipse.
    pub const fn greatest_character(&self) -> CentralSolarEclipseCharacter {
        self.greatest_character
    }

    /// Returns the annular or total character immediately inside the end of the central path.
    pub const fn end_character(&self) -> CentralSolarEclipseCharacter {
        self.end_character
    }

    /// Returns every refined annular/total transition in chronological order.
    pub fn transitions(&self) -> &[HybridSolarEclipseTransition<S>] {
        &self.transitions
    }
}

/// Refined global greatest-eclipse geometry and numerical evidence.
pub struct GlobalSolarEclipseMaximum<S: TimeScale> {
    instant: Instant<S>,
    gamma: SolarEclipseGamma,
    shadow_axis_distance: Length,
    penumbra_intersects_earth: bool,
    umbra_intersects_earth: bool,
    antumbra_intersects_earth: bool,
    geometric_core_shadow_radius_at_axis_plane: SolarShadowRadius,
    geometric_core_shadow_radius_at_surface: Option<SolarShadowRadius>,
    evidence: ExtremumEvidence<S>,
}

impl<S: TimeScale> GlobalSolarEclipseMaximum<S> {
    /// Returns the greatest-eclipse instant.
    pub const fn instant(self) -> Instant<S> {
        self.instant
    }

    /// Returns the signed gamma parameter at greatest eclipse.
    pub const fn gamma(self) -> SolarEclipseGamma {
        self.gamma
    }

    /// Returns the minimum geocentric distance to the lunar shadow axis.
    pub const fn shadow_axis_distance(self) -> Length {
        self.shadow_axis_distance
    }

    /// Returns whether the exact penumbral cone intersects the reference ellipsoid.
    pub const fn penumbra_intersects_earth(self) -> bool {
        self.penumbra_intersects_earth
    }

    /// Returns whether the exact umbral cone intersects the reference ellipsoid.
    pub const fn umbra_intersects_earth(self) -> bool {
        self.umbra_intersects_earth
    }

    /// Returns whether the exact antumbral cone intersects the reference ellipsoid.
    pub const fn antumbra_intersects_earth(self) -> bool {
        self.antumbra_intersects_earth
    }

    /// Returns the physical common-tangent core-shadow radius in the axis-normal plane nearest
    /// the geocentre.
    ///
    /// This uses the [`SolarEclipseModel`] selected for global classification. It is independent
    /// of any empirical [`BesselianLimbModel`] later selected for contact or path calculations.
    pub const fn geometric_core_shadow_radius_at_axis_plane(self) -> SolarShadowRadius {
        self.geometric_core_shadow_radius_at_axis_plane
    }

    /// Returns the physical common-tangent core-shadow radius where the axis first meets Earth's
    /// surface.
    ///
    /// This is absent when the shadow axis misses the selected reference ellipsoid.
    pub const fn geometric_core_shadow_radius_at_surface(self) -> Option<SolarShadowRadius> {
        self.geometric_core_shadow_radius_at_surface
    }

    /// Returns the bounded-minimum evidence retained for greatest eclipse.
    pub const fn evidence(self) -> ExtremumEvidence<S> {
        self.evidence
    }
}

impl<S: TimeScale> Copy for GlobalSolarEclipseMaximum<S> {}

impl<S: TimeScale> Clone for GlobalSolarEclipseMaximum<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for GlobalSolarEclipseMaximum<S> {
    fn eq(&self, other: &Self) -> bool {
        self.instant == other.instant
            && self.gamma == other.gamma
            && self.shadow_axis_distance == other.shadow_axis_distance
            && self.penumbra_intersects_earth == other.penumbra_intersects_earth
            && self.umbra_intersects_earth == other.umbra_intersects_earth
            && self.antumbra_intersects_earth == other.antumbra_intersects_earth
            && self.geometric_core_shadow_radius_at_axis_plane
                == other.geometric_core_shadow_radius_at_axis_plane
            && self.geometric_core_shadow_radius_at_surface
                == other.geometric_core_shadow_radius_at_surface
            && self.evidence == other.evidence
    }
}

impl<S: TimeScale> core::fmt::Debug for GlobalSolarEclipseMaximum<S> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("GlobalSolarEclipseMaximum")
            .field("instant", &self.instant)
            .field("gamma", &self.gamma)
            .field("shadow_axis_distance", &self.shadow_axis_distance)
            .field("penumbra_intersects_earth", &self.penumbra_intersects_earth)
            .field("umbra_intersects_earth", &self.umbra_intersects_earth)
            .field("antumbra_intersects_earth", &self.antumbra_intersects_earth)
            .field(
                "geometric_core_shadow_radius_at_axis_plane",
                &self.geometric_core_shadow_radius_at_axis_plane,
            )
            .field(
                "geometric_core_shadow_radius_at_surface",
                &self.geometric_core_shadow_radius_at_surface,
            )
            .field("evidence", &self.evidence)
            .finish()
    }
}

/// One globally classified solar eclipse whose greatest instant lies in a requested interval.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalSolarEclipse<S: TimeScale> {
    kind: GlobalSolarEclipseKind,
    maximum: GlobalSolarEclipseMaximum<S>,
    central_path: Option<GlobalSolarEclipseCentralPath<S>>,
    earth: Earth,
    model: SolarEclipseModel,
    ephemeris: EphemerisProvenance,
}

impl<S: TimeScale> GlobalSolarEclipse<S> {
    /// Returns the global penumbral, antumbral, umbral, or hybrid classification.
    pub const fn kind(&self) -> GlobalSolarEclipseKind {
        self.kind
    }

    /// Returns the refined greatest-eclipse geometry.
    pub const fn maximum(&self) -> GlobalSolarEclipseMaximum<S> {
        self.maximum
    }

    /// Returns central-path limits and hybrid transitions when the shadow axis crosses Earth.
    pub const fn central_path(&self) -> Option<&GlobalSolarEclipseCentralPath<S>> {
        self.central_path.as_ref()
    }

    /// Returns whether the central shadow reaches Earth without its axis intersecting the ellipsoid.
    pub const fn is_non_central(&self) -> bool {
        !matches!(self.kind, GlobalSolarEclipseKind::Partial) && self.central_path.is_none()
    }

    /// Returns the selected Earth reference ellipsoid.
    pub const fn earth(&self) -> Earth {
        self.earth
    }

    /// Returns the selected spherical Sun and Moon figures.
    pub const fn model(&self) -> SolarEclipseModel {
        self.model
    }

    /// Returns the ephemeris model and data provenance.
    pub const fn ephemeris(&self) -> &EphemerisProvenance {
        &self.ephemeris
    }
}

#[derive(Clone, Copy)]
struct ShadowVector([f64; 3]);

impl ShadowVector {
    const fn new(components: [f64; 3]) -> Self {
        Self(components)
    }

    const fn component(self, index: usize) -> f64 {
        self.0[index]
    }

    fn plus(self, other: Self) -> Self {
        Self([
            self.0[0] + other.0[0],
            self.0[1] + other.0[1],
            self.0[2] + other.0[2],
        ])
    }

    fn minus(self, other: Self) -> Self {
        Self([
            self.0[0] - other.0[0],
            self.0[1] - other.0[1],
            self.0[2] - other.0[2],
        ])
    }

    fn scaled(self, factor: f64) -> Self {
        Self([self.0[0] * factor, self.0[1] * factor, self.0[2] * factor])
    }

    fn dot(self, other: Self) -> f64 {
        self.0[0] * other.0[0] + self.0[1] * other.0[1] + self.0[2] * other.0[2]
    }

    fn norm(self) -> f64 {
        sqrt(self.dot(self))
    }

    fn cross(self, other: Self) -> Self {
        Self([
            self.0[1] * other.0[2] - self.0[2] * other.0[1],
            self.0[2] * other.0[0] - self.0[0] * other.0[2],
            self.0[0] * other.0[1] - self.0[1] * other.0[0],
        ])
    }

    fn normalized(self) -> Self {
        self.scaled(1.0 / self.norm())
    }
}

#[derive(Clone, Copy)]
enum ShadowNappe {
    Upstream,
    Downstream,
}

#[derive(Clone, Copy)]
struct ShadowCone {
    vertex: ShadowVector,
    axis: ShadowVector,
    radial_first: ShadowVector,
    radial_second: ShadowVector,
    half_angle_sine: f64,
    half_angle_cosine: f64,
    inverse_equatorial_squared: f64,
    inverse_polar_squared: f64,
}

impl ShadowCone {
    const AZIMUTH_SAMPLES: usize = 360;
    const GOLDEN_RATIO_COMPLEMENT: f64 = 0.618_033_988_749_894_9;
    const MAX_REFINEMENT_ITERATIONS: usize = 48;

    fn new(
        vertex: ShadowVector,
        axis: ShadowVector,
        half_angle: f64,
        earth: ReferenceEllipsoid,
    ) -> Self {
        let reference = if axis.component(2).abs() < 0.9 {
            ShadowVector::new([0.0, 0.0, 1.0])
        } else {
            ShadowVector::new([1.0, 0.0, 0.0])
        };
        let radial_first = axis.cross(reference).normalized();
        let radial_second = axis.cross(radial_first);
        let equatorial_radius = earth.semi_major_axis().as_metres();
        let polar_radius = earth.semi_minor_axis().as_metres();
        Self {
            vertex,
            axis,
            radial_first,
            radial_second,
            half_angle_sine: sin(half_angle),
            half_angle_cosine: cos(half_angle),
            inverse_equatorial_squared: 1.0 / (equatorial_radius * equatorial_radius),
            inverse_polar_squared: 1.0 / (polar_radius * polar_radius),
        }
    }

    fn intersects_earth(self, nappe: ShadowNappe) -> bool {
        self.maximum_ray_chord_squared(nappe) >= 0.0
    }

    fn maximum_ray_chord_squared(self, nappe: ShadowNappe) -> f64 {
        let step = TAU / Self::AZIMUTH_SAMPLES as f64;
        let mut best_index = 0_usize;
        let mut best_value = f64::NEG_INFINITY;
        for index in 0..Self::AZIMUTH_SAMPLES {
            let value = self.ray_chord_squared(index as f64 * step, nappe);
            if value > best_value {
                best_index = index;
                best_value = value;
            }
        }

        let mut lower = (best_index as f64 - 1.0) * step;
        let mut upper = (best_index as f64 + 1.0) * step;
        let mut first = upper - Self::GOLDEN_RATIO_COMPLEMENT * (upper - lower);
        let mut second = lower + Self::GOLDEN_RATIO_COMPLEMENT * (upper - lower);
        let mut first_value = self.ray_chord_squared(first, nappe);
        let mut second_value = self.ray_chord_squared(second, nappe);
        for _ in 0..Self::MAX_REFINEMENT_ITERATIONS {
            if first_value < second_value {
                lower = first;
                first = second;
                first_value = second_value;
                second = lower + Self::GOLDEN_RATIO_COMPLEMENT * (upper - lower);
                second_value = self.ray_chord_squared(second, nappe);
            } else {
                upper = second;
                second = first;
                second_value = first_value;
                first = upper - Self::GOLDEN_RATIO_COMPLEMENT * (upper - lower);
                first_value = self.ray_chord_squared(first, nappe);
            }
        }
        best_value.max(first_value).max(second_value)
    }

    fn ray_chord_squared(self, azimuth: f64, nappe: ShadowNappe) -> f64 {
        let axial_factor = match nappe {
            ShadowNappe::Upstream => -self.half_angle_cosine,
            ShadowNappe::Downstream => self.half_angle_cosine,
        };
        let direction = self
            .axis
            .scaled(axial_factor)
            .plus(
                self.radial_first
                    .scaled(self.half_angle_sine * cos(azimuth)),
            )
            .plus(
                self.radial_second
                    .scaled(self.half_angle_sine * sin(azimuth)),
            );
        let quadratic = self.ellipsoid_quadratic(direction);
        let linear = self.ellipsoid_bilinear(self.vertex, direction);
        let constant = self.ellipsoid_quadratic(self.vertex) - 1.0;
        let discriminant = linear * linear - quadratic * constant;
        if discriminant < 0.0 {
            return discriminant / (quadratic * quadratic);
        }
        let far_intersection = (-linear + sqrt(discriminant)) / quadratic;
        if far_intersection < 0.0 {
            f64::NEG_INFINITY
        } else {
            discriminant / (quadratic * quadratic)
        }
    }

    fn ellipsoid_quadratic(self, vector: ShadowVector) -> f64 {
        (vector.component(0) * vector.component(0) + vector.component(1) * vector.component(1))
            * self.inverse_equatorial_squared
            + vector.component(2) * vector.component(2) * self.inverse_polar_squared
    }

    fn ellipsoid_bilinear(self, left: ShadowVector, right: ShadowVector) -> f64 {
        (left.component(0) * right.component(0) + left.component(1) * right.component(1))
            * self.inverse_equatorial_squared
            + left.component(2) * right.component(2) * self.inverse_polar_squared
    }
}

#[derive(Clone, Copy)]
struct ShadowAxisGeometry<S: TimeScale> {
    epoch: Instant<S>,
    axis_offset: ShadowVector,
    axis_distance_metres: f64,
    moon_position: ShadowVector,
    axis: ShadowVector,
    sun_moon_distance: f64,
    earth: ReferenceEllipsoid,
}

struct ShadowGeometry<S: TimeScale> {
    epoch: Instant<S>,
    axis_offset: ShadowVector,
    axis_distance_metres: f64,
    moon_position: ShadowVector,
    axis: ShadowVector,
    umbral_angle: f64,
    penumbral_angle: f64,
    umbral_vertex_distance: f64,
    penumbral_vertex_distance: f64,
    core_radius_axis_plane_metres: f64,
    ellipsoid_centrality: f64,
    near_surface_core_radius_metres: Option<f64>,
    earth: ReferenceEllipsoid,
}

impl<S: TimeScale> Copy for ShadowGeometry<S> {}

impl<S: TimeScale> Clone for ShadowGeometry<S> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Clone, Copy)]
struct BesselianSample<S: TimeScale> {
    plane: BesselianFundamentalPlane<S>,
    x: f64,
    y: f64,
    mu: HourAngle,
    l1: f64,
    l2: f64,
    tan_f1: f64,
    tan_f2: f64,
}

struct BesselianComputation {
    time_step: Duration,
    earth: Earth,
    limb_model: BesselianLimbModel,
    ephemeris: EphemerisProvenance,
    astrometric_evaluations: u32,
}

struct BesselianPolynomialComputation {
    earth: Earth,
    limb_model: BesselianLimbModel,
    ephemeris: EphemerisProvenance,
    astrometric_evaluations: u32,
}

struct BesselianSampler<'context, 'data, 'earth, E, P: EphemerisProvider + ?Sized> {
    astrometry: Astrometry<'context, 'data, E, P>,
    earth: &'earth Earth,
    options: BesselianElementsOptions,
}

impl<'context, 'data, 'earth, E, P: EphemerisProvider + ?Sized>
    BesselianSampler<'context, 'data, 'earth, E, P>
{
    const fn new(
        astrometry: Astrometry<'context, 'data, E, P>,
        earth: &'earth Earth,
        options: BesselianElementsOptions,
    ) -> Self {
        Self {
            astrometry,
            earth,
            options,
        }
    }

    fn corrected_lunar_direction<S: TimeScale>(
        &self,
        moon: GeocentricApparentPlace<S>,
    ) -> Result<EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>, Error> {
        let limb_model = self.options.limb_model();
        let latitude_correction = limb_model.lunar_latitude_correction().as_radians();
        let longitude_correction = limb_model.lunar_longitude_correction().as_radians();
        if latitude_correction == 0.0 && longitude_correction == 0.0 {
            return Ok(moon.true_equatorial());
        }

        let coordinates = moon.true_ecliptic().coordinates();
        let corrected = EclipticDirectionAt::<TrueEclipticEquinoxOfDate, S>::new(
            moon.reception_epoch(),
            EclipticDirection::new(
                EclipticLongitude::wrap_radians(
                    coordinates.longitude().as_radians() + longitude_correction,
                )?,
                EclipticLatitude::try_from_radians(
                    coordinates.latitude().as_radians() + latitude_correction,
                )?,
            ),
        );
        let celestial = Frames::new(self.astrometry.time_context())
            .celestial_orientation_at(moon.reception_epoch())
            .map_err(crate::astro::Error::from)?;
        let corrected_gcrs = celestial
            .gcrs_from_true_ecliptic(corrected)
            .map_err(crate::astro::Error::from)?;
        celestial
            .true_equatorial(corrected_gcrs)
            .map_err(crate::astro::Error::from)
            .map_err(Error::from)
    }

    fn sample<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<BesselianSample<S>, Error> {
        if maximum_evaluations.saturating_sub(*evaluations) < 2 {
            return Err(Error::EvaluationLimitExceeded {
                maximum: maximum_evaluations,
            });
        }
        *evaluations += 2;
        let sun = self
            .astrometry
            .solar_apparent_place(epoch, self.options.light_time())?;
        let moon = self.astrometry.geocentric_apparent_place(
            CelestialBody::Moon,
            epoch,
            self.options.light_time(),
        )?;
        let lunar_direction = self.corrected_lunar_direction(moon)?;
        let geometry = ShadowAxisGeometry::from_apparent_places(
            sun,
            moon,
            lunar_direction,
            self.earth.reference_ellipsoid(),
        )?;
        let terrestrial_time =
            JulianDate::<Tt>::from_instant(epoch, self.astrometry.time_context())?;
        let (tt_first, tt_second) = terrestrial_time.parts();
        let ephemeris_sidereal_time = HourAngle::wrap_radians(sofars::erst::gst06a(
            tt_first, tt_second, tt_first, tt_second,
        ))?;
        BesselianSample::from_axis_geometry(
            geometry,
            ephemeris_sidereal_time,
            self.options.limb_model(),
        )
    }
}

struct GlobalSolarEclipseSampler<'context, 'data, 'earth, E, P: EphemerisProvider + ?Sized> {
    astrometry: Astrometry<'context, 'data, E, P>,
    earth: &'earth Earth,
    model: SolarEclipseModel,
    light_time: ReceptionLightTimeOptions,
}

impl<'context, 'data, 'earth, E, P: EphemerisProvider + ?Sized>
    GlobalSolarEclipseSampler<'context, 'data, 'earth, E, P>
{
    const MAXIMUM_HALF_WINDOW: Duration = Duration::from_nanoseconds(12 * 3_600_000_000_000);
    const CENTRAL_LIMIT_SCAN_STEP: Duration = Duration::from_nanoseconds(600_000_000_000);
    const CENTRAL_INTERIOR_OFFSET: Duration = Duration::from_nanoseconds(1_000_000_000);
    const MAX_CENTRAL_LIMIT_STEPS: usize = 72;

    const fn new(
        astrometry: Astrometry<'context, 'data, E, P>,
        earth: &'earth Earth,
        options: SolarEclipseSearchOptions,
    ) -> Self {
        Self {
            astrometry,
            earth,
            model: options.model(),
            light_time: options.angular_search().light_time(),
        }
    }

    fn consume_geometry(evaluations: &mut u32, maximum: u32) -> Result<(), Error> {
        if maximum.saturating_sub(*evaluations) < 2 {
            return Err(Error::EvaluationLimitExceeded { maximum });
        }
        *evaluations += 2;
        Ok(())
    }

    fn geometry<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<ShadowGeometry<S>, Error> {
        Self::consume_geometry(evaluations, maximum_evaluations)?;
        let sun = self
            .astrometry
            .solar_apparent_place(epoch, self.light_time)?;
        let moon = self.astrometry.geocentric_apparent_place(
            CelestialBody::Moon,
            epoch,
            self.light_time,
        )?;
        let axis_geometry = ShadowAxisGeometry::from_apparent_places(
            sun,
            moon,
            moon.true_equatorial(),
            self.earth.reference_ellipsoid(),
        )?;
        ShadowGeometry::from_axis_geometry(axis_geometry, self.model)
    }

    fn maximum_near<S: TimeScale>(
        &self,
        seed: Instant<S>,
        options: SolarEclipseSearchOptions,
    ) -> Result<(GlobalSolarEclipseMaximum<S>, ShadowGeometry<S>, u32), Error> {
        let controls = options.angular_search();
        let start = seed.checked_sub(Self::MAXIMUM_HALF_WINDOW)?;
        let end = seed.checked_add(Self::MAXIMUM_HALF_WINDOW)?;
        let mut evaluations = 0_u32;
        let refined = BracketedExtremumSearch::refine_minimum(
            start,
            seed,
            end,
            controls.time_tolerance(),
            controls.max_refinement_iterations(),
            |epoch| {
                self.geometry(epoch, &mut evaluations, controls.max_evaluations())
                    .map(|geometry| geometry.axis_distance_metres)
            },
        )?;
        let geometry = self.geometry(
            refined.instant(),
            &mut evaluations,
            controls.max_evaluations(),
        )?;
        let maximum = GlobalSolarEclipseMaximum::from_geometry(
            geometry,
            self.earth.reference_ellipsoid(),
            ExtremumEvidence::new(
                refined.bracket_start(),
                refined.bracket_end(),
                refined.time_uncertainty(),
                refined.iterations(),
                evaluations,
            ),
        )?;
        Ok((maximum, geometry, evaluations))
    }

    fn bracket_central_limit<S: TimeScale>(
        &self,
        maximum: ShadowGeometry<S>,
        before: bool,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<(Instant<S>, Instant<S>), Error> {
        let mut inner = maximum.epoch;
        for _ in 0..Self::MAX_CENTRAL_LIMIT_STEPS {
            let outer = if before {
                inner.checked_sub(Self::CENTRAL_LIMIT_SCAN_STEP)?
            } else {
                inner.checked_add(Self::CENTRAL_LIMIT_SCAN_STEP)?
            };
            let geometry = self.geometry(outer, evaluations, maximum_evaluations)?;
            if geometry.ellipsoid_centrality >= 0.0 {
                return if before {
                    Ok((outer, inner))
                } else {
                    Ok((inner, outer))
                };
            }
            inner = outer;
        }
        Err(Error::GlobalSolarEclipsePathLimitNotBracketed {
            limit: if before { "start" } else { "end" },
            maximum_tai_nanoseconds: maximum.epoch.tai_nanoseconds_since_1900(),
        })
    }

    fn refine_central_limit<S: TimeScale>(
        &self,
        maximum: ShadowGeometry<S>,
        kind: CentralSolarEclipsePathLimitKind,
        options: SolarEclipseSearchOptions,
        evaluations: &mut u32,
    ) -> Result<CentralSolarEclipsePathLimit<S>, Error> {
        let controls = options.angular_search();
        let (bracket_start, bracket_end) = self.bracket_central_limit(
            maximum,
            matches!(kind, CentralSolarEclipsePathLimitKind::Start),
            evaluations,
            controls.max_evaluations(),
        )?;
        let root = BracketedRootSearch::refine(
            bracket_start,
            bracket_end,
            controls.time_tolerance(),
            controls.max_refinement_iterations(),
            |epoch| {
                self.geometry(epoch, evaluations, controls.max_evaluations())
                    .map(|geometry| geometry.ellipsoid_centrality)
            },
        )?;
        let geometry = self.geometry(root.instant(), evaluations, controls.max_evaluations())?;
        Ok(CentralSolarEclipsePathLimit {
            kind,
            instant: root.instant(),
            bracket_start: root.bracket_start(),
            bracket_end: root.bracket_end(),
            time_uncertainty: root.time_uncertainty(),
            ellipsoid_residual: geometry.ellipsoid_centrality,
            iterations: root.iterations(),
            evaluations: *evaluations,
        })
    }

    fn central_radius<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        evaluations: &mut u32,
        maximum_evaluations: u32,
    ) -> Result<f64, Error> {
        self.geometry(epoch, evaluations, maximum_evaluations)?
            .near_surface_core_radius_metres
            .ok_or(Error::ShadowAxisDoesNotIntersectEarth {
                epoch_tai_nanoseconds: epoch.tai_nanoseconds_since_1900(),
            })
    }

    fn refine_transition<S: TimeScale>(
        &self,
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        before: CentralSolarEclipseCharacter,
        after: CentralSolarEclipseCharacter,
        options: SolarEclipseSearchOptions,
        evaluations: &mut u32,
    ) -> Result<HybridSolarEclipseTransition<S>, Error> {
        let controls = options.angular_search();
        let root = BracketedRootSearch::refine(
            bracket_start,
            bracket_end,
            controls.time_tolerance(),
            controls.max_refinement_iterations(),
            |epoch| self.central_radius(epoch, evaluations, controls.max_evaluations()),
        )?;
        let radius = SolarShadowRadius::from_metres(self.central_radius(
            root.instant(),
            evaluations,
            controls.max_evaluations(),
        )?)?;
        Ok(HybridSolarEclipseTransition {
            instant: root.instant(),
            before,
            after,
            bracket_start: root.bracket_start(),
            bracket_end: root.bracket_end(),
            time_uncertainty: root.time_uncertainty(),
            shadow_radius: radius,
            iterations: root.iterations(),
            evaluations: *evaluations,
        })
    }

    fn central_path<S: TimeScale>(
        &self,
        maximum_geometry: ShadowGeometry<S>,
        maximum: GlobalSolarEclipseMaximum<S>,
        options: SolarEclipseSearchOptions,
        evaluations: &mut u32,
    ) -> Result<GlobalSolarEclipseCentralPath<S>, Error> {
        let controls = options.angular_search();
        let start = self.refine_central_limit(
            maximum_geometry,
            CentralSolarEclipsePathLimitKind::Start,
            options,
            evaluations,
        )?;
        let end = self.refine_central_limit(
            maximum_geometry,
            CentralSolarEclipsePathLimitKind::End,
            options,
            evaluations,
        )?;

        let start_sample = start.instant().checked_add(Self::CENTRAL_INTERIOR_OFFSET)?;
        let end_sample = end.instant().checked_sub(Self::CENTRAL_INTERIOR_OFFSET)?;
        let start_radius = SolarShadowRadius::from_metres(self.central_radius(
            start_sample,
            evaluations,
            controls.max_evaluations(),
        )?)?;
        let maximum_radius = maximum.geometric_core_shadow_radius_at_surface().ok_or(
            Error::ShadowAxisDoesNotIntersectEarth {
                epoch_tai_nanoseconds: maximum.instant().tai_nanoseconds_since_1900(),
            },
        )?;
        let end_radius = SolarShadowRadius::from_metres(self.central_radius(
            end_sample,
            evaluations,
            controls.max_evaluations(),
        )?)?;

        let peak = BracketedExtremumSearch::refine_minimum(
            start_sample,
            maximum.instant(),
            end_sample,
            controls.time_tolerance(),
            controls.max_refinement_iterations(),
            |epoch| {
                self.central_radius(epoch, evaluations, controls.max_evaluations())
                    .map(|radius| -radius)
            },
        )?;
        let peak_radius = SolarShadowRadius::from_metres(self.central_radius(
            peak.instant(),
            evaluations,
            controls.max_evaluations(),
        )?)?;
        let samples = [
            (start_sample, start_radius),
            (peak.instant(), peak_radius),
            (end_sample, end_radius),
        ];

        let mut transitions = Vec::new();
        for pair in samples.windows(2) {
            let (left_epoch, left_radius) = pair[0];
            let (right_epoch, right_radius) = pair[1];
            let left_character = CentralSolarEclipseCharacter::from_radius(left_radius);
            let right_character = CentralSolarEclipseCharacter::from_radius(right_radius);
            if left_character == right_character {
                continue;
            }
            transitions.push(self.refine_transition(
                left_epoch,
                right_epoch,
                left_character,
                right_character,
                options,
                evaluations,
            )?);
        }

        Ok(GlobalSolarEclipseCentralPath {
            start,
            end,
            start_character: CentralSolarEclipseCharacter::from_radius(start_radius),
            greatest_character: CentralSolarEclipseCharacter::from_radius(maximum_radius),
            end_character: CentralSolarEclipseCharacter::from_radius(end_radius),
            transitions,
        })
    }
}

impl<S: TimeScale> ShadowAxisGeometry<S> {
    fn from_apparent_places(
        sun: SolarApparentPlace<S>,
        moon: GeocentricApparentPlace<S>,
        lunar_direction: EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>,
        earth: ReferenceEllipsoid,
    ) -> Result<Self, Error> {
        let sun_direction = ShadowVector::new(
            sun.true_equatorial()
                .coordinates()
                .to_direction()?
                .components(),
        );
        let moon_direction =
            ShadowVector::new(lunar_direction.coordinates().to_direction()?.components());
        let sun_position = sun_direction.scaled(sun.distance().as_metres());
        let moon_position = moon_direction.scaled(moon.distance().as_metres());
        let sun_to_moon = moon_position.minus(sun_position);
        let sun_moon_distance = sun_to_moon.norm();
        let axis = sun_to_moon.scaled(1.0 / sun_moon_distance);
        let closest_axis_distance = -moon_position.dot(axis);
        let axis_offset = moon_position.plus(axis.scaled(closest_axis_distance));

        Ok(Self {
            epoch: sun.reception_epoch(),
            axis_offset,
            axis_distance_metres: axis_offset.norm(),
            moon_position,
            axis,
            sun_moon_distance,
            earth,
        })
    }
}

impl<S: TimeScale> ShadowGeometry<S> {
    const CENTRALITY_ROUNDOFF: f64 = 1.0e-10;

    fn from_axis_geometry(
        geometry: ShadowAxisGeometry<S>,
        model: SolarEclipseModel,
    ) -> Result<Self, Error> {
        let sun_radius = model.sun().radius().as_metres();
        let moon_radius = model.moon().radius().as_metres();
        let umbral_angle =
            asin(((sun_radius - moon_radius) / geometry.sun_moon_distance).clamp(-1.0, 1.0));
        let penumbral_angle =
            asin(((sun_radius + moon_radius) / geometry.sun_moon_distance).clamp(-1.0, 1.0));
        let umbral_tangent = umbral_angle.tan();
        let umbral_vertex_distance = moon_radius / umbral_angle.sin();
        let penumbral_vertex_distance = moon_radius / penumbral_angle.sin();
        let closest_axis_distance = -geometry.moon_position.dot(geometry.axis);
        let core_radius_axis_plane_metres =
            (umbral_vertex_distance - closest_axis_distance) * umbral_tangent;

        let equatorial_radius = geometry.earth.semi_major_axis().as_metres();
        let polar_radius = geometry.earth.semi_minor_axis().as_metres();

        let inverse_equatorial_squared = 1.0 / (equatorial_radius * equatorial_radius);
        let inverse_polar_squared = 1.0 / (polar_radius * polar_radius);
        let quadratic_axis = geometry.axis.component(0)
            * geometry.axis.component(0)
            * inverse_equatorial_squared
            + geometry.axis.component(1) * geometry.axis.component(1) * inverse_equatorial_squared
            + geometry.axis.component(2) * geometry.axis.component(2) * inverse_polar_squared;
        let quadratic_linear = geometry.moon_position.component(0)
            * geometry.axis.component(0)
            * inverse_equatorial_squared
            + geometry.moon_position.component(1)
                * geometry.axis.component(1)
                * inverse_equatorial_squared
            + geometry.moon_position.component(2)
                * geometry.axis.component(2)
                * inverse_polar_squared;
        let ellipsoid_closest_distance = -quadratic_linear / quadratic_axis;
        let ellipsoid_closest = geometry
            .moon_position
            .plus(geometry.axis.scaled(ellipsoid_closest_distance));
        let ellipsoid_centrality = ellipsoid_closest.component(0)
            * ellipsoid_closest.component(0)
            * inverse_equatorial_squared
            + ellipsoid_closest.component(1)
                * ellipsoid_closest.component(1)
                * inverse_equatorial_squared
            + ellipsoid_closest.component(2)
                * ellipsoid_closest.component(2)
                * inverse_polar_squared
            - 1.0;
        let near_surface_core_radius_metres = if ellipsoid_centrality <= Self::CENTRALITY_ROUNDOFF {
            let half_chord = sqrt((-ellipsoid_centrality / quadratic_axis).max(0.0));
            let near_surface_distance = ellipsoid_closest_distance - half_chord;
            Some((umbral_vertex_distance - near_surface_distance) * umbral_tangent)
        } else {
            None
        };

        Ok(Self {
            epoch: geometry.epoch,
            axis_offset: geometry.axis_offset,
            axis_distance_metres: geometry.axis_distance_metres,
            moon_position: geometry.moon_position,
            axis: geometry.axis,
            umbral_angle,
            penumbral_angle,
            umbral_vertex_distance,
            penumbral_vertex_distance,
            core_radius_axis_plane_metres,
            ellipsoid_centrality,
            near_surface_core_radius_metres,
            earth: geometry.earth,
        })
    }
}

impl<S: TimeScale> BesselianSample<S> {
    fn from_axis_geometry(
        geometry: ShadowAxisGeometry<S>,
        ephemeris_sidereal_time: HourAngle,
        limb_model: BesselianLimbModel,
    ) -> Result<Self, Error> {
        let shadow_axis = geometry.axis.scaled(-1.0);
        let right_ascension = RightAscension::wrap_radians(atan2(
            shadow_axis.component(1),
            shadow_axis.component(0),
        ))?;
        let declination =
            Declination::try_from_radians(asin(shadow_axis.component(2).clamp(-1.0, 1.0)))?;
        let right_ascension_radians = right_ascension.as_radians();
        let declination_radians = declination.as_radians();
        let x_axis = ShadowVector::new([
            -sin(right_ascension_radians),
            cos(right_ascension_radians),
            0.0,
        ]);
        let y_axis = ShadowVector::new([
            -sin(declination_radians) * cos(right_ascension_radians),
            -sin(declination_radians) * sin(right_ascension_radians),
            cos(declination_radians),
        ]);
        let equatorial_radius = geometry.earth.semi_major_axis().as_metres();
        let penumbral_lunar_radius =
            limb_model.penumbral_lunar_radius().as_equatorial_radii() * equatorial_radius;
        let umbral_lunar_radius =
            limb_model.umbral_lunar_radius().as_equatorial_radii() * equatorial_radius;
        let solar_radius = limb_model.solar_radius().as_metres();
        if solar_radius <= umbral_lunar_radius {
            return Err(Error::InvalidBesselianLimbModelValue {
                field: "solar radius minus umbral lunar radius in metres",
                value: solar_radius - umbral_lunar_radius,
            });
        }
        let penumbral_angle = asin(
            ((solar_radius + penumbral_lunar_radius) / geometry.sun_moon_distance).clamp(-1.0, 1.0),
        );
        let umbral_angle = asin(
            ((solar_radius - umbral_lunar_radius) / geometry.sun_moon_distance).clamp(-1.0, 1.0),
        );
        let tan_f1 = penumbral_angle.tan();
        let tan_f2 = umbral_angle.tan();
        let penumbral_vertex_distance = penumbral_lunar_radius / penumbral_angle.sin();
        let umbral_vertex_distance = umbral_lunar_radius / umbral_angle.sin();
        let moon_distance_along_axis = geometry.moon_position.dot(shadow_axis);
        let l1 =
            (moon_distance_along_axis + penumbral_vertex_distance) * tan_f1 / equatorial_radius;
        let l2 = (moon_distance_along_axis - umbral_vertex_distance) * tan_f2 / equatorial_radius;
        let axis_coordinates = EquatorialDirection::new(right_ascension, declination);

        Ok(Self {
            plane: BesselianFundamentalPlane {
                shadow_axis: EquatorialDirectionAt::new(geometry.epoch, axis_coordinates),
            },
            x: geometry.axis_offset.dot(x_axis) / equatorial_radius,
            y: geometry.axis_offset.dot(y_axis) / equatorial_radius,
            mu: HourAngle::wrap_radians(
                ephemeris_sidereal_time.as_radians() - right_ascension.as_radians(),
            )?,
            l1,
            l2,
            tan_f1,
            tan_f2,
        })
    }
}

struct BesselianPolynomialFit;

impl BesselianPolynomialFit {
    const SAMPLE_COUNT: usize = 5;

    fn cubic(hours: [f64; Self::SAMPLE_COUNT], values: [f64; Self::SAMPLE_COUNT]) -> [f64; 4] {
        let mut count = 0.0;
        let mut second_moment = 0.0;
        let mut fourth_moment = 0.0;
        let mut sixth_moment = 0.0;
        let mut value_sum = 0.0;
        let mut time_value_sum = 0.0;
        let mut time_squared_value_sum = 0.0;
        let mut time_cubed_value_sum = 0.0;
        for (time, value) in hours.into_iter().zip(values) {
            let time_squared = time * time;
            count += 1.0;
            second_moment += time_squared;
            fourth_moment += time_squared * time_squared;
            sixth_moment += time_squared * time_squared * time_squared;
            value_sum += value;
            time_value_sum += time * value;
            time_squared_value_sum += time_squared * value;
            time_cubed_value_sum += time_squared * time * value;
        }
        let even_determinant = count * fourth_moment - second_moment * second_moment;
        let odd_determinant = second_moment * sixth_moment - fourth_moment * fourth_moment;
        [
            (fourth_moment * value_sum - second_moment * time_squared_value_sum) / even_determinant,
            (sixth_moment * time_value_sum - fourth_moment * time_cubed_value_sum)
                / odd_determinant,
            (count * time_squared_value_sum - second_moment * value_sum) / even_determinant,
            (second_moment * time_cubed_value_sum - fourth_moment * time_value_sum)
                / odd_determinant,
        ]
    }

    fn quadratic(hours: [f64; Self::SAMPLE_COUNT], values: [f64; Self::SAMPLE_COUNT]) -> [f64; 3] {
        let mut count = 0.0;
        let mut second_moment = 0.0;
        let mut fourth_moment = 0.0;
        let mut value_sum = 0.0;
        let mut time_value_sum = 0.0;
        let mut time_squared_value_sum = 0.0;
        for (time, value) in hours.into_iter().zip(values) {
            let time_squared = time * time;
            count += 1.0;
            second_moment += time_squared;
            fourth_moment += time_squared * time_squared;
            value_sum += value;
            time_value_sum += time * value;
            time_squared_value_sum += time_squared * value;
        }
        let even_determinant = count * fourth_moment - second_moment * second_moment;
        [
            (fourth_moment * value_sum - second_moment * time_squared_value_sum) / even_determinant,
            time_value_sum / second_moment,
            (count * time_squared_value_sum - second_moment * value_sum) / even_determinant,
        ]
    }

    fn linear(hours: [f64; Self::SAMPLE_COUNT], values: [f64; Self::SAMPLE_COUNT]) -> [f64; 2] {
        let mut count = 0.0;
        let mut second_moment = 0.0;
        let mut value_sum = 0.0;
        let mut time_value_sum = 0.0;
        for (time, value) in hours.into_iter().zip(values) {
            count += 1.0;
            second_moment += time * time;
            value_sum += value;
            time_value_sum += time * value;
        }
        [value_sum / count, time_value_sum / second_moment]
    }

    fn unwrap_near(value: f64, reference: f64) -> f64 {
        reference + (value - reference + 0.5 * TAU).rem_euclid(TAU) - 0.5 * TAU
    }
}

impl<S: TimeScale> BesselianElementsPolynomial<S> {
    fn from_samples(
        reference_epoch: Instant<S>,
        validity: TimeInterval<S>,
        hours: [f64; BesselianPolynomialFit::SAMPLE_COUNT],
        samples: &[BesselianSample<S>],
        computation: BesselianPolynomialComputation,
    ) -> Result<Self, Error> {
        let reference_sample = samples[BesselianPolynomialFit::SAMPLE_COUNT / 2];
        let reference_mu = reference_sample.mu.as_radians();
        let reference_right_ascension = reference_sample
            .plane
            .shadow_axis()
            .coordinates()
            .right_ascension()
            .as_radians();
        let mut x_values = [0.0; BesselianPolynomialFit::SAMPLE_COUNT];
        let mut y_values = [0.0; BesselianPolynomialFit::SAMPLE_COUNT];
        let mut d_values = [0.0; BesselianPolynomialFit::SAMPLE_COUNT];
        let mut mu_values = [0.0; BesselianPolynomialFit::SAMPLE_COUNT];
        let mut l1_values = [0.0; BesselianPolynomialFit::SAMPLE_COUNT];
        let mut l2_values = [0.0; BesselianPolynomialFit::SAMPLE_COUNT];
        let mut right_ascension_values = [0.0; BesselianPolynomialFit::SAMPLE_COUNT];
        let mut tan_f1_sum = 0.0;
        let mut tan_f2_sum = 0.0;
        for (index, sample) in samples.iter().copied().enumerate() {
            x_values[index] = sample.x;
            y_values[index] = sample.y;
            d_values[index] = sample
                .plane
                .shadow_axis()
                .coordinates()
                .declination()
                .as_radians();
            mu_values[index] =
                BesselianPolynomialFit::unwrap_near(sample.mu.as_radians(), reference_mu);
            l1_values[index] = sample.l1;
            l2_values[index] = sample.l2;
            right_ascension_values[index] = BesselianPolynomialFit::unwrap_near(
                sample
                    .plane
                    .shadow_axis()
                    .coordinates()
                    .right_ascension()
                    .as_radians(),
                reference_right_ascension,
            );
            tan_f1_sum += sample.tan_f1;
            tan_f2_sum += sample.tan_f2;
        }

        let x = BesselianScalarPolynomial::new(BesselianPolynomialFit::cubic(hours, x_values));
        let y = BesselianScalarPolynomial::new(BesselianPolynomialFit::cubic(hours, y_values));
        let d = BesselianAnglePolynomial::new(BesselianPolynomialFit::quadratic(hours, d_values));
        let mu = BesselianAnglePolynomial::new(BesselianPolynomialFit::linear(hours, mu_values));
        let l1 =
            BesselianScalarPolynomial::new(BesselianPolynomialFit::quadratic(hours, l1_values));
        let l2 =
            BesselianScalarPolynomial::new(BesselianPolynomialFit::quadratic(hours, l2_values));
        let axis_right_ascension = BesselianAnglePolynomial::new(BesselianPolynomialFit::cubic(
            hours,
            right_ascension_values,
        ));
        let sample_count = BesselianPolynomialFit::SAMPLE_COUNT as f64;
        let tan_f1 = tan_f1_sum / sample_count;
        let tan_f2 = tan_f2_sum / sample_count;
        let mut maximum_x = 0.0_f64;
        let mut maximum_y = 0.0_f64;
        let mut maximum_d = 0.0_f64;
        let mut maximum_mu = 0.0_f64;
        let mut maximum_l1 = 0.0_f64;
        let mut maximum_l2 = 0.0_f64;
        let mut maximum_right_ascension = 0.0_f64;
        let mut maximum_tan_f1 = 0.0_f64;
        let mut maximum_tan_f2 = 0.0_f64;
        for index in 0..BesselianPolynomialFit::SAMPLE_COUNT {
            let time = hours[index];
            maximum_x = maximum_x.max((x.evaluate(time) - x_values[index]).abs());
            maximum_y = maximum_y.max((y.evaluate(time) - y_values[index]).abs());
            maximum_d = maximum_d.max((d.evaluate_radians(time) - d_values[index]).abs());
            maximum_mu = maximum_mu.max((mu.evaluate_radians(time) - mu_values[index]).abs());
            maximum_l1 = maximum_l1.max((l1.evaluate(time) - l1_values[index]).abs());
            maximum_l2 = maximum_l2.max((l2.evaluate(time) - l2_values[index]).abs());
            maximum_right_ascension = maximum_right_ascension.max(
                (axis_right_ascension.evaluate_radians(time) - right_ascension_values[index]).abs(),
            );
            maximum_tan_f1 = maximum_tan_f1.max((tan_f1 - samples[index].tan_f1).abs());
            maximum_tan_f2 = maximum_tan_f2.max((tan_f2 - samples[index].tan_f2).abs());
        }

        Ok(Self {
            reference_epoch,
            validity,
            x,
            y,
            d,
            mu,
            l1,
            l2,
            axis_right_ascension,
            tan_f1: Dimensionless::new(tan_f1)?,
            tan_f2: Dimensionless::new(tan_f2)?,
            residuals: BesselianPolynomialResiduals {
                x: BesselianPlaneCoordinate::from_equatorial_radii(maximum_x)?,
                y: BesselianPlaneCoordinate::from_equatorial_radii(maximum_y)?,
                d: Angle::from_radians(maximum_d)?,
                mu: Angle::from_radians(maximum_mu)?,
                l1: BesselianShadowRadius::from_equatorial_radii(maximum_l1)?,
                l2: BesselianShadowRadius::from_equatorial_radii(maximum_l2)?,
                axis_right_ascension: Angle::from_radians(maximum_right_ascension)?,
                tan_f1: Dimensionless::new(maximum_tan_f1)?,
                tan_f2: Dimensionless::new(maximum_tan_f2)?,
            },
            earth: computation.earth,
            limb_model: computation.limb_model,
            ephemeris: computation.ephemeris,
            astrometric_evaluations: computation.astrometric_evaluations,
        })
    }
}

impl<S: TimeScale> BesselianElements<S> {
    fn from_samples(
        before: BesselianSample<S>,
        centre: BesselianSample<S>,
        after: BesselianSample<S>,
        computation: BesselianComputation,
    ) -> Result<Self, Error> {
        let derivative_span_seconds = computation.time_step.as_seconds_f64() * 2.0;
        let mu_difference = (after.mu.as_radians() - before.mu.as_radians() + 0.5 * TAU)
            .rem_euclid(TAU)
            - 0.5 * TAU;
        let derivatives = BesselianElementDerivatives {
            x: BesselianElementRate::from_per_second(
                (after.x - before.x) / derivative_span_seconds,
            )?,
            y: BesselianElementRate::from_per_second(
                (after.y - before.y) / derivative_span_seconds,
            )?,
            d: AngularSpeed::from_radians_per_second(
                (after
                    .plane
                    .shadow_axis()
                    .coordinates()
                    .declination()
                    .as_radians()
                    - before
                        .plane
                        .shadow_axis()
                        .coordinates()
                        .declination()
                        .as_radians())
                    / derivative_span_seconds,
            )?,
            mu: AngularSpeed::from_radians_per_second(mu_difference / derivative_span_seconds)?,
            l1: BesselianElementRate::from_per_second(
                (after.l1 - before.l1) / derivative_span_seconds,
            )?,
            l2: BesselianElementRate::from_per_second(
                (after.l2 - before.l2) / derivative_span_seconds,
            )?,
            method: BesselianDerivativeMethod::SymmetricDifference {
                half_step: computation.time_step,
            },
        };
        Ok(Self {
            plane: centre.plane,
            x: BesselianPlaneCoordinate::from_equatorial_radii(centre.x)?,
            y: BesselianPlaneCoordinate::from_equatorial_radii(centre.y)?,
            mu: centre.mu,
            l1: BesselianShadowRadius::from_equatorial_radii(centre.l1)?,
            l2: BesselianShadowRadius::from_equatorial_radii(centre.l2)?,
            tan_f1: Dimensionless::new(centre.tan_f1)?,
            tan_f2: Dimensionless::new(centre.tan_f2)?,
            derivatives,
            earth: computation.earth,
            limb_model: computation.limb_model,
            ephemeris: computation.ephemeris,
            astrometric_evaluations: computation.astrometric_evaluations,
        })
    }
}

impl<S: TimeScale> GlobalSolarEclipseMaximum<S> {
    fn from_geometry(
        geometry: ShadowGeometry<S>,
        earth: ReferenceEllipsoid,
        evidence: ExtremumEvidence<S>,
    ) -> Result<Self, Error> {
        let umbral_vertex = geometry
            .moon_position
            .plus(geometry.axis.scaled(geometry.umbral_vertex_distance));
        let penumbral_vertex = geometry
            .moon_position
            .minus(geometry.axis.scaled(geometry.penumbral_vertex_distance));
        let core_cone = ShadowCone::new(
            umbral_vertex,
            geometry.axis,
            geometry.umbral_angle,
            geometry.earth,
        );
        let penumbral_cone = ShadowCone::new(
            penumbral_vertex,
            geometry.axis,
            geometry.penumbral_angle,
            geometry.earth,
        );
        Ok(Self {
            instant: geometry.epoch,
            gamma: SolarEclipseGamma::from_geometry(
                geometry.axis_offset,
                earth.semi_major_axis().as_metres(),
            ),
            shadow_axis_distance: Length::from_metres(geometry.axis_distance_metres)?,
            penumbra_intersects_earth: penumbral_cone.intersects_earth(ShadowNappe::Downstream),
            umbra_intersects_earth: core_cone.intersects_earth(ShadowNappe::Upstream),
            antumbra_intersects_earth: core_cone.intersects_earth(ShadowNappe::Downstream),
            geometric_core_shadow_radius_at_axis_plane: SolarShadowRadius::from_metres(
                geometry.core_radius_axis_plane_metres,
            )?,
            geometric_core_shadow_radius_at_surface: geometry
                .near_surface_core_radius_metres
                .map(SolarShadowRadius::from_metres)
                .transpose()?,
            evidence,
        })
    }
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Events<'context, 'data, E, P> {
    /// Computes instantaneous Besselian solar-eclipse elements at one physical epoch.
    ///
    /// The fundamental plane passes through the geocentre and is normal to the apparent lunar
    /// shadow axis on true equator and equinox of date axes. `x`, `y`, `l1`, and `l2` are expressed
    /// in units of the selected Earth's equatorial radius; `d` and `μ` are angular values. The
    /// Besselian limb convention and reception light-time controls come from `options`.
    ///
    /// `μ` is the conventional ephemeris hour angle: its sidereal argument uses TT as the
    /// independent ephemeris time. Applying the elements to the rotating Earth therefore requires
    /// a caller-supplied `ΔT = TT−UT1`; `μ` is deliberately not silently changed by whatever EOP
    /// table happens to be installed in the time context. The returned first derivatives use a
    /// symmetric 60-second TT stencil and consume two additional apparent Sun/Moon pairs.
    pub fn solar_eclipse_besselian_elements_at<S: TimeScale>(
        &self,
        earth: &Earth,
        epoch: Instant<S>,
        options: BesselianElementsOptions,
    ) -> Result<BesselianElements<S>, Error> {
        const DERIVATIVE_STEP: Duration =
            Duration::from_nanoseconds(60 * Duration::NANOSECONDS_PER_SECOND);
        const MAXIMUM_EVALUATIONS: u32 = 6;

        let before_epoch = epoch.checked_sub(DERIVATIVE_STEP)?;
        let after_epoch = epoch.checked_add(DERIVATIVE_STEP)?;
        let sampler = BesselianSampler::new(self.astrometry, earth, options);
        let mut evaluations = 0_u32;
        let before = sampler.sample(before_epoch, &mut evaluations, MAXIMUM_EVALUATIONS)?;
        let centre = sampler.sample(epoch, &mut evaluations, MAXIMUM_EVALUATIONS)?;
        let after = sampler.sample(after_epoch, &mut evaluations, MAXIMUM_EVALUATIONS)?;
        let ephemeris = self
            .astrometry
            .ephemeris()
            .provenance()
            .map_err(crate::astro::Error::from)?;
        BesselianElements::from_samples(
            before,
            centre,
            after,
            BesselianComputation {
                time_step: DERIVATIVE_STEP,
                earth: *earth,
                limb_model: options.limb_model(),
                ephemeris,
                astrometric_evaluations: evaluations,
            },
        )
    }

    /// Fits a short-lived Besselian polynomial around one reference epoch.
    ///
    /// Five evenly spaced apparent Sun/Moon pairs cover the selected closed interval. `x` and
    /// `y` use cubic least squares; `d`, `l1`, and `l2` use quadratic fits; `μ` uses a linear fit.
    /// Evaluation later uses analytic derivatives and performs no ephemeris queries. Maximum
    /// sampled residuals and all model provenance remain attached to the polynomial.
    pub fn solar_eclipse_besselian_polynomial<S: TimeScale>(
        &self,
        earth: &Earth,
        reference_epoch: Instant<S>,
        options: BesselianPolynomialOptions,
    ) -> Result<BesselianElementsPolynomial<S>, Error> {
        const MAXIMUM_EVALUATIONS: u32 = 2 * BesselianPolynomialFit::SAMPLE_COUNT as u32;

        let start = reference_epoch.checked_sub(options.validity_half_span())?;
        let end = reference_epoch.checked_add(options.validity_half_span())?;
        let validity = TimeInterval::new(start, end)?;
        let total_nanoseconds = validity.duration().as_nanoseconds();
        let sampler = BesselianSampler::new(self.astrometry, earth, options.elements());
        let mut evaluations = 0_u32;
        let mut hours = [0.0; BesselianPolynomialFit::SAMPLE_COUNT];
        let mut samples = Vec::with_capacity(BesselianPolynomialFit::SAMPLE_COUNT);
        for (index, hour) in hours.iter_mut().enumerate() {
            let offset_nanoseconds = total_nanoseconds * index as i128
                / (BesselianPolynomialFit::SAMPLE_COUNT - 1) as i128;
            let epoch = start.checked_add(Duration::from_nanoseconds(offset_nanoseconds))?;
            *hour = epoch.duration_since(reference_epoch)?.as_seconds_f64() / 3_600.0;
            samples.push(sampler.sample(epoch, &mut evaluations, MAXIMUM_EVALUATIONS)?);
        }
        let ephemeris = self
            .astrometry
            .ephemeris()
            .provenance()
            .map_err(crate::astro::Error::from)?;
        BesselianElementsPolynomial::from_samples(
            reference_epoch,
            validity,
            hours,
            &samples,
            BesselianPolynomialComputation {
                earth: *earth,
                limb_model: options.elements().limb_model(),
                ephemeris,
                astrometric_evaluations: evaluations,
            },
        )
    }

    /// Finds globally visible solar eclipses whose greatest instants lie in a closed interval.
    ///
    /// Apparent geocentric New Moons seed a bounded minimum of the physical Sun-Moon shadow axis.
    /// The selected rotational reference ellipsoid determines gamma, central-axis intersection,
    /// and the central path interval. Exact common-tangent cones for the selected spherical Sun and
    /// Moon figures distinguish penumbra, antumbra, and umbra. A hybrid is reported only when the
    /// cone vertex crosses the near Earth surface along the refined central path.
    ///
    /// This stage does not compute geographic centre-line coordinates, northern or southern limits,
    /// atmospheric visibility, or lunar-limb topography. It needs TT for celestial orientation but
    /// does not need UT1 or observed Earth-orientation data because a rotational ellipsoid is
    /// invariant under rotation about its pole.
    pub fn global_solar_eclipses_in<S: TimeScale>(
        &self,
        earth: &Earth,
        interval: TimeInterval<S>,
        options: SolarEclipseSearchOptions,
    ) -> Result<Vec<GlobalSolarEclipse<S>>, Error> {
        let controls = options.angular_search();
        let seed_interval = TimeInterval::new(
            interval
                .start()
                .checked_sub(GlobalSolarEclipseSampler::<E, P>::MAXIMUM_HALF_WINDOW)?,
            interval
                .end()
                .checked_add(GlobalSolarEclipseSampler::<E, P>::MAXIMUM_HALF_WINDOW)?,
        )?;
        let seeds = self.moon_phase_angle_in(
            seed_interval,
            MoonPhase::NewMoon.target_longitude_difference(),
            controls,
        )?;
        let sampler = GlobalSolarEclipseSampler::new(self.astrometry, earth, options);
        let ephemeris = self
            .astrometry
            .ephemeris()
            .provenance()
            .map_err(crate::astro::Error::from)?;
        let mut eclipses = Vec::new();

        for seed in seeds {
            let (maximum, geometry, mut evaluations) =
                sampler.maximum_near(seed.instant(), options)?;
            if !interval.contains(maximum.instant()) || !maximum.penumbra_intersects_earth() {
                continue;
            }

            let central_path = if geometry.ellipsoid_centrality <= 0.0 {
                Some(sampler.central_path(geometry, maximum, options, &mut evaluations)?)
            } else {
                None
            };
            let kind = if let Some(path) = &central_path {
                if path.transitions().is_empty() {
                    match path.greatest_character() {
                        CentralSolarEclipseCharacter::Annular => GlobalSolarEclipseKind::Annular,
                        CentralSolarEclipseCharacter::Total => GlobalSolarEclipseKind::Total,
                    }
                } else {
                    GlobalSolarEclipseKind::Hybrid
                }
            } else if maximum.umbra_intersects_earth() {
                GlobalSolarEclipseKind::Total
            } else if maximum.antumbra_intersects_earth() {
                GlobalSolarEclipseKind::Annular
            } else {
                GlobalSolarEclipseKind::Partial
            };
            eclipses.push(GlobalSolarEclipse {
                kind,
                maximum,
                central_path,
                earth: *earth,
                model: options.model(),
                ephemeris: ephemeris.clone(),
            });
        }

        eclipses.sort_by_key(|eclipse| eclipse.maximum().instant().tai_nanoseconds_since_1900());
        eclipses.dedup_by_key(|eclipse| eclipse.maximum().instant().tai_nanoseconds_since_1900());
        Ok(eclipses)
    }
}
