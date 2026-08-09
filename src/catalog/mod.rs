//! Typed catalog-place inputs and barycentric stellar space motion.

mod covariance;
mod error;

use core::f64::consts::PI;

use libm::cos;

use crate::{
    frame::{Bcrs, EquatorialDirection, Icrs},
    math::{Angle, AngularSpeed, Declination, Error as MathError, Length, Speed, Vector3},
    time::{Duration, JulianDate, Tcb},
    uncertainty::StandardUncertainty,
};

#[cfg(feature = "std")]
use crate::math::RightAscension;

pub use covariance::{
    SPATIAL_CATALOG_PARAMETER_COUNT, SpatialCatalogCovariance, SpatialCatalogJacobian,
    SpatialCatalogParameter, SpatialCatalogPlaceWithCovariance, SpatialCatalogPropagation,
    SpatialCatalogStandardUncertainties,
};
pub use error::Error;

const ARCSECONDS_PER_RADIAN: f64 = 648_000.0 / PI;
const RADIANS_PER_MILLIARCSECOND: f64 = PI / 648_000_000.0;

/// Proper motion of a catalog source using the conventional `mu_alpha*` component.
///
/// The right-ascension component is the physical tangent-plane rate
/// `d(alpha)/dt * cos(delta)`, not the coordinate rate `d(alpha)/dt`. Both
/// components use one 365.25-day TCB Julian year.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CatalogProperMotion {
    right_ascension_cos_declination: AngularSpeed,
    declination: AngularSpeed,
}

impl CatalogProperMotion {
    /// Number of SI seconds in one 365.25-day Julian year.
    pub const SECONDS_PER_JULIAN_YEAR: f64 =
        Duration::NANOSECONDS_PER_JULIAN_YEAR as f64 / Duration::NANOSECONDS_PER_SECOND as f64;

    /// Constructs proper motion from typed angular rates.
    pub const fn new(
        right_ascension_cos_declination: AngularSpeed,
        declination: AngularSpeed,
    ) -> Self {
        Self {
            right_ascension_cos_declination,
            declination,
        }
    }

    /// Constructs proper motion from radians per TCB Julian year.
    pub fn from_radians_per_julian_year(
        right_ascension_cos_declination: f64,
        declination: f64,
    ) -> Result<Self, MathError> {
        Ok(Self::new(
            AngularSpeed::from_radians_per_second(
                right_ascension_cos_declination / Self::SECONDS_PER_JULIAN_YEAR,
            )?,
            AngularSpeed::from_radians_per_second(declination / Self::SECONDS_PER_JULIAN_YEAR)?,
        ))
    }

    /// Constructs proper motion from milliarcseconds per TCB Julian year.
    pub fn from_milliarcseconds_per_julian_year(
        right_ascension_cos_declination: f64,
        declination: f64,
    ) -> Result<Self, MathError> {
        Self::from_radians_per_julian_year(
            right_ascension_cos_declination * RADIANS_PER_MILLIARCSECOND,
            declination * RADIANS_PER_MILLIARCSECOND,
        )
    }

    /// Returns `mu_alpha*` as a typed angular rate.
    pub const fn right_ascension_cos_declination(self) -> AngularSpeed {
        self.right_ascension_cos_declination
    }

    /// Returns the declination proper motion as a typed angular rate.
    pub const fn declination(self) -> AngularSpeed {
        self.declination
    }

    /// Returns `mu_alpha*` in radians per TCB Julian year.
    pub fn right_ascension_cos_declination_radians_per_julian_year(self) -> f64 {
        self.right_ascension_cos_declination.as_radians_per_second() * Self::SECONDS_PER_JULIAN_YEAR
    }

    /// Returns the declination proper motion in radians per TCB Julian year.
    pub fn declination_radians_per_julian_year(self) -> f64 {
        self.declination.as_radians_per_second() * Self::SECONDS_PER_JULIAN_YEAR
    }

