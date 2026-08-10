use core::{
    f64::consts::{PI, TAU},
    fmt,
};

use libm::cos;

use crate::{
    ephem::{CelestialBody, EphemerisProvider, EphemerisQuery},
    frame::Bcrs,
    math::{Angle, Direction, Error as MathError, PhaseAngle, Separation},
    time::{Instant, TimeScale},
};

use super::{
    Astrometry, Error, GeocentricApparentPlace, ReceptionLightTime, ReceptionLightTimeOptions,
    SolarApparentPlace,
};

/// The named half of one directed lunar-solar longitude cycle.
///
/// This is a branch classification, not the instantaneous derivative of illuminated fraction.
/// The waxing branch covers directed apparent longitude differences in `[0, π)`, beginning at
/// New Moon. The waning branch covers `[π, 2π)`, beginning at Full Moon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoonPhaseBranch {
    /// The directed longitude difference is on the New-Moon-to-Full-Moon half-cycle.
    Waxing,
    /// The directed longitude difference is on the Full-Moon-to-New-Moon half-cycle.
    Waning,
}

/// A directed apparent lunar phase-cycle angle in the half-open interval `[0, 2π)`.
///
/// The value is the Moon's apparent geocentric longitude minus the Sun's on true ecliptic and
/// equinox of date axes. It distinguishes waxing from waning and must not be confused with the
/// unsigned physical [`PhaseAngle`] measured at the Moon.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MoonPhaseAngle(f64);

impl MoonPhaseAngle {
    pub(crate) const fn from_validated_radians(value: f64) -> Self {
        Self(value)
    }

    /// Constructs a directed Moon phase angle from radians without normalization.
    pub fn try_from_radians(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("directed Moon phase angle", value)?;
        if (0.0..TAU).contains(&value) {
            Ok(Self(value))
        } else {
            Err(MathError::OutOfRange {
                field: "directed Moon phase angle",
                value,
                interval: "[0, 2π)",
                unit: "rad",
            })
        }
    }

    /// Constructs a directed Moon phase angle from degrees without normalization.
    pub fn try_from_degrees(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("directed Moon phase angle", value)?;
        Self::try_from_radians(value.to_radians())
    }

    /// Normalizes radians into the directed Moon phase-angle interval.
    pub fn wrap_radians(value: f64) -> Result<Self, MathError> {
        Angle::wrap_zero_tau(value, "directed Moon phase angle").map(Self)
    }

    /// Normalizes degrees into the directed Moon phase-angle interval.
    pub fn wrap_degrees(value: f64) -> Result<Self, MathError> {
        MathError::ensure_finite("directed Moon phase angle", value)?;
        Self::wrap_radians(value.to_radians())
    }

    /// Returns the directed Moon phase angle in radians.
    pub const fn as_radians(self) -> f64 {
        self.0
    }

    /// Returns the directed Moon phase angle in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Returns the value as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        Angle::from_finite(self.0)
    }
}

/// A lunar-disk illuminated fraction in the closed interval `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct IlluminatedFraction(f64);

impl IlluminatedFraction {
    const fn from_valid_ratio(ratio: f64) -> Self {
        Self(ratio)
    }

    /// Returns the illuminated fraction as a ratio in `[0, 1]`.
    pub const fn as_ratio(self) -> f64 {
        self.0
    }

    /// Returns the illuminated fraction as a percentage in `[0, 100]`.
    pub fn as_percent(self) -> f64 {
        self.0 * 100.0
    }
}

/// Geocentric apparent lunar illumination at one Earth reception epoch.
///
/// Three coherent light-time legs are retained. The apparent Moon and Sun are both received at
/// Earth at [`Self::reception_epoch`]. The sunlight leg is received at the Moon when the observed
/// lunar light leaves it, and is emitted by the Sun at its own retarded epoch. The physical phase
/// angle is measured at that lunar emission event between the incoming solar direction and the
/// outgoing direction toward the receiving Earth.
pub struct LunarIllumination<S: TimeScale> {
    apparent_moon: GeocentricApparentPlace<S>,
    apparent_sun: SolarApparentPlace<S>,
    sunlight_at_moon: ReceptionLightTime<S>,
    directed_elongation: MoonPhaseAngle,
    apparent_separation: Separation,
    phase_angle: PhaseAngle,
    illuminated_fraction: IlluminatedFraction,
    branch: MoonPhaseBranch,
}

impl<S: TimeScale> LunarIllumination<S> {
    /// Returns the common Earth reception epoch of the apparent Moon and Sun.
    pub const fn reception_epoch(self) -> Instant<S> {
        self.apparent_moon.reception_epoch()
    }

    /// Returns the Moon's geocentric apparent place received at Earth.
    pub const fn apparent_moon(self) -> GeocentricApparentPlace<S> {
        self.apparent_moon
    }

    /// Returns the Sun's geocentric apparent place received at Earth.
    pub const fn apparent_sun(self) -> SolarApparentPlace<S> {
        self.apparent_sun
    }

    /// Returns the Sun-to-Moon reception light-time solution.
    ///
    /// Its observer is the Moon at [`GeocentricApparentPlace::emission_epoch`] and its target is
    /// the Sun at the retarded solar emission epoch.
    pub const fn sunlight_at_moon(self) -> ReceptionLightTime<S> {
        self.sunlight_at_moon
    }

