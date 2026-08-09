use libm::cos;

use crate::{
    math::{Angle, AngularSpeed, Speed},
    uncertainty::{CorrelationMatrix, StandardUncertainty},
};

#[cfg(feature = "std")]
use core::f64::consts::{FRAC_PI_2, PI, TAU};
#[cfg(feature = "std")]
use libm::sqrt;

#[cfg(feature = "std")]
use crate::{
    frame::{EquatorialDirection, Icrs},
    math::{Declination, RightAscension},
    time::{JulianDate, Tcb},
};

#[cfg(feature = "std")]
use super::{CatalogProperMotion, CatalogRadialVelocity, Parallax};
use super::{Error, SpatialCatalogPlace};

/// Number of parameters in a finite-distance catalog solution.
pub const SPATIAL_CATALOG_PARAMETER_COUNT: usize = 6;

/// Fixed order and canonical units of a spatial-catalog covariance.
///
/// Right ascension is represented by its local tangent-plane differential
/// $d\alpha\cos\delta$, never by an unscaled coordinate differential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SpatialCatalogParameter {
    /// Local right-ascension tangent-plane coordinate $\alpha*$, in radians.
    RightAscensionTangentPlane = 0,
    /// Declination, in radians.
    Declination = 1,
    /// Annual parallax, in radians.
    Parallax = 2,
    /// Proper motion $\mu_{\alpha*}$, in radians per second.
    RightAscensionProperMotion = 3,
    /// Declination proper motion, in radians per second.
    DeclinationProperMotion = 4,
    /// Barycentric astrometric radial velocity, in metres per second.
    RadialVelocity = 5,
}

impl SpatialCatalogParameter {
    /// Every parameter in covariance row and column order.
    pub const ALL: [Self; SPATIAL_CATALOG_PARAMETER_COUNT] = [
        Self::RightAscensionTangentPlane,
        Self::Declination,
        Self::Parallax,
        Self::RightAscensionProperMotion,
        Self::DeclinationProperMotion,
        Self::RadialVelocity,
    ];

    /// Returns the stable parameter name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::RightAscensionTangentPlane => "right-ascension tangent-plane coordinate",
            Self::Declination => "declination",
            Self::Parallax => "parallax",
            Self::RightAscensionProperMotion => "right-ascension proper motion mu_alpha*",
            Self::DeclinationProperMotion => "declination proper motion",
            Self::RadialVelocity => "barycentric astrometric radial velocity",
        }
    }

    /// Returns the canonical unit used by covariance elements.
    pub const fn canonical_unit(self) -> &'static str {
        match self {
            Self::RightAscensionTangentPlane | Self::Declination | Self::Parallax => "rad",
            Self::RightAscensionProperMotion | Self::DeclinationProperMotion => "rad/s",
            Self::RadialVelocity => "m/s",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// Typed one-standard-deviation uncertainties for the fixed six-parameter order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialCatalogStandardUncertainties {
    right_ascension_tangent_plane: StandardUncertainty<Angle>,
    declination: StandardUncertainty<Angle>,
    parallax: StandardUncertainty<Angle>,
    right_ascension_proper_motion: StandardUncertainty<AngularSpeed>,
    declination_proper_motion: StandardUncertainty<AngularSpeed>,
    radial_velocity: StandardUncertainty<Speed>,
}

impl SpatialCatalogStandardUncertainties {
    /// Constructs uncertainties in [`SpatialCatalogParameter::ALL`] order.
    pub const fn new(
        right_ascension_tangent_plane: StandardUncertainty<Angle>,
        declination: StandardUncertainty<Angle>,
        parallax: StandardUncertainty<Angle>,
        right_ascension_proper_motion: StandardUncertainty<AngularSpeed>,
        declination_proper_motion: StandardUncertainty<AngularSpeed>,
        radial_velocity: StandardUncertainty<Speed>,
    ) -> Self {
        Self {
            right_ascension_tangent_plane,
            declination,
            parallax,
            right_ascension_proper_motion,
            declination_proper_motion,
            radial_velocity,
        }
    }