    /// Converts `mu_alpha*` to the coordinate rate `d(alpha)/dt` at a declination.
    pub fn right_ascension_radians_per_julian_year_at(
        self,
        declination: Declination,
    ) -> Result<f64, Error> {
        let declination_radians = declination.as_radians();
        let right_ascension_cos_declination =
            self.right_ascension_cos_declination_radians_per_julian_year();
        let declination_cosine = cos(declination_radians);
        if right_ascension_cos_declination == 0.0 {
            Ok(0.0)
        } else if declination_cosine.abs() <= 64.0 * f64::EPSILON {
            Err(Error::UndefinedRightAscensionMotion {
                declination_radians,
                right_ascension_cos_declination_radians_per_year: right_ascension_cos_declination,
            })
        } else {
            Ok(right_ascension_cos_declination / declination_cosine)
        }
    }
}

/// A signed catalog parallax measurement and its standard uncertainty.
///
/// The fitted value may be zero or negative. Such a measurement remains valid
/// data but cannot be converted directly into a physical [`Parallax`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParallaxMeasurement {
    value: Angle,
    standard_uncertainty: StandardUncertainty<Angle>,
}

impl ParallaxMeasurement {
    /// Constructs a measurement from signed milliarcseconds and a non-negative uncertainty.
    pub fn from_milliarcseconds(value: f64, standard_uncertainty: f64) -> Result<Self, Error> {
        let value = Angle::from_radians(value * RADIANS_PER_MILLIARCSECOND)?;
        let standard_uncertainty = StandardUncertainty::new(Angle::from_radians(
            standard_uncertainty * RADIANS_PER_MILLIARCSECOND,
        )?)?;
        Ok(Self {
            value,
            standard_uncertainty,
        })
    }

    /// Returns the signed fitted parallax angle.
    pub const fn value(self) -> Angle {
        self.value
    }

    /// Returns the non-negative standard uncertainty.
    pub const fn standard_uncertainty(self) -> StandardUncertainty<Angle> {
        self.standard_uncertainty
    }

    /// Returns the signed fitted value in milliarcseconds.
    pub fn as_milliarcseconds(self) -> f64 {
        self.value.as_radians() / RADIANS_PER_MILLIARCSECOND
    }

    /// Returns the standard uncertainty in milliarcseconds.
    pub fn standard_uncertainty_milliarcseconds(self) -> f64 {
        self.standard_uncertainty.value().as_radians() / RADIANS_PER_MILLIARCSECOND
    }

    /// Interprets a strictly positive fitted value as physical parallax.
    pub fn try_physical(self) -> Result<Parallax, Error> {
        Parallax::from_angle(self.value)
    }
}

/// A positive finite physical annual parallax.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Parallax(Angle);

impl Parallax {
    /// Constructs physical parallax from a positive angle.
    pub fn from_angle(value: Angle) -> Result<Self, Error> {
        if value.as_radians() > 0.0 {
            Ok(Self(value))
        } else {
            Err(Error::InvalidPhysicalParallax {
                arcseconds: value.as_radians() * ARCSECONDS_PER_RADIAN,
            })
        }
    }

    /// Constructs physical parallax from arcseconds.
    pub fn from_arcseconds(value: f64) -> Result<Self, Error> {
        Self::from_angle(Angle::from_radians(value / ARCSECONDS_PER_RADIAN)?)
    }

    /// Constructs physical parallax from milliarcseconds.
    pub fn from_milliarcseconds(value: f64) -> Result<Self, Error> {
        Self::from_angle(Angle::from_radians(value * RADIANS_PER_MILLIARCSECOND)?)
    }

    /// Returns physical parallax as an angle.
    pub const fn as_angle(self) -> Angle {
        self.0
    }

    /// Returns physical parallax in arcseconds.
    pub fn as_arcseconds(self) -> f64 {
        self.0.as_radians() * ARCSECONDS_PER_RADIAN
    }

    /// Returns physical parallax in milliarcseconds.
    pub fn as_milliarcseconds(self) -> f64 {
        self.0.as_radians() / RADIANS_PER_MILLIARCSECOND
    }
}

