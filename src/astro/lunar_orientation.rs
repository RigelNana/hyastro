use core::fmt;

use libm::{asin, atan2, cos, sin};

use crate::{
    ephem::EphemerisProvider,
    frame::{EquatorialDirection, EquatorialDirectionAt, Frames, Gcrs},
    math::{
        Angle, Declination, Error as MathError, Latitude, Longitude, PositionAngle, RightAscension,
    },
    time::{Hifitime, Instant, JulianDate, Tdb, TimeScale},
};

use super::{Astrometry, Error, LunarIllumination, ReceptionLightTimeOptions};

const DAYS_PER_JULIAN_CENTURY: f64 = 36_525.0;

const ARGUMENT_CONSTANTS_DEGREES: [f64; 13] = [
    125.045, 250.089, 260.008, 176.625, 357.529, 311.589, 134.963, 276.617, 34.226, 15.134,
    119.743, 239.961, 25.053,
];
const ARGUMENT_RATES_DEGREES_PER_DAY: [f64; 13] = [
    -0.052_992_1,
    -0.105_984_2,
    13.012_000_9,
    13.340_715_4,
    0.985_600_3,
    26.405_708_4,
    13.064_993_0,
    0.328_714_6,
    1.748_487_7,
    -0.158_976_3,
    0.003_609_6,
    0.164_357_3,
    12.959_008_8,
];
const POLE_RA_SINE_COEFFICIENTS_DEGREES: [f64; 13] = [
    -3.8787, -0.1204, 0.0700, -0.0172, 0.0, 0.0072, 0.0, 0.0, 0.0, -0.0052, 0.0, 0.0, 0.0043,
];
const POLE_DEC_COSINE_COEFFICIENTS_DEGREES: [f64; 13] = [
    1.5419, 0.0239, -0.0278, 0.0068, 0.0, -0.0029, 0.0009, 0.0, 0.0, 0.0008, 0.0, 0.0, -0.0009,
];
const PRIME_MERIDIAN_SINE_COEFFICIENTS_DEGREES: [f64; 13] = [
    3.5610, 0.1208, -0.0642, 0.0158, 0.0252, -0.0066, -0.0047, -0.0046, 0.0028, 0.0052, 0.0040,
    0.0019, -0.0044,
];

/// Supported analytic lunar body-orientation models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum LunarRotationModel {
    /// IAU WGCCRE 2009 lunar pole and prime-meridian elements.
    #[default]
    Iau2009Wgccre,
}

impl LunarRotationModel {
    /// Exact stable identifier for this model and coefficient set.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::Iau2009Wgccre => "IAU WGCCRE 2009 lunar rotation elements (NAIF pck00011)",
        }
    }

    /// Primary machine-readable source for the implemented coefficients.
    pub const fn source(self) -> &'static str {
        match self {
            Self::Iau2009Wgccre => {
                "https://naif.jpl.nasa.gov/pub/naif/generic_kernels/pck/pck00011.tpc"
            }
        }
    }

    /// Published applicability note for this analytic coefficient set.
    pub const fn applicability(self) -> &'static str {
        match self {
            Self::Iau2009Wgccre => {
                "The WGCCRE report gives no Moon-specific validity interval; use a mission-grade lunar frame kernel when sub-arcminute orientation is required."
            }
        }
    }

    /// Evaluates mean and periodic lunar rotation elements at one physical epoch.
    ///
    /// The WGCCRE independent variable is TDB. `mean` omits the thirteen periodic
    /// terms; `instantaneous` includes them and therefore carries the analytic
    /// physical-libration contribution.
    pub fn rotation_at<S: TimeScale>(self, epoch: Instant<S>) -> Result<LunarRotation<S>, Error> {
        let tdb = JulianDate::<Tdb>::from_instant(epoch, &Hifitime::new())?;
        let (first, second) = tdb.parts();
        let days_since_j2000 = (first - JulianDate::<Tdb>::J2000_VALUE) + second;
        let centuries_since_j2000 = days_since_j2000 / DAYS_PER_JULIAN_CENTURY;

        let mean = LunarRotationElements::new(
            epoch,
            RightAscension::wrap_degrees(269.9949 + 0.0031 * centuries_since_j2000)?,
            Declination::try_from_degrees(66.5392 + 0.0130 * centuries_since_j2000)?,
            PositionAngle::wrap_degrees(
                38.3213 + 13.176_358_15 * days_since_j2000
                    - 1.4e-12 * days_since_j2000 * days_since_j2000,
            )?,
        );

        let mut arguments = [0.0; 13];
        for (index, argument) in arguments.iter_mut().enumerate() {
            *argument = (ARGUMENT_CONSTANTS_DEGREES[index]
                + ARGUMENT_RATES_DEGREES_PER_DAY[index] * days_since_j2000)
                .to_radians();
        }

        let pole_ra_correction = periodic_sine_sum(&arguments, &POLE_RA_SINE_COEFFICIENTS_DEGREES);
        let pole_dec_correction =
            periodic_cosine_sum(&arguments, &POLE_DEC_COSINE_COEFFICIENTS_DEGREES);
        let prime_meridian_correction =
            periodic_sine_sum(&arguments, &PRIME_MERIDIAN_SINE_COEFFICIENTS_DEGREES);
        let instantaneous = LunarRotationElements::new(
            epoch,
            RightAscension::wrap_degrees(
                mean.pole_right_ascension().as_degrees() + pole_ra_correction,
            )?,
            Declination::try_from_degrees(
                mean.pole_declination().as_degrees() + pole_dec_correction,
            )?,
            PositionAngle::wrap_degrees(
                mean.prime_meridian_angle().as_degrees() + prime_meridian_correction,
            )?,
        );

        Ok(LunarRotation {
            model: self,
            mean,
            instantaneous,
        })
    }
}

