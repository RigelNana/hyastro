use crate::{
    ephem::SphericalBodyFigure,
    frame::Bcrs,
    math::{Angle, Direction, Length, Vector3},
    time::{Instant, TimeScale},
};

use super::{ApparentSemidiameter, Error};

/// Whether and why the solar point-mass correction was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SolarDeflectionDisposition {
    /// The ray was corrected by the solar point-mass model.
    Applied,
    /// The ray was corrected and its finite source lies in front of the apparent solar disk.
    AppliedToForegroundTarget,
    /// No correction was applied because the observed target is the deflecting Sun.
    NotAppliedToSun,
    /// No correction was applied because the opaque solar figure blocks the target centre.
    NotAppliedToOccultedTarget,
}

/// Diagnostics for one finite-distance solar light-deflection evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarLightDeflection<S: TimeScale> {
    deflector_epoch: Instant<S>,
    solar_distance: Length,
    solar_separation: Angle,
    solar_semidiameter: ApparentSemidiameter,
    solar_limb_clearance: Angle,
    correction: Angle,
    disposition: SolarDeflectionDisposition,
}

impl<S: TimeScale> SolarLightDeflection<S> {
    /// Exact production algorithm and coefficient-set identifier.
    pub const MODEL: &'static str =
        "IAU SOFA ld finite-source solar monopole via sofars 0.6.1; IAU 2015 nominal solar radius";

    const SOFA_SOLAR_MASS: f64 = 1.0;
    const SOFA_SOLAR_DEFLECTION_LIMITER: f64 = 6.0e-6;

    pub(crate) fn for_sun(
        deflector_epoch: Instant<S>,
        source_direction: Direction<Bcrs>,
        solar_distance: Length,
    ) -> Result<(Direction<Bcrs>, Self), Error> {
        let solar_semidiameter = ApparentSemidiameter::from_spherical_figure(
            SphericalBodyFigure::IAU_2015_NOMINAL_SUN,
            solar_distance,
        )?;
        Ok((
            source_direction,
            Self {
                deflector_epoch,
                solar_distance,
                solar_separation: Angle::from_finite(0.0),
                solar_semidiameter,
                solar_limb_clearance: Angle::from_finite(-solar_semidiameter.as_radians()),
                correction: Angle::from_finite(0.0),
                disposition: SolarDeflectionDisposition::NotAppliedToSun,
            },
        ))
    }

    pub(crate) fn apply_to(
        deflector_epoch: Instant<S>,
        source_direction: Direction<Bcrs>,
        target_distance: Length,
        sun_to_observer: Vector3<Bcrs, Length>,
        sun_to_target: Vector3<Bcrs, Length>,
    ) -> Result<(Direction<Bcrs>, Self), Error> {
        let solar_distance = sun_to_observer.magnitude()?;
        let sun_to_observer_direction = sun_to_observer.direction()?;
        let observer_to_sun_components = sun_to_observer_direction.components().map(|value| -value);
        let observer_to_sun = Direction::<Bcrs>::try_from_components(observer_to_sun_components)?;
        let solar_separation = source_direction.angle_to(observer_to_sun)?;
        let solar_semidiameter = ApparentSemidiameter::from_spherical_figure(
            SphericalBodyFigure::IAU_2015_NOMINAL_SUN,
            solar_distance,
        )?;
        let solar_limb_clearance =
            Angle::from_finite(solar_separation.as_radians() - solar_semidiameter.as_radians());
        let centre_overlaps_sun = solar_limb_clearance.as_radians() < 0.0;
        let sun_is_foreground = solar_distance.as_metres() < target_distance.as_metres();

        if centre_overlaps_sun && sun_is_foreground {
            return Ok((
                source_direction,
                Self {
                    deflector_epoch,
                    solar_distance,
                    solar_separation,
                    solar_semidiameter,
                    solar_limb_clearance,
                    correction: Angle::from_finite(0.0),
                    disposition: SolarDeflectionDisposition::NotAppliedToOccultedTarget,
                },
            ));
        }

        let corrected_direction = Self::apply_sofa_model(
            Self::SOFA_SOLAR_MASS,
            source_direction,
            sun_to_target.direction()?,
            sun_to_observer_direction,
            solar_distance.as_astronomical_units(),
            Self::SOFA_SOLAR_DEFLECTION_LIMITER,
        )?;
        let correction = source_direction.angle_to(corrected_direction)?;
        let disposition = if centre_overlaps_sun {
            SolarDeflectionDisposition::AppliedToForegroundTarget
        } else {
            SolarDeflectionDisposition::Applied
        };
        Ok((
            corrected_direction,
            Self {
                deflector_epoch,
                solar_distance,
                solar_separation,
                solar_semidiameter,
                solar_limb_clearance,
                correction,
                disposition,
            },
        ))
    }