/// Barycentric astrometric radial velocity, positive when the source recedes.
///
/// This is a catalog space-motion quantity, not an unconverted optical, radio,
/// or relativistic spectroscopic velocity convention.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CatalogRadialVelocity(Speed);

impl CatalogRadialVelocity {
    /// Constructs a finite barycentric astrometric radial velocity.
    pub const fn new(value: Speed) -> Self {
        Self(value)
    }

    /// Constructs the velocity in metres per second, positive for recession.
    pub fn from_metres_per_second(value: f64) -> Result<Self, Error> {
        Ok(Self(Speed::from_metres_per_second(value)?))
    }

    /// Constructs the velocity in kilometres per second, positive for recession.
    pub fn from_kilometres_per_second(value: f64) -> Result<Self, Error> {
        Ok(Self(Speed::from_kilometres_per_second(value)?))
    }

    /// Returns the signed typed velocity.
    pub const fn as_speed(self) -> Speed {
        self.0
    }

    /// Returns metres per second, positive for recession.
    pub const fn as_metres_per_second(self) -> f64 {
        self.0.as_metres_per_second()
    }

    /// Returns kilometres per second, positive for recession.
    pub fn as_kilometres_per_second(self) -> f64 {
        self.0.as_kilometres_per_second()
    }
}

/// An infinite-distance ICRS catalog place at one TCB reference epoch.
///
/// This model deliberately has no parallax or radial-velocity fields. It is
/// therefore distinct from [`SpatialCatalogPlace`] and cannot accidentally
/// enter finite-distance space-motion calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InfiniteCatalogPlace {
    reference_epoch: JulianDate<Tcb>,
    direction: EquatorialDirection<Icrs>,
    proper_motion: CatalogProperMotion,
}

impl InfiniteCatalogPlace {
    /// Constructs an infinite-distance catalog place.
    pub const fn new(
        reference_epoch: JulianDate<Tcb>,
        direction: EquatorialDirection<Icrs>,
        proper_motion: CatalogProperMotion,
    ) -> Self {
        Self {
            reference_epoch,
            direction,
            proper_motion,
        }
    }

    /// Constructs a fixed infinite direction with zero proper motion.
    pub fn stationary(
        reference_epoch: JulianDate<Tcb>,
        direction: EquatorialDirection<Icrs>,
    ) -> Result<Self, MathError> {
        Ok(Self::new(
            reference_epoch,
            direction,
            CatalogProperMotion::from_radians_per_julian_year(0.0, 0.0)?,
        ))
    }

    /// Returns the TCB reference epoch.
    pub const fn reference_epoch(self) -> JulianDate<Tcb> {
        self.reference_epoch
    }

    /// Returns the ICRS catalog direction at the reference epoch.
    pub const fn direction(self) -> EquatorialDirection<Icrs> {
        self.direction
    }

    /// Returns the catalog proper motion and its explicit `mu_alpha*` convention.
    pub const fn proper_motion(self) -> CatalogProperMotion {
        self.proper_motion
    }
}

/// A finite-distance six-parameter ICRS catalog place at one TCB epoch.
///
/// The parameters are observables for an imaginary observer at the
/// solar-system barycentre. Proper motion uses the `mu_alpha*` convention and
/// radial velocity is positive for recession.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialCatalogPlace {
    reference_epoch: JulianDate<Tcb>,
    direction: EquatorialDirection<Icrs>,
    proper_motion: CatalogProperMotion,
    parallax: Parallax,
    radial_velocity: CatalogRadialVelocity,
}

impl SpatialCatalogPlace {
    /// IAU model used for catalog/state conversion and full space-motion propagation.
    #[cfg(feature = "std")]
    pub const MODEL: &'static str = "IAU SOFA starpv/pvstar/starpm, Stumpff 1985";