fn periodic_sine_sum(arguments: &[f64; 13], coefficients: &[f64; 13]) -> f64 {
    arguments
        .iter()
        .zip(coefficients)
        .map(|(argument, coefficient)| coefficient * sin(*argument))
        .sum()
}

fn periodic_cosine_sum(arguments: &[f64; 13], coefficients: &[f64; 13]) -> f64 {
    arguments
        .iter()
        .zip(coefficients)
        .map(|(argument, coefficient)| coefficient * cos(*argument))
        .sum()
}

/// Lunar pole and prime-meridian elements at one physical epoch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarRotationElements<S: TimeScale> {
    epoch: Instant<S>,
    pole: EquatorialDirectionAt<Gcrs, S>,
    prime_meridian_angle: PositionAngle,
}

impl<S: TimeScale> LunarRotationElements<S> {
    fn new(
        epoch: Instant<S>,
        pole_right_ascension: RightAscension,
        pole_declination: Declination,
        prime_meridian_angle: PositionAngle,
    ) -> Self {
        Self {
            epoch,
            pole: EquatorialDirectionAt::new(
                epoch,
                EquatorialDirection::new(pole_right_ascension, pole_declination),
            ),
            prime_meridian_angle,
        }
    }

    /// Returns the physical epoch at which these elements apply.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the lunar north-pole right ascension on ICRS-aligned GCRS axes.
    pub const fn pole_right_ascension(self) -> RightAscension {
        self.pole.coordinates().right_ascension()
    }

    /// Returns the lunar north-pole declination on ICRS-aligned GCRS axes.
    pub const fn pole_declination(self) -> Declination {
        self.pole.coordinates().declination()
    }

    /// Returns the lunar north-pole direction on ICRS-aligned GCRS axes.
    pub const fn north_pole(self) -> EquatorialDirectionAt<Gcrs, S> {
        self.pole
    }

    /// Returns the east-positive prime-meridian rotation angle `W` in `[0, 2π)`.
    pub const fn prime_meridian_angle(self) -> PositionAngle {
        self.prime_meridian_angle
    }

    fn body_axes(self) -> Result<LunarBodyAxes, MathError> {
        let right_ascension = self.pole_right_ascension().as_radians();
        let declination = self.pole_declination().as_radians();
        let prime_meridian = self.prime_meridian_angle.as_radians();
        let right_ascension_sine = sin(right_ascension);
        let right_ascension_cosine = cos(right_ascension);
        let declination_sine = sin(declination);
        let declination_cosine = cos(declination);
        let prime_meridian_sine = sin(prime_meridian);
        let prime_meridian_cosine = cos(prime_meridian);

        let node = [-right_ascension_sine, right_ascension_cosine, 0.0];
        let equator_north = [
            -declination_sine * right_ascension_cosine,
            -declination_sine * right_ascension_sine,
            declination_cosine,
        ];
        let north = self.pole.coordinates().to_direction()?.components();
        Ok(LunarBodyAxes {
            prime_meridian: combine(
                prime_meridian_cosine,
                node,
                prime_meridian_sine,
                equator_north,
            ),
            east_90: combine(
                -prime_meridian_sine,
                node,
                prime_meridian_cosine,
                equator_north,
            ),
            north,
        })
    }
}

