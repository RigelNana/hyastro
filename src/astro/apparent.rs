use libm::asin;

use crate::{
    ephem::SphericalBodyFigure,
    math::{Angle, Length},
    time::TimeScale,
};

use super::{Error, VacuumObservedPlace};

/// Angular radius of a spherical body's vacuum apparent disk.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ApparentSemidiameter(Angle);

impl ApparentSemidiameter {
    /// Computes an exact spherical semidiameter from figure radius and centre distance.
    ///
    /// This uses `asin(radius / distance)`, not a small-angle approximation.
    pub fn from_spherical_figure(
        figure: SphericalBodyFigure,
        distance: Length,
    ) -> Result<Self, Error> {
        let radius_metres = figure.radius().as_metres();
        let distance_metres = distance.as_metres();
        if distance_metres <= radius_metres {
            return Err(Error::ObserverNotOutsideBodyFigure {
                body: figure.body(),
                radius_metres,
                distance_metres,
            });
        }
        Ok(Self(Angle::from_radians(asin(
            radius_metres / distance_metres,
        ))?))
    }

    /// Returns the semidiameter as an unrestricted angle.
    pub const fn as_angle(self) -> Angle {
        self.0
    }

    /// Returns the semidiameter in radians.
    pub const fn as_radians(self) -> f64 {
        self.0.as_radians()
    }

    /// Returns the semidiameter in degrees.
    pub fn as_degrees(self) -> f64 {
        self.0.as_degrees()
    }

    /// Returns twice the semidiameter.
    pub fn diameter(self) -> Angle {
        Angle::from_finite(2.0 * self.0.as_radians())
    }
}

/// Topological relationship between two circular apparent disks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApparentDiskRelationship {
    /// The disks have positive limb-to-limb clearance.
    Separate,
    /// The disks touch at one exterior point.
    ExternallyTangent,
    /// The disks overlap, while neither contains the other.
    PartialOverlap,
    /// The first disk contains and internally touches the second.
    FirstContainsSecondTangentially,
    /// The second disk contains and internally touches the first.
    SecondContainsFirstTangentially,
    /// The first disk strictly contains the second.
    FirstContainsSecond,
    /// The second disk strictly contains the first.
    SecondContainsFirst,
    /// The disks have the same centre and semidiameter.
    Coincident,
}

/// Centre separation, signed exterior-limb clearance, and overlap classification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApparentDiskSeparation {
    centre_separation: Angle,
    limb_clearance: Angle,
    relationship: ApparentDiskRelationship,
}

impl ApparentDiskSeparation {
    fn classify(
        centre_separation: Angle,
        first: ApparentSemidiameter,
        second: ApparentSemidiameter,
    ) -> Self {
        let separation = centre_separation.as_radians();
        let first_radius = first.as_radians();
        let second_radius = second.as_radians();
        let radius_sum = first_radius + second_radius;
        let radius_difference = (first_radius - second_radius).abs();
        let relationship = if separation > radius_sum {
            ApparentDiskRelationship::Separate
        } else if separation == radius_sum {
            ApparentDiskRelationship::ExternallyTangent
        } else if separation > radius_difference {
            ApparentDiskRelationship::PartialOverlap
        } else if separation == 0.0 && first_radius == second_radius {
            ApparentDiskRelationship::Coincident
        } else if separation == radius_difference {
            if first_radius > second_radius {
                ApparentDiskRelationship::FirstContainsSecondTangentially
            } else {
                ApparentDiskRelationship::SecondContainsFirstTangentially
            }
        } else if first_radius > second_radius {
            ApparentDiskRelationship::FirstContainsSecond
        } else {
            ApparentDiskRelationship::SecondContainsFirst
        };

        Self {
            centre_separation,
            limb_clearance: Angle::from_finite(separation - radius_sum),
            relationship,
        }
    }

    /// Returns the angular separation between disk centres.
    pub const fn centre_separation(self) -> Angle {
        self.centre_separation
    }

    /// Returns centre separation minus the sum of both semidiameters.
    ///
    /// Positive values mean separated limbs; negative values mean overlap.
    pub const fn limb_clearance(self) -> Angle {
        self.limb_clearance
    }

    /// Returns the circular-disk overlap classification.
    pub const fn relationship(self) -> ApparentDiskRelationship {
        self.relationship
    }
}

/// One spherical target's topocentric vacuum apparent disk.
///
/// The retained centre place and figure refer to the same body. Atmospheric
/// differential refraction is intentionally absent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VacuumApparentDisk<S: TimeScale> {
    centre: VacuumObservedPlace<S>,
    figure: SphericalBodyFigure,
    semidiameter: ApparentSemidiameter,
}