    /// Constructs a physical finite-distance catalog place.
    pub const fn new(
        reference_epoch: JulianDate<Tcb>,
        direction: EquatorialDirection<Icrs>,
        proper_motion: CatalogProperMotion,
        parallax: Parallax,
        radial_velocity: CatalogRadialVelocity,
    ) -> Self {
        Self {
            reference_epoch,
            direction,
            proper_motion,
            parallax,
            radial_velocity,
        }
    }

    /// Returns the TCB reference epoch.
    pub const fn reference_epoch(self) -> JulianDate<Tcb> {
        self.reference_epoch
    }

    /// Returns the ICRS catalog direction at the reference epoch.
    pub const fn direction(self) -> EquatorialDirection<Icrs> {
        self.direction
    }

    /// Returns the tangent-plane proper motion.
    pub const fn proper_motion(self) -> CatalogProperMotion {
        self.proper_motion
    }

    /// Returns the positive physical annual parallax.
    pub const fn parallax(self) -> Parallax {
        self.parallax
    }

    /// Returns the barycentric astrometric radial velocity.
    pub const fn radial_velocity(self) -> CatalogRadialVelocity {
        self.radial_velocity
    }

    /// Converts the six catalog parameters into a barycentric Cartesian state.
    #[cfg(feature = "std")]
    pub fn barycentric_state(self) -> Result<BarycentricCatalogState, Error> {
        let data = self.sofa_catalog_data()?;
        let (pv, status) =
            sofars::astro::starpv(data[0], data[1], data[2], data[3], data[4], data[5]);
        if status != 0 {
            return Err(Error::SpaceMotionFallbackRejected {
                operation: "converting catalog parameters to a barycentric state",
                status,
            });
        }
        BarycentricCatalogState::new(
            self.reference_epoch,
            Vector3::new(
                Length::from_astronomical_units(pv[0][0])?,
                Length::from_astronomical_units(pv[0][1])?,
                Length::from_astronomical_units(pv[0][2])?,
            ),
            Vector3::new(
                Speed::from_astronomical_units_per_day(pv[1][0])?,
                Speed::from_astronomical_units_per_day(pv[1][1])?,
                Speed::from_astronomical_units_per_day(pv[1][2])?,
            ),
        )
    }

    /// Propagates all six catalog parameters to another TCB epoch.
    #[cfg(feature = "std")]
    pub fn propagate_to(self, epoch: JulianDate<Tcb>) -> Result<Self, Error> {
        if epoch == self.reference_epoch {
            return Ok(self);
        }
        let data = self.sofa_catalog_data()?;
        let (reference_first, reference_second) = self.reference_epoch.parts();
        let (epoch_first, epoch_second) = epoch.parts();
        let (propagated, status) = sofars::astro::starpm(
            data[0],
            data[1],
            data[2],
            data[3],
            data[4],
            data[5],
            reference_first,
            reference_second,
            epoch_first,
            epoch_second,
        )
        .map_err(|status| Error::SpaceMotionConversionFailed {
            operation: "propagating catalog space motion",
            status,
        })?;
        if status != 0 {
            return Err(Error::SpaceMotionFallbackRejected {
                operation: "propagating catalog space motion",
                status,
            });
        }
        Self::from_sofa_catalog_data(epoch, propagated)
    }

    #[cfg(feature = "std")]
    pub(crate) fn sofa_catalog_data(self) -> Result<[f64; 6], Error> {
        let declination = self.direction.declination();
        Ok([
            self.direction.right_ascension().as_radians(),
            declination.as_radians(),
            self.proper_motion
                .right_ascension_radians_per_julian_year_at(declination)?,
            self.proper_motion.declination_radians_per_julian_year(),
            self.parallax.as_arcseconds(),
            self.radial_velocity.as_kilometres_per_second(),
        ])
    }