/// Mean and instantaneous lunar body orientation at one epoch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarRotation<S: TimeScale> {
    model: LunarRotationModel,
    mean: LunarRotationElements<S>,
    instantaneous: LunarRotationElements<S>,
}

impl<S: TimeScale> LunarRotation<S> {
    /// Returns the coefficient model used for both element sets.
    pub const fn model(self) -> LunarRotationModel {
        self.model
    }

    /// Returns the mean pole and prime meridian with periodic terms omitted.
    pub const fn mean(self) -> LunarRotationElements<S> {
        self.mean
    }

    /// Returns the orientation including the WGCCRE periodic terms.
    pub const fn instantaneous(self) -> LunarRotationElements<S> {
        self.instantaneous
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LunarBodyAxes {
    prime_meridian: [f64; 3],
    east_90: [f64; 3],
    north: [f64; 3],
}

impl LunarBodyAxes {
    fn subobserver_coordinates(
        self,
        observer_from_moon: [f64; 3],
    ) -> Result<LunarLibration, Error> {
        let x = dot(self.prime_meridian, observer_from_moon);
        let y = dot(self.east_90, observer_from_moon);
        let z = dot(self.north, observer_from_moon).clamp(-1.0, 1.0);
        if x == 0.0 && y == 0.0 {
            return Err(MathError::UndefinedLongitude.into());
        }
        Ok(LunarLibration {
            longitude: Longitude::wrap_radians(atan2(y, x))?,
            latitude: Latitude::try_from_radians(asin(z))?,
        })
    }
}

fn combine(left_scale: f64, left: [f64; 3], right_scale: f64, right: [f64; 3]) -> [f64; 3] {
    [
        left_scale * left[0] + right_scale * right[0],
        left_scale * left[1] + right_scale * right[1],
        left_scale * left[2] + right_scale * right[2],
    ]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

/// East-positive selenographic longitude and north-positive latitude of the disk centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarLibration {
    longitude: Longitude,
    latitude: Latitude,
}

impl LunarLibration {
    /// Returns the east-positive selenographic longitude at disk centre.
    pub const fn longitude(self) -> Longitude {
        self.longitude
    }

    /// Returns the north-positive selenographic latitude at disk centre.
    pub const fn latitude(self) -> Latitude {
        self.latitude
    }
}

/// Periodic physical-libration correction relative to the mean lunar rotation model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarPhysicalLibration {
    longitude: Angle,
    latitude: Angle,
}

impl LunarPhysicalLibration {
    /// Returns the shortest signed longitude correction, positive eastward.
    pub const fn longitude(self) -> Angle {
        self.longitude
    }

    /// Returns the signed latitude correction, positive northward.
    pub const fn latitude(self) -> Angle {
        self.latitude
    }
}

/// Geocentric apparent lunar-disk orientation at one Earth reception epoch.
///
/// Optical libration uses the model's mean pole and prime meridian. Total
/// libration uses the periodic WGCCRE orientation. Their signed difference is
/// reported separately as physical libration. Position angles are measured
/// eastward from true-of-date celestial north on the apparent lunar disk.
pub struct LunarDiskOrientation<S: TimeScale> {
    illumination: LunarIllumination<S>,
    rotation: LunarRotation<S>,
    optical_libration: LunarLibration,
    physical_libration: LunarPhysicalLibration,
    total_libration: LunarLibration,
    axis_position_angle: PositionAngle,
    bright_limb_position_angle: PositionAngle,
}

impl<S: TimeScale> LunarDiskOrientation<S> {
    /// Returns the coherent Moon-Sun illumination solution used by this result.
    pub const fn illumination(self) -> LunarIllumination<S> {
        self.illumination
    }

    /// Returns the mean and instantaneous lunar rotation elements.
    pub const fn rotation(self) -> LunarRotation<S> {
        self.rotation
    }

