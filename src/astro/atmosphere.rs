use super::Error;

/// Atmospheric pressure at the observer for the SOFA refraction model.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AtmosphericPressure(f64);

impl AtmosphericPressure {
    /// Lowest accepted pressure in hectopascals; zero explicitly selects vacuum.
    pub const MIN_HECTOPASCALS: f64 = 0.0;
    /// Highest accepted pressure in hectopascals supported by the SOFA model.
    pub const MAX_HECTOPASCALS: f64 = 10_000.0;

    /// Constructs atmospheric pressure in hectopascals.
    pub fn from_hectopascals(value: f64) -> Result<Self, Error> {
        Error::ensure_atmospheric_range(
            "atmospheric pressure",
            value,
            Self::MIN_HECTOPASCALS,
            Self::MAX_HECTOPASCALS,
        )
        .map(Self)
    }

    /// Returns atmospheric pressure in hectopascals.
    pub const fn as_hectopascals(self) -> f64 {
        self.0
    }
}

/// Ambient air temperature at the observer.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AirTemperature(f64);

impl AirTemperature {
    /// Lowest accepted temperature in degrees Celsius.
    pub const MIN_DEGREES_CELSIUS: f64 = -150.0;
    /// Highest accepted temperature in degrees Celsius.
    pub const MAX_DEGREES_CELSIUS: f64 = 200.0;

    /// Constructs ambient air temperature in degrees Celsius.
    pub fn from_degrees_celsius(value: f64) -> Result<Self, Error> {
        Error::ensure_atmospheric_range(
            "air temperature",
            value,
            Self::MIN_DEGREES_CELSIUS,
            Self::MAX_DEGREES_CELSIUS,
        )
        .map(Self)
    }

    /// Returns ambient air temperature in degrees Celsius.
    pub const fn as_degrees_celsius(self) -> f64 {
        self.0
    }
}

/// Relative humidity as a fraction in the closed interval `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct RelativeHumidity(f64);

impl RelativeHumidity {
    /// Constructs relative humidity from a fraction in `[0, 1]`.
    pub fn from_fraction(value: f64) -> Result<Self, Error> {
        Error::ensure_atmospheric_range("relative humidity", value, 0.0, 1.0).map(Self)
    }

    /// Returns relative humidity as a fraction.
    pub const fn as_fraction(self) -> f64 {
        self.0
    }
}

/// Observing wavelength used to select optical/infrared or radio refraction.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ObservingWavelength(f64);

impl ObservingWavelength {
    /// Smallest wavelength accepted by the SOFA model.
    pub const MIN_MICROMETRES: f64 = 0.1;
    /// Largest wavelength accepted by the SOFA model.
    pub const MAX_MICROMETRES: f64 = 1.0e6;
    /// Largest wavelength treated as optical/infrared by the SOFA model.
    pub const MAX_OPTICAL_INFRARED_MICROMETRES: f64 = 100.0;

    /// Constructs a wavelength in the explicit SOFA interval `[0.1, 1e6]` micrometres.
    pub fn from_micrometres(value: f64) -> Result<Self, Error> {
        Error::ensure_atmospheric_range(
            "observing wavelength",
            value,
            Self::MIN_MICROMETRES,
            Self::MAX_MICROMETRES,
        )
        .map(Self)
    }

    /// Returns the observing wavelength in micrometres.
    pub const fn as_micrometres(self) -> f64 {
        self.0
    }

    /// Returns whether SOFA treats this wavelength as optical or infrared.
    pub const fn is_optical_or_infrared(self) -> bool {
        self.0 <= Self::MAX_OPTICAL_INFRARED_MICROMETRES
    }
}

/// Validated meteorological inputs for the SOFA atmospheric-refraction model.
///
/// No standard atmosphere is selected implicitly. Zero pressure is an explicit
/// vacuum condition and produces zero refraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtmosphericConditions {
    pressure: AtmosphericPressure,
    temperature: AirTemperature,
    relative_humidity: RelativeHumidity,
    wavelength: ObservingWavelength,
}

impl AtmosphericConditions {
    /// Constructs one immutable set of atmospheric observations.
    pub const fn new(
        pressure: AtmosphericPressure,
        temperature: AirTemperature,
        relative_humidity: RelativeHumidity,
        wavelength: ObservingWavelength,
    ) -> Self {
        Self {
            pressure,
            temperature,
            relative_humidity,
            wavelength,
        }
    }

    /// Returns the atmospheric pressure.
    pub const fn pressure(self) -> AtmosphericPressure {
        self.pressure
    }

    /// Returns the ambient air temperature.
    pub const fn temperature(self) -> AirTemperature {
        self.temperature
    }

    /// Returns the relative humidity.
    pub const fn relative_humidity(self) -> RelativeHumidity {
        self.relative_humidity
    }

    /// Returns the observing wavelength.
    pub const fn wavelength(self) -> ObservingWavelength {
        self.wavelength
    }

    pub(crate) fn sofa_coefficients(self) -> (f64, f64) {
        sofars::astro::refco(
            self.pressure.as_hectopascals(),
            self.temperature.as_degrees_celsius(),
            self.relative_humidity.as_fraction(),
            self.wavelength.as_micrometres(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sofa_refraction_matches_published_ray_tracing_table() {
        // SOFA refco notes: latitude 50°, sea level, 1005 hPa, 280.15 K,
        // 80% humidity, 5740 Å. Values are refraction in arcseconds.
        let table: [(f64, f64); 15] = [
            (10.0, 10.27),
            (20.0, 21.19),
            (30.0, 33.61),
            (40.0, 48.82),
            (45.0, 58.16),
            (50.0, 69.28),
            (55.0, 82.97),
            (60.0, 100.51),
            (65.0, 124.23),
            (70.0, 158.63),
            (72.0, 177.32),
            (74.0, 200.35),
            (76.0, 229.45),
            (78.0, 267.44),
            (80.0, 319.13),
        ];
        let conditions = AtmosphericConditions::new(
            AtmosphericPressure::from_hectopascals(1_005.0).unwrap(),
            AirTemperature::from_degrees_celsius(7.0).unwrap(),
            RelativeHumidity::from_fraction(0.8).unwrap(),
            ObservingWavelength::from_micrometres(0.574).unwrap(),
        );
        let (coefficient_a, coefficient_b) = conditions.sofa_coefficients();
        let arcseconds_per_radian = 180.0 / core::f64::consts::PI * 3_600.0;
        let ray_tracing_tolerance_arcseconds = 0.7;

        for (zenith_distance_degrees, ray_tracing) in table {
            let tangent = zenith_distance_degrees.to_radians().tan();
            let calculated =
                (coefficient_a * tangent + coefficient_b * tangent.powi(3)) * arcseconds_per_radian;
            assert!(
                (calculated - ray_tracing).abs() < ray_tracing_tolerance_arcseconds,
                "ray-tracing mismatch at ZD={zenith_distance_degrees}°: {calculated} arcsec"
            );
        }
    }

    #[test]
    fn zero_pressure_produces_zero_refraction_coefficients() {
        let conditions = AtmosphericConditions::new(
            AtmosphericPressure::from_hectopascals(0.0).unwrap(),
            AirTemperature::from_degrees_celsius(10.0).unwrap(),
            RelativeHumidity::from_fraction(0.5).unwrap(),
            ObservingWavelength::from_micrometres(0.55).unwrap(),
        );

        assert_eq!(conditions.sofa_coefficients(), (0.0, 0.0));
    }
}
