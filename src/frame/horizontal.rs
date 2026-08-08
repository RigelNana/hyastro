use core::f64::consts::FRAC_PI_2;

#[cfg(feature = "std")]
use libm::{asin, atan2, sqrt};
use libm::{cos, sin};

use crate::math::{Altitude, Azimuth, Error as MathError, ZenithDistance};

/// A local horizontal unit direction using north-zero, east-positive azimuth.
///
/// This value describes only coordinates on east-north-up axes. It does not
/// identify a site, epoch, atmospheric model, or spatial origin. Azimuth is
/// absent at the zenith and nadir, where it is mathematically undefined.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizontalDirection {
    azimuth: Option<Azimuth>,
    altitude: Altitude,
    enu_components: [f64; 3],
}

impl HorizontalDirection {
    /// Constructs a horizontal direction from defined azimuth and altitude.
    pub fn new(azimuth: Azimuth, altitude: Altitude) -> Self {
        let altitude_radians = altitude.as_radians();
        let azimuth_radians = azimuth.as_radians();
        let altitude_cosine = cos(altitude_radians);
        Self {
            azimuth: Some(azimuth),
            altitude,
            enu_components: [
                altitude_cosine * sin(azimuth_radians),
                altitude_cosine * cos(azimuth_radians),
                sin(altitude_radians),
            ],
        }
    }

    /// Constructs the zenith direction, whose azimuth is undefined.
    pub fn zenith() -> Result<Self, MathError> {
        Ok(Self {
            azimuth: None,
            altitude: Altitude::try_from_radians(FRAC_PI_2)?,
            enu_components: [0.0, 0.0, 1.0],
        })
    }

    /// Constructs the nadir direction, whose azimuth is undefined.
    pub fn nadir() -> Result<Self, MathError> {
        Ok(Self {
            azimuth: None,
            altitude: Altitude::try_from_radians(-FRAC_PI_2)?,
            enu_components: [0.0, 0.0, -1.0],
        })
    }

    /// Returns azimuth eastward from north, or `None` at the zenith or nadir.
    pub const fn azimuth(self) -> Option<Azimuth> {
        self.azimuth
    }

    /// Returns altitude above the astronomical horizon.
    pub const fn altitude(self) -> Altitude {
        self.altitude
    }

    /// Returns angular distance from the zenith.
    pub fn zenith_distance(self) -> Result<ZenithDistance, MathError> {
        ZenithDistance::try_from_radians(FRAC_PI_2 - self.altitude.as_radians())
    }

    #[cfg(feature = "std")]
    pub(crate) fn from_enu_components(components: [f64; 3]) -> Result<Self, MathError> {
        let norm = sqrt(
            components[0] * components[0]
                + components[1] * components[1]
                + components[2] * components[2],
        );
        if norm == 0.0 {
            return Err(MathError::ZeroVector);
        }
        MathError::ensure_finite("horizontal direction norm", norm)?;
        let enu_components = [
            components[0] / norm,
            components[1] / norm,
            components[2] / norm,
        ];
        let horizontal =
            sqrt(enu_components[0] * enu_components[0] + enu_components[1] * enu_components[1]);
        let azimuth = if horizontal <= 32.0 * f64::EPSILON {
            None
        } else {
            Some(Azimuth::wrap_radians(atan2(
                enu_components[0],
                enu_components[1],
            ))?)
        };
        Ok(Self {
            azimuth,
            altitude: Altitude::try_from_radians(asin(enu_components[2].clamp(-1.0, 1.0)))?,
            enu_components,
        })
    }

    #[cfg(feature = "std")]
    pub(crate) const fn enu_components(self) -> [f64; 3] {
        self.enu_components
    }
}
