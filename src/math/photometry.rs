use core::{fmt::Debug, marker::PhantomData};

use libm::{log10, pow};

use super::Error;

mod sealed {
    pub trait Passband {}
    pub trait MagnitudeSystem {}
}

/// A named photometric response function over wavelength.
///
/// A passband does not define a magnitude zero point. Values in the same passband but different
/// magnitude systems are not directly interchangeable.
pub trait PhotometricPassband:
    sealed::Passband + Copy + Clone + Debug + PartialEq + Eq + 'static
{
    /// Stable human-readable passband identifier.
    const IDENTIFIER: &'static str;
}

/// A named photometric magnitude zero-point convention.
///
/// A magnitude system does not define an instrument passband. Both semantics are required to
/// interpret one magnitude value.
pub trait MagnitudeSystem:
    sealed::MagnitudeSystem + Copy + Clone + Debug + PartialEq + Eq + 'static
{
    /// Stable human-readable magnitude-system identifier.
    const IDENTIFIER: &'static str;
}

/// The Johnson V photometric passband.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JohnsonV;

impl sealed::Passband for JohnsonV {}

impl PhotometricPassband for JohnsonV {
    const IDENTIFIER: &'static str = "Johnson V";
}

/// The Vega-relative magnitude zero-point convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Vega;

impl sealed::MagnitudeSystem for Vega {}

impl MagnitudeSystem for Vega {
    const IDENTIFIER: &'static str = "Vega";
}

/// The AB spectral-flux-density magnitude zero-point convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ab;

impl sealed::MagnitudeSystem for Ab {}

impl MagnitudeSystem for Ab {
    const IDENTIFIER: &'static str = "AB";
}

/// The ST spectral-flux-density magnitude zero-point convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct St;

impl sealed::MagnitudeSystem for St {}

impl MagnitudeSystem for St {
    const IDENTIFIER: &'static str = "ST";
}

/// A finite apparent magnitude in passband `B` and magnitude system `Z`.
///
/// Negative magnitudes are valid. The type carries no implicit atmospheric-extinction state;
/// higher-level results must state whether local extinction has been applied.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ApparentMagnitude<B: PhotometricPassband, Z: MagnitudeSystem> {
    value: f64,
    semantics: PhantomData<(B, Z)>,
}

impl<B: PhotometricPassband, Z: MagnitudeSystem> ApparentMagnitude<B, Z> {
    /// Constructs an apparent magnitude from a finite value in magnitudes.
    pub fn from_magnitudes(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("apparent magnitude", value)?;
        Ok(Self {
            value,
            semantics: PhantomData,
        })
    }

    /// Returns the numerical value in magnitudes.
    pub const fn as_magnitudes(self) -> f64 {
        self.value
    }

    /// Returns `self - reference` as a signed magnitude difference.
    pub fn difference_from(self, reference: Self) -> Result<MagnitudeDifference, Error> {
        MagnitudeDifference::from_magnitudes(self.value - reference.value)
    }

    /// Returns the passband flux ratio `F_self / F_reference`.
    pub fn flux_ratio_to(self, reference: Self) -> Result<FluxRatio, Error> {
        self.difference_from(reference)?.flux_ratio()
    }
}

/// A finite signed difference `m_target - m_reference` in magnitudes.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MagnitudeDifference(f64);

impl MagnitudeDifference {
    /// Constructs a finite signed magnitude difference.
    pub fn from_magnitudes(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("magnitude difference", value).map(Self)
    }

    /// Returns the signed difference in magnitudes.
    pub const fn as_magnitudes(self) -> f64 {
        self.0
    }

    /// Converts `m_target - m_reference` to `F_target / F_reference`.
    pub fn flux_ratio(self) -> Result<FluxRatio, Error> {
        FluxRatio::from_ratio(pow(10.0, -0.4 * self.0))
    }
}

/// A finite, strictly positive passband flux ratio `F_target / F_reference`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FluxRatio(f64);

impl FluxRatio {
    /// Constructs a finite, strictly positive flux ratio.
    pub fn from_ratio(value: f64) -> Result<Self, Error> {
        Error::ensure_finite("flux ratio", value)?;
        if value <= 0.0 {
            return Err(Error::OutOfRange {
                field: "flux ratio",
                value,
                interval: "(0, +infinity)",
                unit: "ratio",
            });
        }
        Ok(Self(value))
    }

    /// Returns the dimensionless flux ratio.
    pub const fn as_ratio(self) -> f64 {
        self.0
    }

    /// Converts `F_target / F_reference` to `m_target - m_reference`.
    pub fn magnitude_difference(self) -> MagnitudeDifference {
        MagnitudeDifference(-2.5 * log10(self.0))
    }
}