    /// Returns optical libration from orbital viewing geometry and mean rotation.
    pub const fn optical_libration(self) -> LunarLibration {
        self.optical_libration
    }

    /// Returns the periodic physical-libration correction.
    pub const fn physical_libration(self) -> LunarPhysicalLibration {
        self.physical_libration
    }

    /// Returns the total apparent geocentric libration of the disk centre.
    pub const fn total_libration(self) -> LunarLibration {
        self.total_libration
    }

    /// Returns the apparent lunar north-pole position angle.
    pub const fn axis_position_angle(self) -> PositionAngle {
        self.axis_position_angle
    }

    /// Returns the apparent position angle toward the illuminated limb.
    pub const fn bright_limb_position_angle(self) -> PositionAngle {
        self.bright_limb_position_angle
    }
}

impl<S: TimeScale> Copy for LunarDiskOrientation<S> {}

impl<S: TimeScale> Clone for LunarDiskOrientation<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for LunarDiskOrientation<S> {
    fn eq(&self, other: &Self) -> bool {
        self.illumination == other.illumination
            && self.rotation == other.rotation
            && self.optical_libration == other.optical_libration
            && self.physical_libration == other.physical_libration
            && self.total_libration == other.total_libration
            && self.axis_position_angle == other.axis_position_angle
            && self.bright_limb_position_angle == other.bright_limb_position_angle
    }
}

impl<S: TimeScale> fmt::Debug for LunarDiskOrientation<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LunarDiskOrientation")
            .field("illumination", &self.illumination)
            .field("rotation", &self.rotation)
            .field("optical_libration", &self.optical_libration)
            .field("physical_libration", &self.physical_libration)
            .field("total_libration", &self.total_libration)
            .field("axis_position_angle", &self.axis_position_angle)
            .field(
                "bright_limb_position_angle",
                &self.bright_limb_position_angle,
            )
            .finish()
    }
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Astrometry<'context, 'data, E, P> {
    /// Computes geocentric optical, physical, and total lunar libration and disk angles.
    pub fn lunar_disk_orientation_at<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        light_time_options: ReceptionLightTimeOptions,
        rotation_model: LunarRotationModel,
    ) -> Result<LunarDiskOrientation<S>, Error> {
        let illumination = self.lunar_illumination_at(epoch, light_time_options)?;
        let apparent_moon = illumination.apparent_moon();
        let apparent_sun = illumination.apparent_sun();
        let rotation = rotation_model.rotation_at(apparent_moon.emission_epoch())?;
        let observer_to_moon = apparent_moon
            .reception_light_time()
            .direction()
            .components();
        let observer_from_moon = [
            -observer_to_moon[0],
            -observer_to_moon[1],
            -observer_to_moon[2],
        ];
        let optical_libration = rotation
            .mean()
            .body_axes()?
            .subobserver_coordinates(observer_from_moon)?;
        let total_libration = rotation
            .instantaneous()
            .body_axes()?
            .subobserver_coordinates(observer_from_moon)?;
        let longitude_correction = Longitude::wrap_radians(
            total_libration.longitude().as_radians() - optical_libration.longitude().as_radians(),
        )?;
        let physical_libration = LunarPhysicalLibration {
            longitude: longitude_correction.as_angle(),
            latitude: Angle::from_radians(
                total_libration.latitude().as_radians() - optical_libration.latitude().as_radians(),
            )?,
        };

        let moon_true = apparent_moon
            .true_equatorial()
            .coordinates()
            .to_spherical()?;
        let sun_true = apparent_sun
            .true_equatorial()
            .coordinates()
            .to_spherical()?;
        let bright_limb_position_angle = moon_true.position_angle_to(sun_true)?;

        let pole_gcrs = rotation.instantaneous().north_pole().coordinates();
        let celestial = Frames::new(self.time_context()).celestial_orientation_at(epoch)?;
        let pole_true = celestial
            .true_equatorial(pole_gcrs)?
            .coordinates()
            .to_spherical()?;
        let axis_position_angle = moon_true.position_angle_to(pole_true)?;

        Ok(LunarDiskOrientation {
            illumination,
            rotation,
            optical_libration,
            physical_libration,
            total_libration,
            axis_position_angle,
            bright_limb_position_angle,
        })
    }
}