    #[cfg(feature = "std")]
    fn from_sofa_catalog_data(
        reference_epoch: JulianDate<Tcb>,
        data: [f64; 6],
    ) -> Result<Self, Error> {
        let right_ascension = RightAscension::wrap_radians(data[0])?;
        let declination = Declination::try_from_radians(data[1])?;
        let proper_motion =
            CatalogProperMotion::from_radians_per_julian_year(data[2] * cos(data[1]), data[3])?;
        Ok(Self::new(
            reference_epoch,
            EquatorialDirection::new(right_ascension, declination),
            proper_motion,
            Parallax::from_arcseconds(data[4])?,
            CatalogRadialVelocity::from_kilometres_per_second(data[5])?,
        ))
    }
}

/// A finite-distance stellar position and velocity relative to the SSB.
///
/// The axes are aligned with ICRS and the state is bound to a TCB epoch. The
/// state follows SOFA's relativistically adjusted Stumpff space-motion model;
/// it is not an ephemeris state for a solar-system body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarycentricCatalogState {
    epoch: JulianDate<Tcb>,
    position: Vector3<Bcrs, Length>,
    velocity: Vector3<Bcrs, Speed>,
}

impl BarycentricCatalogState {
    /// Constructs a non-zero, subluminal barycentric catalog state.
    pub fn new(
        epoch: JulianDate<Tcb>,
        position: Vector3<Bcrs, Length>,
        velocity: Vector3<Bcrs, Speed>,
    ) -> Result<Self, Error> {
        if position.magnitude()?.as_metres() == 0.0 {
            return Err(Error::NullBarycentricPosition);
        }
        let speed = velocity.magnitude()?.as_metres_per_second();
        if speed >= Length::METRES_PER_LIGHT_SECOND {
            return Err(Error::SuperluminalSpaceMotion {
                metres_per_second: speed,
            });
        }
        Ok(Self {
            epoch,
            position,
            velocity,
        })
    }

    /// Returns the TCB epoch of the state.
    pub const fn epoch(self) -> JulianDate<Tcb> {
        self.epoch
    }

    /// Returns the SSB-relative BCRS position.
    pub const fn position(self) -> Vector3<Bcrs, Length> {
        self.position
    }

    /// Returns the SSB-relative BCRS velocity.
    pub const fn velocity(self) -> Vector3<Bcrs, Speed> {
        self.velocity
    }

    /// Advances the Cartesian state with constant inertial velocity.
    pub fn propagate_to(self, epoch: JulianDate<Tcb>) -> Result<Self, Error> {
        let (epoch_first, epoch_second) = epoch.parts();
        let (reference_first, reference_second) = self.epoch.parts();
        let elapsed_seconds = ((epoch_first - reference_first) + (epoch_second - reference_second))
            * Duration::NANOSECONDS_PER_DAY as f64
            / Duration::NANOSECONDS_PER_SECOND as f64;
        let [x, y, z] = self.position.components();
        let [vx, vy, vz] = self.velocity.components();
        Self::new(
            epoch,
            Vector3::new(
                Length::from_metres(x.as_metres() + vx.as_metres_per_second() * elapsed_seconds)?,
                Length::from_metres(y.as_metres() + vy.as_metres_per_second() * elapsed_seconds)?,
                Length::from_metres(z.as_metres() + vz.as_metres_per_second() * elapsed_seconds)?,
            ),
            self.velocity,
        )
    }

    /// Converts this barycentric state into six catalog parameters.
    #[cfg(feature = "std")]
    pub fn catalog_place(self) -> Result<SpatialCatalogPlace, Error> {
        let [x, y, z] = self.position.components();
        let [vx, vy, vz] = self.velocity.components();
        let pv = [
            [
                x.as_astronomical_units(),
                y.as_astronomical_units(),
                z.as_astronomical_units(),
            ],
            [
                vx.as_astronomical_units_per_day(),
                vy.as_astronomical_units_per_day(),
                vz.as_astronomical_units_per_day(),
            ],
        ];
        let data =
            sofars::astro::pvstar(&pv).map_err(|status| Error::SpaceMotionConversionFailed {
                operation: "converting a barycentric state to catalog parameters",
                status,
            })?;
        SpatialCatalogPlace::from_sofa_catalog_data(self.epoch, data)
    }
}