    /// Returns the Moon-minus-Sun apparent longitude difference in `[0, 2π)`.
    ///
    /// Both longitudes use true ecliptic and equinox of date axes at the Earth reception epoch.
    pub const fn directed_elongation(self) -> MoonPhaseAngle {
        self.directed_elongation
    }

    /// Returns the three-dimensional apparent geocentric angular separation of Moon and Sun.
    pub const fn apparent_separation(self) -> Separation {
        self.apparent_separation
    }

    /// Returns the physical Sun-Moon-Earth phase angle measured at the Moon.
    pub const fn phase_angle(self) -> PhaseAngle {
        self.phase_angle
    }

    /// Returns the visible lunar disk's illuminated fraction.
    pub const fn illuminated_fraction(self) -> IlluminatedFraction {
        self.illuminated_fraction
    }

    /// Returns the named waxing or waning half of the directed phase cycle.
    pub const fn branch(self) -> MoonPhaseBranch {
        self.branch
    }
}

impl<S: TimeScale> Copy for LunarIllumination<S> {}

impl<S: TimeScale> Clone for LunarIllumination<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for LunarIllumination<S> {
    fn eq(&self, other: &Self) -> bool {
        self.apparent_moon == other.apparent_moon
            && self.apparent_sun == other.apparent_sun
            && self.sunlight_at_moon == other.sunlight_at_moon
            && self.directed_elongation == other.directed_elongation
            && self.apparent_separation == other.apparent_separation
            && self.phase_angle == other.phase_angle
            && self.illuminated_fraction == other.illuminated_fraction
            && self.branch == other.branch
    }
}

impl<S: TimeScale> fmt::Debug for LunarIllumination<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LunarIllumination")
            .field("apparent_moon", &self.apparent_moon)
            .field("apparent_sun", &self.apparent_sun)
            .field("sunlight_at_moon", &self.sunlight_at_moon)
            .field("directed_elongation", &self.directed_elongation)
            .field("apparent_separation", &self.apparent_separation)
            .field("phase_angle", &self.phase_angle)
            .field("illuminated_fraction", &self.illuminated_fraction)
            .field("branch", &self.branch)
            .finish()
    }
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Astrometry<'context, 'data, E, P> {
    /// Computes the Moon's apparent phase geometry and illuminated fraction at one epoch.
    ///
    /// The calculation solves separate one-way light-time paths from the Moon and Sun to Earth,
    /// then solves the sunlight path received at the Moon's retained emission epoch. Apparent
    /// elongation uses the Earth-received directions; physical phase angle and illuminated
    /// fraction use the coherent Sun-Moon-Earth geometry at the lunar emission event. Station
    /// parallax, lunar limb topography, libration, and opposition surge are excluded.
    pub fn lunar_illumination_at<S: TimeScale>(
        &self,
        reception_epoch: Instant<S>,
        light_time_options: ReceptionLightTimeOptions,
    ) -> Result<LunarIllumination<S>, Error> {
        let apparent_moon = self.geocentric_apparent_place(
            CelestialBody::Moon,
            reception_epoch,
            light_time_options,
        )?;
        let apparent_sun = self.solar_apparent_place(reception_epoch, light_time_options)?;
        let sunlight_at_moon = self.reception_light_time(
            EphemerisQuery::<Bcrs, S>::new(
                CelestialBody::Sun,
                CelestialBody::Moon,
                apparent_moon.emission_epoch(),
            ),
            light_time_options,
        )?;

        let directed_elongation_radians = Angle::wrap_zero_tau(
            apparent_moon.longitude().as_radians() - apparent_sun.longitude().as_radians(),
            "apparent lunar-minus-solar longitude",
        )?;
        let directed_elongation =
            MoonPhaseAngle::from_validated_radians(directed_elongation_radians);
        let branch = if directed_elongation_radians < PI {
            MoonPhaseBranch::Waxing
        } else {
            MoonPhaseBranch::Waning
        };

        let moon_direction = apparent_moon
            .gcrs_direction()
            .coordinates()
            .to_direction()?;
        let sun_direction = apparent_sun.gcrs_direction().coordinates().to_direction()?;
        let apparent_separation =
            Separation::try_from_radians(moon_direction.angle_to(sun_direction)?.as_radians())?;

        let [earth_to_moon_x, earth_to_moon_y, earth_to_moon_z] = apparent_moon
            .reception_light_time()
            .direction()
            .components();
        let moon_to_earth = Direction::<Bcrs>::try_from_components([
            -earth_to_moon_x,
            -earth_to_moon_y,
            -earth_to_moon_z,
        ])?;
        let phase_angle = PhaseAngle::try_from_radians(
            sunlight_at_moon
                .direction()
                .angle_to(moon_to_earth)?
                .as_radians(),
        )?;
        let illuminated_fraction =
            IlluminatedFraction::from_valid_ratio((1.0 + cos(phase_angle.as_radians())) * 0.5);

        Ok(LunarIllumination {
            apparent_moon,
            apparent_sun,
            sunlight_at_moon,
            directed_elongation,
            apparent_separation,
            phase_angle,
            illuminated_fraction,
            branch,
        })
    }
}