    /// Returns the $\alpha*$ tangent-plane uncertainty.
    pub const fn right_ascension_tangent_plane(self) -> StandardUncertainty<Angle> {
        self.right_ascension_tangent_plane
    }

    /// Returns the declination uncertainty.
    pub const fn declination(self) -> StandardUncertainty<Angle> {
        self.declination
    }

    /// Returns the parallax uncertainty.
    pub const fn parallax(self) -> StandardUncertainty<Angle> {
        self.parallax
    }

    /// Returns the $\mu_{\alpha*}$ uncertainty.
    pub const fn right_ascension_proper_motion(self) -> StandardUncertainty<AngularSpeed> {
        self.right_ascension_proper_motion
    }

    /// Returns the declination proper-motion uncertainty.
    pub const fn declination_proper_motion(self) -> StandardUncertainty<AngularSpeed> {
        self.declination_proper_motion
    }

    /// Returns the radial-velocity uncertainty.
    pub const fn radial_velocity(self) -> StandardUncertainty<Speed> {
        self.radial_velocity
    }

    fn canonical_values(self) -> [f64; SPATIAL_CATALOG_PARAMETER_COUNT] {
        [
            self.right_ascension_tangent_plane.value().as_radians(),
            self.declination.value().as_radians(),
            self.parallax.value().as_radians(),
            self.right_ascension_proper_motion
                .value()
                .as_radians_per_second(),
            self.declination_proper_motion
                .value()
                .as_radians_per_second(),
            self.radial_velocity.value().as_metres_per_second(),
        ]
    }

    #[cfg(feature = "std")]
    fn from_canonical_values(
        values: [f64; SPATIAL_CATALOG_PARAMETER_COUNT],
    ) -> Result<Self, Error> {
        Ok(Self::new(
            StandardUncertainty::new(Angle::from_radians(values[0])?)?,
            StandardUncertainty::new(Angle::from_radians(values[1])?)?,
            StandardUncertainty::new(Angle::from_radians(values[2])?)?,
            StandardUncertainty::new(AngularSpeed::from_radians_per_second(values[3])?)?,
            StandardUncertainty::new(AngularSpeed::from_radians_per_second(values[4])?)?,
            StandardUncertainty::new(Speed::from_metres_per_second(values[5])?)?,
        ))
    }
}

/// Complete covariance of a six-parameter spatial catalog solution.
///
/// The matrix is represented by typed standard uncertainties plus a validated
/// correlation matrix. This prevents callers from losing the mixed units or
/// the fixed parameter order while retaining every covariance element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialCatalogCovariance {
    standard_uncertainties: SpatialCatalogStandardUncertainties,
    correlations: CorrelationMatrix<SPATIAL_CATALOG_PARAMETER_COUNT>,
}

impl SpatialCatalogCovariance {
    /// Constructs a complete covariance from standard uncertainties and correlations.
    pub fn new(
        standard_uncertainties: SpatialCatalogStandardUncertainties,
        correlations: CorrelationMatrix<SPATIAL_CATALOG_PARAMETER_COUNT>,
    ) -> Result<Self, Error> {
        let values = standard_uncertainties.canonical_values();
        let coefficients = correlations.coefficients();
        for parameter in SpatialCatalogParameter::ALL {
            let row = parameter.index();
            if values[row] == 0.0 {
                for other in SpatialCatalogParameter::ALL {
                    let column = other.index();
                    if row != column && coefficients[row][column] != 0.0 {
                        return Err(Error::UndefinedCorrelationForZeroUncertainty {
                            parameter: parameter.name(),
                            other_parameter: other.name(),
                            coefficient: coefficients[row][column],
                        });
                    }
                }
            }
        }
        Ok(Self {
            standard_uncertainties,
            correlations,
        })
    }

    /// Constructs a covariance whose six parameters are mutually uncorrelated.
    pub fn uncorrelated(
        standard_uncertainties: SpatialCatalogStandardUncertainties,
    ) -> Result<Self, Error> {
        Self::new(standard_uncertainties, CorrelationMatrix::identity())
    }