impl<S: TimeScale> VacuumApparentDisk<S> {
    fn new(centre: VacuumObservedPlace<S>, figure: SphericalBodyFigure) -> Result<Self, Error> {
        if centre.target() != figure.body() {
            return Err(Error::BodyFigureTargetMismatch {
                target: centre.target(),
                figure_body: figure.body(),
            });
        }
        let semidiameter = ApparentSemidiameter::from_spherical_figure(figure, centre.distance())?;
        Ok(Self {
            centre,
            figure,
            semidiameter,
        })
    }

    /// Returns the target's vacuum observed centre place.
    pub const fn centre(self) -> VacuumObservedPlace<S> {
        self.centre
    }

    /// Returns the spherical figure used for the limb.
    pub const fn figure(self) -> SphericalBodyFigure {
        self.figure
    }

    /// Returns the centre-to-limb angular radius.
    pub const fn semidiameter(self) -> ApparentSemidiameter {
        self.semidiameter
    }

    /// Returns the full angular diameter.
    pub fn diameter(self) -> Angle {
        self.semidiameter.diameter()
    }

    /// Compares two disks seen from the same topocentric observer and epoch.
    pub fn separation_from(self, other: Self) -> Result<ApparentDiskSeparation, Error> {
        if self.centre.reception_epoch() != other.centre.reception_epoch() {
            return Err(Error::ApparentDiskEpochMismatch {
                left_tai_nanoseconds: self.centre.reception_epoch().tai_nanoseconds_since_1900(),
                right_tai_nanoseconds: other.centre.reception_epoch().tai_nanoseconds_since_1900(),
            });
        }
        if self.centre.topocentric_frame() != other.centre.topocentric_frame() {
            return Err(Error::ApparentDiskObserverMismatch);
        }
        let left = self
            .centre
            .intermediate_equatorial()
            .coordinates()
            .to_direction()?;
        let right = other
            .centre
            .intermediate_equatorial()
            .coordinates()
            .to_direction()?;
        let centre_separation = left.angle_to(right)?;
        Ok(ApparentDiskSeparation::classify(
            centre_separation,
            self.semidiameter,
            other.semidiameter,
        ))
    }
}

impl<S: TimeScale> VacuumObservedPlace<S> {
    /// Attaches a same-target spherical figure and derives the vacuum apparent disk.
    pub fn apparent_disk(
        self,
        figure: SphericalBodyFigure,
    ) -> Result<VacuumApparentDisk<S>, Error> {
        VacuumApparentDisk::new(self, figure)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;
    use crate::ephem::CelestialBody;

    #[test]
    fn authoritative_spherical_radii_produce_exact_angular_diameters() {
        let sun = ApparentSemidiameter::from_spherical_figure(
            SphericalBodyFigure::IAU_2015_NOMINAL_SUN,
            Length::from_astronomical_units(1.0).unwrap(),
        )
        .unwrap();
        assert_abs_diff_eq!(
            sun.diameter().as_radians(),
            0.009_300_968_047_229_952,
            epsilon = 2.0e-18
        );

        let moon = ApparentSemidiameter::from_spherical_figure(
            SphericalBodyFigure::IAU_WGCCRE_2015_MOON,
            Length::from_kilometres(384_400.0).unwrap(),
        )
        .unwrap();
        assert_abs_diff_eq!(
            moon.diameter().as_radians(),
            0.009_039_572_921_009_154,
            epsilon = 2.0e-18
        );
    }

    #[test]
    fn semidiameter_rejects_an_observer_on_or_inside_the_figure() {
        let figure = SphericalBodyFigure::IAU_WGCCRE_2015_MOON;
        assert!(matches!(
            ApparentSemidiameter::from_spherical_figure(figure, figure.radius()),
            Err(Error::ObserverNotOutsideBodyFigure {
                body: CelestialBody::Moon,
                ..
            })
        ));
    }

    #[test]
    fn disk_relationship_classification_distinguishes_contact_and_containment() {
        let first = ApparentSemidiameter(Angle::from_radians(0.02).unwrap());
        let second = ApparentSemidiameter(Angle::from_radians(0.01).unwrap());

        assert_eq!(
            ApparentDiskSeparation::classify(Angle::from_radians(0.04).unwrap(), first, second,)
                .relationship(),
            ApparentDiskRelationship::Separate
        );
        assert_eq!(
            ApparentDiskSeparation::classify(Angle::from_radians(0.03).unwrap(), first, second,)
                .relationship(),
            ApparentDiskRelationship::ExternallyTangent
        );
        assert_eq!(
            ApparentDiskSeparation::classify(Angle::from_radians(0.02).unwrap(), first, second,)
                .relationship(),
            ApparentDiskRelationship::PartialOverlap
        );
        assert_eq!(
            ApparentDiskSeparation::classify(Angle::from_radians(0.01).unwrap(), first, second,)
                .relationship(),
            ApparentDiskRelationship::FirstContainsSecondTangentially
        );
        assert_eq!(
            ApparentDiskSeparation::classify(Angle::from_radians(0.005).unwrap(), first, second,)
                .relationship(),
            ApparentDiskRelationship::FirstContainsSecond
        );
    }
}