    fn apply_sofa_model(
        mass_in_solar_masses: f64,
        observer_to_source: Direction<Bcrs>,
        sun_to_source: Direction<Bcrs>,
        sun_to_observer: Direction<Bcrs>,
        sun_observer_distance_au: f64,
        deflection_limiter: f64,
    ) -> Result<Direction<Bcrs>, Error> {
        let corrected = Self::apply_sofa_model_components(
            mass_in_solar_masses,
            observer_to_source.components(),
            sun_to_source.components(),
            sun_to_observer.components(),
            sun_observer_distance_au,
            deflection_limiter,
        );
        Direction::try_from_components(corrected).map_err(Error::from)
    }

    fn apply_sofa_model_components(
        mass_in_solar_masses: f64,
        observer_to_source: [f64; 3],
        sun_to_source: [f64; 3],
        sun_to_observer: [f64; 3],
        sun_observer_distance_au: f64,
        deflection_limiter: f64,
    ) -> [f64; 3] {
        sofars::astro::ld(
            mass_in_solar_masses,
            observer_to_source,
            sun_to_source,
            sun_to_observer,
            sun_observer_distance_au,
            deflection_limiter,
        )
    }

    /// Returns the epoch used for the Sun at the target ray's solar passage.
    pub const fn deflector_epoch(self) -> Instant<S> {
        self.deflector_epoch
    }

    /// Returns the distance from the solar centre at ray passage to the observer at reception.
    pub const fn solar_distance(self) -> Length {
        self.solar_distance
    }

    /// Returns the uncorrected target-centre separation from the solar centre.
    pub const fn solar_separation(self) -> Angle {
        self.solar_separation
    }

    /// Returns the nominal solar apparent semidiameter at the retained distance.
    pub const fn solar_semidiameter(self) -> ApparentSemidiameter {
        self.solar_semidiameter
    }

    /// Returns target-centre separation minus the solar semidiameter.
    pub const fn solar_limb_clearance(self) -> Angle {
        self.solar_limb_clearance
    }

    /// Returns the angular change made to the source direction.
    pub const fn correction(self) -> Angle {
        self.correction
    }

    /// Returns whether the model was applied and any physical exclusion reason.
    pub const fn disposition(self) -> SolarDeflectionDisposition {
        self.disposition
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn sofa_ld_reference_vector_is_preserved() {
        let actual = SolarLightDeflection::<crate::time::Tdb>::apply_sofa_model_components(
            0.000_285_74,
            [-0.763_276_255, -0.608_633_767, -0.216_735_543],
            [-0.763_276_255, -0.608_633_767, -0.216_735_543],
            [0.767_004_21, 0.605_629_598, 0.211_937_094],
            8.912_769_83,
            3.0e-10,
        );

        assert_abs_diff_eq!(actual[0], -0.763_276_254_896_815_9, epsilon = 1.0e-12);
        assert_abs_diff_eq!(actual[1], -0.608_633_767_082_376_3, epsilon = 1.0e-12);
        assert_abs_diff_eq!(actual[2], -0.216_735_543_132_054_7, epsilon = 1.0e-12);
    }

    #[test]
    fn finite_source_geometry_distinguishes_occulted_and_foreground_targets() {
        let epoch = Instant::<crate::time::Tdb>::from_tai_nanoseconds(0);
        let astronomical_unit = Length::METRES_PER_AU;
        let separation = 0.003_f64;
        let source =
            Direction::try_from_components([-separation.cos(), separation.sin(), 0.0]).unwrap();
        let sun_to_observer = Vector3::new(
            Length::from_metres(astronomical_unit).unwrap(),
            Length::from_metres(0.0).unwrap(),
            Length::from_metres(0.0).unwrap(),
        );

        let background_distance = 2.0 * astronomical_unit;
        let background_sun_to_target = Vector3::new(
            Length::from_metres(source.components()[0] * background_distance + astronomical_unit)
                .unwrap(),
            Length::from_metres(source.components()[1] * background_distance).unwrap(),
            Length::from_metres(0.0).unwrap(),
        );
        let (background_direction, background) = SolarLightDeflection::apply_to(
            epoch,
            source,
            Length::from_metres(background_distance).unwrap(),
            sun_to_observer,
            background_sun_to_target,
        )
        .unwrap();
        assert_eq!(background_direction, source);
        assert_eq!(
            background.disposition(),
            SolarDeflectionDisposition::NotAppliedToOccultedTarget
        );
        assert!(background.solar_limb_clearance().as_radians() < 0.0);
        assert_eq!(background.correction().as_radians(), 0.0);

        let foreground_distance = 0.5 * astronomical_unit;
        let foreground_sun_to_target = Vector3::new(
            Length::from_metres(source.components()[0] * foreground_distance + astronomical_unit)
                .unwrap(),
            Length::from_metres(source.components()[1] * foreground_distance).unwrap(),
            Length::from_metres(0.0).unwrap(),
        );
        let (foreground_direction, foreground) = SolarLightDeflection::apply_to(
            epoch,
            source,
            Length::from_metres(foreground_distance).unwrap(),
            sun_to_observer,
            foreground_sun_to_target,
        )
        .unwrap();
        assert_ne!(foreground_direction, source);
        assert_eq!(
            foreground.disposition(),
            SolarDeflectionDisposition::AppliedToForegroundTarget
        );
        assert!(foreground.correction().as_radians() > 0.0);
    }
}