    /// Returns the typed diagonal standard uncertainties.
    pub const fn standard_uncertainties(self) -> SpatialCatalogStandardUncertainties {
        self.standard_uncertainties
    }

    /// Returns the validated dimensionless correlation matrix.
    pub const fn correlations(self) -> CorrelationMatrix<SPATIAL_CATALOG_PARAMETER_COUNT> {
        self.correlations
    }

    /// Returns one correlation coefficient.
    pub fn correlation(self, left: SpatialCatalogParameter, right: SpatialCatalogParameter) -> f64 {
        self.correlations.coefficients()[left.index()][right.index()]
    }

    /// Returns one covariance in the product of the two parameters' canonical units.
    ///
    /// The units are available from [`SpatialCatalogParameter::canonical_unit`].
    pub fn canonical_covariance(
        self,
        left: SpatialCatalogParameter,
        right: SpatialCatalogParameter,
    ) -> f64 {
        let values = self.standard_uncertainties.canonical_values();
        values[left.index()] * self.correlation(left, right) * values[right.index()]
    }

    #[cfg(feature = "std")]
    fn canonical_matrix(
        self,
    ) -> [[f64; SPATIAL_CATALOG_PARAMETER_COUNT]; SPATIAL_CATALOG_PARAMETER_COUNT] {
        let uncertainties = self.standard_uncertainties.canonical_values();
        let correlations = self.correlations.coefficients();
        let mut covariance =
            [[0.0; SPATIAL_CATALOG_PARAMETER_COUNT]; SPATIAL_CATALOG_PARAMETER_COUNT];
        for row in 0..SPATIAL_CATALOG_PARAMETER_COUNT {
            for column in 0..SPATIAL_CATALOG_PARAMETER_COUNT {
                covariance[row][column] =
                    correlations[row][column] * uncertainties[row] * uncertainties[column];
            }
        }
        covariance
    }

    #[cfg(feature = "std")]
    fn from_canonical_matrix(
        covariance: [[f64; SPATIAL_CATALOG_PARAMETER_COUNT]; SPATIAL_CATALOG_PARAMETER_COUNT],
    ) -> Result<Self, Error> {
        let mut uncertainties = [0.0; SPATIAL_CATALOG_PARAMETER_COUNT];
        for parameter in SpatialCatalogParameter::ALL {
            let variance = covariance[parameter.index()][parameter.index()];
            if !variance.is_finite() || variance < 0.0 {
                return Err(Error::InvalidPropagatedVariance {
                    parameter: parameter.name(),
                    variance,
                });
            }
            uncertainties[parameter.index()] = sqrt(variance);
        }

        let mut coefficients =
            CorrelationMatrix::<SPATIAL_CATALOG_PARAMETER_COUNT>::identity().coefficients();
        for row in 0..SPATIAL_CATALOG_PARAMETER_COUNT {
            for column in (row + 1)..SPATIAL_CATALOG_PARAMETER_COUNT {
                let denominator = uncertainties[row] * uncertainties[column];
                let coefficient = if denominator == 0.0 {
                    0.0
                } else {
                    covariance[row][column] / denominator
                };
                coefficients[row][column] = coefficient;
                coefficients[column][row] = coefficient;
            }
        }

        Self::new(
            SpatialCatalogStandardUncertainties::from_canonical_values(uncertainties)?,
            CorrelationMatrix::try_from_coefficients(coefficients)?,
        )
    }

