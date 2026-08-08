use core::fmt;

use crate::{
    constants::earth::{ROTATION_DETERMINANT_TOLERANCE, ROTATION_ORTHOGONALITY_TOLERANCE},
    math::{Angle, Matrix3, Rotation, RotationTolerance},
    time::{Instant, JulianDate, TimeScale, Tt},
};

use super::{
    EclipticDirection, EclipticDirectionAt, EquatorialDirection, EquatorialDirectionAt, Error,
    Gcrs, Icrs, MeanEclipticEquinoxOfDate, MeanEquatorEquinoxOfDate, PrecessionNutation,
    TrueEclipticEquinoxOfDate, TrueEquatorEquinoxOfDate,
};

/// IAU 2006 precession, IAU 2000A nutation, and mean-ecliptic orientation at one epoch.
///
/// The solution is driven entirely by TT. It does not apply observed celestial-pole offsets,
/// Earth rotation, polar motion, aberration, parallax, or light deflection.
pub struct CelestialOrientationSolution<S: TimeScale> {
    epoch: Instant<S>,
    terrestrial_time: JulianDate<Tt>,
    precession_nutation: PrecessionNutation,
    gcrs_to_mean_equator: Rotation<Gcrs, MeanEquatorEquinoxOfDate>,
    gcrs_to_true_equator: Rotation<Gcrs, TrueEquatorEquinoxOfDate>,
    icrs_to_mean_ecliptic: Rotation<Icrs, MeanEclipticEquinoxOfDate>,
    icrs_to_true_ecliptic: Rotation<Icrs, TrueEclipticEquinoxOfDate>,
    gcrs_to_true_ecliptic: Rotation<Gcrs, TrueEclipticEquinoxOfDate>,
}

impl<S: TimeScale> CelestialOrientationSolution<S> {
    pub(super) fn at(epoch: Instant<S>, terrestrial_time: JulianDate<Tt>) -> Result<Self, Error> {
        let precession_nutation = PrecessionNutation::at(terrestrial_time)?;
        let (tt_first, tt_second) = terrestrial_time.parts();
        let mut true_ecliptic_matrix = precession_nutation.bias_precession_nutation_matrix().rows();
        sofars::vm::rx(
            precession_nutation.true_obliquity().as_radians(),
            &mut true_ecliptic_matrix,
        );
        Ok(Self {
            epoch,
            terrestrial_time,
            precession_nutation,
            gcrs_to_mean_equator: Self::rotation_from_matrix(
                precession_nutation.bias_precession_matrix(),
            )?,
            gcrs_to_true_equator: Self::rotation_from_matrix(
                precession_nutation.bias_precession_nutation_matrix(),
            )?,
            icrs_to_mean_ecliptic: Self::rotation_from_matrix(Matrix3::try_from_rows(
                sofars::coords::ecm06(tt_first, tt_second),
            )?)?,
            icrs_to_true_ecliptic: Self::rotation_from_matrix(Matrix3::try_from_rows(
                true_ecliptic_matrix,
            )?)?,
            gcrs_to_true_ecliptic: Self::rotation_from_matrix(Matrix3::try_from_rows(
                true_ecliptic_matrix,
            )?)?,
        })
    }

    /// Returns the physical epoch represented by every result in this solution.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the two-part TT date used by every orientation model.
    pub const fn terrestrial_time(self) -> JulianDate<Tt> {
        self.terrestrial_time
    }

    /// Returns the underlying IAU 2006/2000A precession-nutation values.
    pub const fn precession_nutation(self) -> PrecessionNutation {
        self.precession_nutation
    }

    /// Returns the IAU 2006 mean obliquity.
    pub const fn mean_obliquity(self) -> Angle {
        self.precession_nutation.mean_obliquity()
    }

    /// Returns the true obliquity from the IAU 2006/2000A nutation model.
    pub const fn true_obliquity(self) -> Angle {
        self.precession_nutation.true_obliquity()
    }

    /// Converts a GCRS direction to mean equator and equinox of date coordinates.
    pub fn mean_equatorial(
        self,
        source: EquatorialDirection<Gcrs>,
    ) -> Result<EquatorialDirectionAt<MeanEquatorEquinoxOfDate, S>, Error> {
        let direction = self
            .gcrs_to_mean_equator
            .apply_direction(source.to_direction()?)?;
        Ok(EquatorialDirectionAt::new(
            self.epoch,
            EquatorialDirection::from_direction(direction)?,
        ))
    }

    /// Converts mean equator and equinox of date coordinates back to GCRS.
    pub fn gcrs_from_mean_equatorial(
        self,
        source: EquatorialDirectionAt<MeanEquatorEquinoxOfDate, S>,
    ) -> Result<EquatorialDirection<Gcrs>, Error> {
        self.ensure_epoch(source.epoch())?;
        let direction = self
            .gcrs_to_mean_equator
            .inverse()
            .apply_direction(source.coordinates().to_direction()?)?;
        EquatorialDirection::from_direction(direction).map_err(Error::from)
    }

    /// Converts a GCRS direction to true equator and equinox of date coordinates.
    pub fn true_equatorial(
        self,
        source: EquatorialDirection<Gcrs>,
    ) -> Result<EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>, Error> {
        let direction = self
            .gcrs_to_true_equator
            .apply_direction(source.to_direction()?)?;
        Ok(EquatorialDirectionAt::new(
            self.epoch,
            EquatorialDirection::from_direction(direction)?,
        ))
    }

    /// Converts true equator and equinox of date coordinates back to GCRS.
    pub fn gcrs_from_true_equatorial(
        self,
        source: EquatorialDirectionAt<TrueEquatorEquinoxOfDate, S>,
    ) -> Result<EquatorialDirection<Gcrs>, Error> {
        self.ensure_epoch(source.epoch())?;
        let direction = self
            .gcrs_to_true_equator
            .inverse()
            .apply_direction(source.coordinates().to_direction()?)?;
        EquatorialDirection::from_direction(direction).map_err(Error::from)
    }