    #[cfg(feature = "std")]
    fn propagated_by(self, jacobian: SpatialCatalogJacobian) -> Result<Self, Error> {
        let covariance = self.canonical_matrix();
        let derivatives = jacobian.canonical_matrix;
        let mut left_product =
            [[0.0; SPATIAL_CATALOG_PARAMETER_COUNT]; SPATIAL_CATALOG_PARAMETER_COUNT];
        for row in 0..SPATIAL_CATALOG_PARAMETER_COUNT {
            for column in 0..SPATIAL_CATALOG_PARAMETER_COUNT {
                for index in 0..SPATIAL_CATALOG_PARAMETER_COUNT {
                    left_product[row][column] +=
                        derivatives[row][index] * covariance[index][column];
                }
            }
        }

        let mut propagated =
            [[0.0; SPATIAL_CATALOG_PARAMETER_COUNT]; SPATIAL_CATALOG_PARAMETER_COUNT];
        for row in 0..SPATIAL_CATALOG_PARAMETER_COUNT {
            for column in 0..SPATIAL_CATALOG_PARAMETER_COUNT {
                for index in 0..SPATIAL_CATALOG_PARAMETER_COUNT {
                    propagated[row][column] +=
                        left_product[row][index] * derivatives[column][index];
                }
            }
        }
        let mut row = 0;
        while row < SPATIAL_CATALOG_PARAMETER_COUNT {
            let mut column = row + 1;
            while column < SPATIAL_CATALOG_PARAMETER_COUNT {
                let symmetric = (propagated[row][column] + propagated[column][row]) * 0.5;
                propagated[row][column] = symmetric;
                propagated[column][row] = symmetric;
                column += 1;
            }
            row += 1;
        }
        Self::from_canonical_matrix(propagated)
    }
}

/// Numerical Jacobian for one six-parameter catalog propagation.
///
/// Rows and columns use [`SpatialCatalogParameter::ALL`]. Each element is the
/// derivative of the output parameter's canonical value with respect to the
/// input parameter's canonical value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialCatalogJacobian {
    canonical_matrix: [[f64; SPATIAL_CATALOG_PARAMETER_COUNT]; SPATIAL_CATALOG_PARAMETER_COUNT],
}

impl SpatialCatalogJacobian {
    /// Returns the identity Jacobian.
    pub const fn identity() -> Self {
        let mut canonical_matrix =
            [[0.0; SPATIAL_CATALOG_PARAMETER_COUNT]; SPATIAL_CATALOG_PARAMETER_COUNT];
        let mut index = 0;
        while index < SPATIAL_CATALOG_PARAMETER_COUNT {
            canonical_matrix[index][index] = 1.0;
            index += 1;
        }
        Self { canonical_matrix }
    }

    /// Returns one derivative in output-canonical-units per input-canonical-unit.
    pub const fn canonical_derivative(
        self,
        output: SpatialCatalogParameter,
        input: SpatialCatalogParameter,
    ) -> f64 {
        self.canonical_matrix[output.index()][input.index()]
    }

    /// Returns the complete canonical derivative matrix.
    pub const fn canonical_matrix(
        self,
    ) -> [[f64; SPATIAL_CATALOG_PARAMETER_COUNT]; SPATIAL_CATALOG_PARAMETER_COUNT] {
        self.canonical_matrix
    }
}

/// A finite-distance catalog place together with its complete covariance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialCatalogPlaceWithCovariance {
    place: SpatialCatalogPlace,
    covariance: SpatialCatalogCovariance,
}

impl SpatialCatalogPlaceWithCovariance {
    /// Associates a catalog place with covariance in its local tangent basis.
    pub fn new(
        place: SpatialCatalogPlace,
        covariance: SpatialCatalogCovariance,
    ) -> Result<Self, Error> {
        ensure_tangent_plane(place)?;
        Ok(Self { place, covariance })
    }

    /// Returns the central catalog place.
    pub const fn place(self) -> SpatialCatalogPlace {
        self.place
    }

    /// Returns the covariance at the place's reference epoch.
    pub const fn covariance(self) -> SpatialCatalogCovariance {
        self.covariance
    }

    /// Propagates the central place and covariance to another TCB epoch.
    ///
    /// The Jacobian is evaluated with a symmetric five-point stencil around
    /// the existing SOFA `starpm` model in local $\alpha*$ tangent bases.
    /// A non-identity propagation performs one central and four perturbed
    /// `starpm` evaluations per parameter, for 25 model evaluations without
    /// heap allocation.
    #[cfg(feature = "std")]
    pub fn propagate_to(self, epoch: JulianDate<Tcb>) -> Result<SpatialCatalogPropagation, Error> {
        if epoch == self.place.reference_epoch() {
            return Ok(SpatialCatalogPropagation {
                result: self,
                jacobian: SpatialCatalogJacobian::identity(),
            });
        }
        let propagated_place = self.place.propagate_to(epoch)?;
        ensure_tangent_plane(propagated_place)?;
        let jacobian = numerical_jacobian(self.place, propagated_place, epoch)?;
        let covariance = self.covariance.propagated_by(jacobian)?;
        Ok(SpatialCatalogPropagation {
            result: Self::new(propagated_place, covariance)?,
            jacobian,
        })
    }
}

impl SpatialCatalogPlace {
    /// Associates this place with a complete covariance.
    pub fn with_covariance(
        self,
        covariance: SpatialCatalogCovariance,
    ) -> Result<SpatialCatalogPlaceWithCovariance, Error> {
        SpatialCatalogPlaceWithCovariance::new(self, covariance)
    }
}

/// Result and numerical evidence from six-parameter covariance propagation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialCatalogPropagation {
    result: SpatialCatalogPlaceWithCovariance,
    jacobian: SpatialCatalogJacobian,
}

impl SpatialCatalogPropagation {
    /// Returns the propagated place and covariance.
    pub const fn result(self) -> SpatialCatalogPlaceWithCovariance {
        self.result
    }

    /// Returns the numerical local-tangent-plane Jacobian used for propagation.
    pub const fn jacobian(self) -> SpatialCatalogJacobian {
        self.jacobian
    }
}

fn ensure_tangent_plane(place: SpatialCatalogPlace) -> Result<(), Error> {
    let declination = place.direction().declination().as_radians();
    if cos(declination).abs() <= 1.0e-8 {
        Err(Error::UndefinedCatalogTangentPlane {
            declination_radians: declination,
        })
    } else {
        Ok(())
    }
}

#[cfg(feature = "std")]
fn numerical_jacobian(
    source: SpatialCatalogPlace,
    propagated: SpatialCatalogPlace,
    epoch: JulianDate<Tcb>,
) -> Result<SpatialCatalogJacobian, Error> {
    let mut canonical_matrix =
        [[0.0; SPATIAL_CATALOG_PARAMETER_COUNT]; SPATIAL_CATALOG_PARAMETER_COUNT];
    for input in SpatialCatalogParameter::ALL {
        let step = finite_difference_step(source, input);
        let plus_two = perturb(source, input, 2.0 * step)?.propagate_to(epoch)?;
        let plus_one = perturb(source, input, step)?.propagate_to(epoch)?;
        let minus_one = perturb(source, input, -step)?.propagate_to(epoch)?;
        let minus_two = perturb(source, input, -2.0 * step)?.propagate_to(epoch)?;
        let plus_two = local_offsets(propagated, plus_two);
        let plus_one = local_offsets(propagated, plus_one);
        let minus_one = local_offsets(propagated, minus_one);
        let minus_two = local_offsets(propagated, minus_two);
        for output in SpatialCatalogParameter::ALL {
            let row = output.index();
            canonical_matrix[row][input.index()] =
                (-plus_two[row] + 8.0 * plus_one[row] - 8.0 * minus_one[row] + minus_two[row])
                    / (12.0 * step);
        }
    }
    Ok(SpatialCatalogJacobian { canonical_matrix })
}

#[cfg(feature = "std")]
fn finite_difference_step(place: SpatialCatalogPlace, parameter: SpatialCatalogParameter) -> f64 {
    match parameter {
        SpatialCatalogParameter::RightAscensionTangentPlane => {
            1.0e-7_f64.min(cos(place.direction().declination().as_radians()).abs() * 1.0e-4)
        }
        SpatialCatalogParameter::Declination => {
            let margin = FRAC_PI_2 - place.direction().declination().as_radians().abs();
            1.0e-7_f64.min(margin * 0.2)
        }
        SpatialCatalogParameter::Parallax => {
            let parallax = place.parallax().as_angle().as_radians();
            (parallax * 1.0e-4).max(1.0e-12).min(parallax * 0.2)
        }
        SpatialCatalogParameter::RightAscensionProperMotion => place
            .proper_motion()
            .right_ascension_cos_declination()
            .as_radians_per_second()
            .abs()
            .mul_add(1.0e-4, 0.0)
            .max(1.0e-20),
        SpatialCatalogParameter::DeclinationProperMotion => place
            .proper_motion()
            .declination()
            .as_radians_per_second()
            .abs()
            .mul_add(1.0e-4, 0.0)
            .max(1.0e-20),
        SpatialCatalogParameter::RadialVelocity => place
            .radial_velocity()
            .as_metres_per_second()
            .abs()
            .mul_add(1.0e-5, 0.0)
            .max(1.0e-2),
    }
}