    /// Converts an ICRS direction to IAU 2006 mean ecliptic and equinox of date coordinates.
    pub fn mean_ecliptic(
        self,
        source: EquatorialDirection<Icrs>,
    ) -> Result<EclipticDirectionAt<MeanEclipticEquinoxOfDate, S>, Error> {
        let direction = self
            .icrs_to_mean_ecliptic
            .apply_direction(source.to_direction()?)?;
        Ok(EclipticDirectionAt::new(
            self.epoch,
            EclipticDirection::from_direction(direction)?,
        ))
    }

    /// Converts mean ecliptic and equinox of date coordinates back to ICRS.
    pub fn icrs_from_mean_ecliptic(
        self,
        source: EclipticDirectionAt<MeanEclipticEquinoxOfDate, S>,
    ) -> Result<EquatorialDirection<Icrs>, Error> {
        self.ensure_epoch(source.epoch())?;
        let direction = self
            .icrs_to_mean_ecliptic
            .inverse()
            .apply_direction(source.coordinates().to_direction()?)?;
        EquatorialDirection::from_direction(direction).map_err(Error::from)
    }

    /// Converts an ICRS direction to conventional true ecliptic and equinox of date coordinates.
    ///
    /// The axes include IAU 2006 frame bias and precession, IAU 2000A
    /// nutation, and true obliquity. The direction itself receives no
    /// light-time, aberration, parallax, or light-deflection correction.
    pub fn true_ecliptic(
        self,
        source: EquatorialDirection<Icrs>,
    ) -> Result<EclipticDirectionAt<TrueEclipticEquinoxOfDate, S>, Error> {
        let direction = self
            .icrs_to_true_ecliptic
            .apply_direction(source.to_direction()?)?;
        Ok(EclipticDirectionAt::new(
            self.epoch,
            EclipticDirection::from_direction(direction)?,
        ))
    }

    /// Converts true ecliptic and equinox of date coordinates back to ICRS.
    pub fn icrs_from_true_ecliptic(
        self,
        source: EclipticDirectionAt<TrueEclipticEquinoxOfDate, S>,
    ) -> Result<EquatorialDirection<Icrs>, Error> {
        self.ensure_epoch(source.epoch())?;
        let direction = self
            .icrs_to_true_ecliptic
            .inverse()
            .apply_direction(source.coordinates().to_direction()?)?;
        EquatorialDirection::from_direction(direction).map_err(Error::from)
    }

    /// Converts a GCRS proper direction to conventional true ecliptic coordinates of date.
    ///
    /// This is the frame-rotation stage used after geocentric aberration.
    /// The method does not itself apply light-time, aberration, parallax, or
    /// light-deflection corrections.
    pub fn true_ecliptic_from_gcrs(
        self,
        source: EquatorialDirection<Gcrs>,
    ) -> Result<EclipticDirectionAt<TrueEclipticEquinoxOfDate, S>, Error> {
        let direction = self
            .gcrs_to_true_ecliptic
            .apply_direction(source.to_direction()?)?;
        Ok(EclipticDirectionAt::new(
            self.epoch,
            EclipticDirection::from_direction(direction)?,
        ))
    }

    /// Converts true ecliptic coordinates of date back to a GCRS proper direction.
    pub fn gcrs_from_true_ecliptic(
        self,
        source: EclipticDirectionAt<TrueEclipticEquinoxOfDate, S>,
    ) -> Result<EquatorialDirection<Gcrs>, Error> {
        self.ensure_epoch(source.epoch())?;
        let direction = self
            .gcrs_to_true_ecliptic
            .inverse()
            .apply_direction(source.coordinates().to_direction()?)?;
        EquatorialDirection::from_direction(direction).map_err(Error::from)
    }

    fn ensure_epoch(self, value: Instant<S>) -> Result<(), Error> {
        let solution_tai_nanoseconds = self.epoch.tai_nanoseconds_since_1900();
        let value_tai_nanoseconds = value.tai_nanoseconds_since_1900();
        if solution_tai_nanoseconds == value_tai_nanoseconds {
            Ok(())
        } else {
            Err(Error::epoch_mismatch(
                solution_tai_nanoseconds,
                value_tai_nanoseconds,
            ))
        }
    }

    fn rotation_from_matrix<From, To>(matrix: Matrix3) -> Result<Rotation<From, To>, Error> {
        let tolerance = RotationTolerance::new(
            ROTATION_ORTHOGONALITY_TOLERANCE,
            ROTATION_DETERMINANT_TOLERANCE,
        )?;
        Rotation::try_from_matrix(matrix, tolerance).map_err(Error::from)
    }
}

impl<S: TimeScale> Copy for CelestialOrientationSolution<S> {}

impl<S: TimeScale> Clone for CelestialOrientationSolution<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> fmt::Debug for CelestialOrientationSolution<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CelestialOrientationSolution")
            .field("epoch", &self.epoch)
            .field("terrestrial_time", &self.terrestrial_time)
            .field("precession_nutation", &self.precession_nutation)
            .field("gcrs_to_mean_equator", &self.gcrs_to_mean_equator)
            .field("gcrs_to_true_equator", &self.gcrs_to_true_equator)
            .field("icrs_to_mean_ecliptic", &self.icrs_to_mean_ecliptic)
            .field("icrs_to_true_ecliptic", &self.icrs_to_true_ecliptic)
            .field("gcrs_to_true_ecliptic", &self.gcrs_to_true_ecliptic)
            .finish()
    }
}