#[cfg(feature = "std")]
fn perturb(
    place: SpatialCatalogPlace,
    parameter: SpatialCatalogParameter,
    offset: f64,
) -> Result<SpatialCatalogPlace, Error> {
    let direction = place.direction();
    let mut right_ascension = direction.right_ascension().as_radians();
    let mut declination = direction.declination().as_radians();
    let mut parallax = place.parallax().as_angle().as_radians();
    let mut proper_motion_right_ascension = place
        .proper_motion()
        .right_ascension_cos_declination()
        .as_radians_per_second();
    let mut proper_motion_declination = place.proper_motion().declination().as_radians_per_second();
    let mut radial_velocity = place.radial_velocity().as_metres_per_second();

    match parameter {
        SpatialCatalogParameter::RightAscensionTangentPlane => {
            right_ascension += offset / cos(declination);
        }
        SpatialCatalogParameter::Declination => declination += offset,
        SpatialCatalogParameter::Parallax => parallax += offset,
        SpatialCatalogParameter::RightAscensionProperMotion => {
            proper_motion_right_ascension += offset;
        }
        SpatialCatalogParameter::DeclinationProperMotion => {
            proper_motion_declination += offset;
        }
        SpatialCatalogParameter::RadialVelocity => radial_velocity += offset,
    }

    Ok(SpatialCatalogPlace::new(
        place.reference_epoch(),
        EquatorialDirection::<Icrs>::new(
            RightAscension::wrap_radians(right_ascension)?,
            Declination::try_from_radians(declination)?,
        ),
        CatalogProperMotion::new(
            AngularSpeed::from_radians_per_second(proper_motion_right_ascension)?,
            AngularSpeed::from_radians_per_second(proper_motion_declination)?,
        ),
        Parallax::from_angle(Angle::from_radians(parallax)?)?,
        CatalogRadialVelocity::new(Speed::from_metres_per_second(radial_velocity)?),
    ))
}

#[cfg(feature = "std")]
fn local_offsets(
    reference: SpatialCatalogPlace,
    value: SpatialCatalogPlace,
) -> [f64; SPATIAL_CATALOG_PARAMETER_COUNT] {
    let reference_direction = reference.direction();
    let value_direction = value.direction();
    let right_ascension_difference = signed_angle_difference(
        value_direction.right_ascension().as_radians(),
        reference_direction.right_ascension().as_radians(),
    );
    [
        right_ascension_difference * cos(reference_direction.declination().as_radians()),
        value_direction.declination().as_radians() - reference_direction.declination().as_radians(),
        value.parallax().as_angle().as_radians() - reference.parallax().as_angle().as_radians(),
        value
            .proper_motion()
            .right_ascension_cos_declination()
            .as_radians_per_second()
            - reference
                .proper_motion()
                .right_ascension_cos_declination()
                .as_radians_per_second(),
        value.proper_motion().declination().as_radians_per_second()
            - reference
                .proper_motion()
                .declination()
                .as_radians_per_second(),
        value.radial_velocity().as_metres_per_second()
            - reference.radial_velocity().as_metres_per_second(),
    ]
}

#[cfg(feature = "std")]
fn signed_angle_difference(value: f64, reference: f64) -> f64 {
    let difference = (value - reference).rem_euclid(TAU);
    if difference > PI {
        difference - TAU
    } else {
        difference
    }
}
