use core::fmt;

use crate::{
    constants::earth::{
        ROTATION_DETERMINANT_TOLERANCE, ROTATION_ORTHOGONALITY_TOLERANCE,
        ROTATION_RATE_CONVERGENCE_TOLERANCE_RADIANS_PER_SECOND,
        ROTATION_RATE_DIFFERENCE_STEP_SECONDS,
    },
    math::{
        Angle, AngularSpeed, HourAngle, Length, Longitude, Matrix3, Rotation, RotationTolerance,
        Speed, Vector3,
    },
    time::{
        Duration, EarthAttitudeModel, EarthAttitudeState, EarthOrientation, EarthOrientationTable,
        Instant, JulianDate, TimeContext, TimeScale, TimeScaleModel, Tt, Ut1,
    },
};

use super::{
    Cirs, CoordinateFrame, EquatorialDirection, EquatorialDirectionAt, Error, FrameRotation, Gcrs,
    Itrs, StateTransform, Tirs,
};

/// Fukushima-Williams angles for IAU 2006 bias and precession.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FukushimaWilliamsAngles {
    gamma_bar: Angle,
    phi_bar: Angle,
    psi_bar: Angle,
    mean_obliquity: Angle,
}

impl FukushimaWilliamsAngles {
    fn at(tt: JulianDate<Tt>) -> Result<Self, Error> {
        let (tt_first, tt_second) = tt.parts();
        let (gamma_bar, phi_bar, psi_bar, mean_obliquity) = sofars::pnp::pfw06(tt_first, tt_second);
        Ok(Self {
            gamma_bar: Angle::from_radians(gamma_bar)?,
            phi_bar: Angle::from_radians(phi_bar)?,
            psi_bar: Angle::from_radians(psi_bar)?,
            mean_obliquity: Angle::from_radians(mean_obliquity)?,
        })
    }

    /// Returns the Fukushima-Williams $\bar{\gamma}$ angle.
    pub const fn gamma_bar(self) -> Angle {
        self.gamma_bar
    }

    /// Returns the Fukushima-Williams $\bar{\phi}$ angle.
    pub const fn phi_bar(self) -> Angle {
        self.phi_bar
    }

    /// Returns the Fukushima-Williams $\bar{\psi}$ angle.
    pub const fn psi_bar(self) -> Angle {
        self.psi_bar
    }

    /// Returns the IAU 2006 mean obliquity $\epsilon_A$.
    pub const fn mean_obliquity(self) -> Angle {
        self.mean_obliquity
    }
}

/// IAU 2006 precession and IAU 2000A nutation evaluated at one TT date.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrecessionNutation {
    fukushima_williams: FukushimaWilliamsAngles,
    nutation_longitude: Angle,
    nutation_obliquity: Angle,
    true_obliquity: Angle,
    frame_bias_matrix: Matrix3,
    precession_matrix: Matrix3,
    bias_precession_matrix: Matrix3,
    nutation_matrix: Matrix3,
    bias_precession_nutation_matrix: Matrix3,
}

impl PrecessionNutation {
    pub(super) fn at(tt: JulianDate<Tt>) -> Result<Self, Error> {
        let (tt_first, tt_second) = tt.parts();
        let (
            nutation_longitude,
            nutation_obliquity,
            mean_obliquity,
            frame_bias_matrix,
            precession_matrix,
            bias_precession_matrix,
            nutation_matrix,
            bias_precession_nutation_matrix,
        ) = sofars::pnp::pn06a(tt_first, tt_second);
        let fukushima_williams = FukushimaWilliamsAngles::at(tt)?;

        Ok(Self {
            fukushima_williams,
            nutation_longitude: Angle::from_radians(nutation_longitude)?,
            nutation_obliquity: Angle::from_radians(nutation_obliquity)?,
            true_obliquity: Angle::from_radians(mean_obliquity + nutation_obliquity)?,
            frame_bias_matrix: Matrix3::try_from_rows(frame_bias_matrix)?,
            precession_matrix: Matrix3::try_from_rows(precession_matrix)?,
            bias_precession_matrix: Matrix3::try_from_rows(bias_precession_matrix)?,
            nutation_matrix: Matrix3::try_from_rows(nutation_matrix)?,
            bias_precession_nutation_matrix: Matrix3::try_from_rows(
                bias_precession_nutation_matrix,
            )?,
        })
    }

    /// Returns the IAU 2006 Fukushima-Williams angles.
    pub const fn fukushima_williams(self) -> FukushimaWilliamsAngles {
        self.fukushima_williams
    }

    /// Returns the IAU 2006 mean obliquity $\epsilon_A$.
    pub const fn mean_obliquity(self) -> Angle {
        self.fukushima_williams.mean_obliquity()
    }

    /// Returns IAU 2006/2000A nutation in longitude $\Delta\psi$.
    pub const fn nutation_longitude(self) -> Angle {
        self.nutation_longitude
    }

    /// Returns IAU 2006/2000A nutation in obliquity $\Delta\epsilon$.
    pub const fn nutation_obliquity(self) -> Angle {
        self.nutation_obliquity
    }

    /// Returns true obliquity $\epsilon_A + \Delta\epsilon$.
    pub const fn true_obliquity(self) -> Angle {
        self.true_obliquity
    }

    /// Returns the GCRS-to-mean-J2000 frame-bias matrix.
    pub const fn frame_bias_matrix(self) -> Matrix3 {
        self.frame_bias_matrix
    }

    /// Returns the mean-J2000-to-mean-of-date precession matrix.
    pub const fn precession_matrix(self) -> Matrix3 {
        self.precession_matrix
    }

    /// Returns the GCRS-to-mean-of-date bias-precession matrix.
    pub const fn bias_precession_matrix(self) -> Matrix3 {
        self.bias_precession_matrix
    }

    /// Returns the mean-of-date-to-true-of-date nutation matrix.
    pub const fn nutation_matrix(self) -> Matrix3 {
        self.nutation_matrix
    }

    /// Returns the GCRS-to-true-of-date bias-precession-nutation matrix.
    pub const fn bias_precession_nutation_matrix(self) -> Matrix3 {
        self.bias_precession_nutation_matrix
    }
}

/// IAU 2000/2006 Earth-rotation and sidereal angles at one physical epoch.
///
/// TT drives precession and nutation while UT1 drives Earth rotation. This
/// solution therefore needs only leap seconds and `UT1−UTC`; polar motion,
/// celestial-pole corrections, and length of day are not required.
pub struct SiderealTimeSolution<S: TimeScale> {
    epoch: Instant<S>,
    terrestrial_time: JulianDate<Tt>,
    universal_time: JulianDate<Ut1>,
    earth_rotation_angle: HourAngle,
    greenwich_mean_sidereal_time: HourAngle,
    greenwich_apparent_sidereal_time: HourAngle,
    equation_of_origins: Angle,
    equation_of_equinoxes: Angle,
}

impl<S: TimeScale> SiderealTimeSolution<S> {
    pub(super) fn at<'a, E>(epoch: Instant<S>, time: &TimeContext<'a, E>) -> Result<Self, Error>
    where
        TimeContext<'a, E>: TimeScaleModel<Ut1>,
    {
        let terrestrial_time = JulianDate::<Tt>::from_instant(epoch, time)?;
        let universal_time = JulianDate::<Ut1>::from_instant(epoch, time)?;
        let precession_nutation = PrecessionNutation::at(terrestrial_time)?;
        Self::from_dates(epoch, terrestrial_time, universal_time, precession_nutation)
    }

    fn from_dates(
        epoch: Instant<S>,
        terrestrial_time: JulianDate<Tt>,
        universal_time: JulianDate<Ut1>,
        precession_nutation: PrecessionNutation,
    ) -> Result<Self, Error> {
        let (tt_first, tt_second) = terrestrial_time.parts();
        let (ut1_first, ut1_second) = universal_time.parts();
        let bias_precession_nutation = precession_nutation.bias_precession_nutation_matrix().rows();
        let (model_x, model_y) = sofars::pnp::bpn2xy(&bias_precession_nutation);
        let modeled_cio_locator = sofars::pnp::s06(tt_first, tt_second, model_x, model_y);
        let earth_rotation_angle = sofars::erst::era00(ut1_first, ut1_second);
        let greenwich_mean_sidereal_time =
            sofars::erst::gmst06(ut1_first, ut1_second, tt_first, tt_second);
        let greenwich_apparent_sidereal_time = sofars::erst::gst06(
            ut1_first,
            ut1_second,
            tt_first,
            tt_second,
            &bias_precession_nutation,
        );
        let equation_of_origins = sofars::pnp::eors(&bias_precession_nutation, modeled_cio_locator);
        let equation_of_equinoxes = sofars::erst::ee06a(tt_first, tt_second);

        Ok(Self {
            epoch,
            terrestrial_time,
            universal_time,
            earth_rotation_angle: HourAngle::wrap_radians(earth_rotation_angle)?,
            greenwich_mean_sidereal_time: HourAngle::wrap_radians(greenwich_mean_sidereal_time)?,
            greenwich_apparent_sidereal_time: HourAngle::wrap_radians(
                greenwich_apparent_sidereal_time,
            )?,
            equation_of_origins: Angle::from_radians(equation_of_origins)?,
            equation_of_equinoxes: Angle::from_radians(equation_of_equinoxes)?,
        })
    }

    /// Returns the physical epoch represented by every result in this solution.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the two-part TT date used for precession and nutation.
    pub const fn terrestrial_time(self) -> JulianDate<Tt> {
        self.terrestrial_time
    }

    /// Returns the two-part UT1 date used for Earth rotation.
    pub const fn universal_time(self) -> JulianDate<Ut1> {
        self.universal_time
    }

    /// Returns the IAU 2000 Earth Rotation Angle in $[0,2\pi)$.
    pub const fn earth_rotation_angle(self) -> HourAngle {
        self.earth_rotation_angle
    }

    /// Returns IAU 2006 Greenwich Mean Sidereal Time in $[0,2\pi)$.
    pub const fn greenwich_mean_sidereal_time(self) -> HourAngle {
        self.greenwich_mean_sidereal_time
    }

    /// Returns IAU 2006/2000A Greenwich Apparent Sidereal Time in $[0,2\pi)$.
    pub const fn greenwich_apparent_sidereal_time(self) -> HourAngle {
        self.greenwich_apparent_sidereal_time
    }

    /// Returns local mean sidereal time for an east-positive longitude.
    pub fn local_mean_sidereal_time(self, longitude: Longitude) -> Result<HourAngle, Error> {
        Ok(HourAngle::wrap_radians(
            self.greenwich_mean_sidereal_time.as_radians() + longitude.as_radians(),
        )?)
    }

    /// Returns local apparent sidereal time for an east-positive longitude.
    pub fn local_apparent_sidereal_time(self, longitude: Longitude) -> Result<HourAngle, Error> {
        Ok(HourAngle::wrap_radians(
            self.greenwich_apparent_sidereal_time.as_radians() + longitude.as_radians(),
        )?)
    }

    /// Returns the equation of the origins, ERA minus GAST.
    pub const fn equation_of_origins(self) -> Angle {
        self.equation_of_origins
    }

    /// Returns the equation of the equinoxes, GAST minus GMST.
    pub const fn equation_of_equinoxes(self) -> Angle {
        self.equation_of_equinoxes
    }
}

impl<S: TimeScale> Copy for SiderealTimeSolution<S> {}

impl<S: TimeScale> Clone for SiderealTimeSolution<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> fmt::Debug for SiderealTimeSolution<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SiderealTimeSolution")
            .field("epoch", &self.epoch)
            .field("terrestrial_time", &self.terrestrial_time)
            .field("universal_time", &self.universal_time)
            .field("earth_rotation_angle", &self.earth_rotation_angle)
            .field(
                "greenwich_mean_sidereal_time",
                &self.greenwich_mean_sidereal_time,
            )
            .field(
                "greenwich_apparent_sidereal_time",
                &self.greenwich_apparent_sidereal_time,
            )
            .field("equation_of_origins", &self.equation_of_origins)
            .field("equation_of_equinoxes", &self.equation_of_equinoxes)
            .finish()
    }
}

/// GCRS coordinates of the Celestial Intermediate Pole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CelestialIntermediatePole {
    x: Angle,
    y: Angle,
}

impl CelestialIntermediatePole {
    fn from_radians(x: f64, y: f64) -> Result<Self, Error> {
        Ok(Self {
            x: Angle::from_radians(x)?,
            y: Angle::from_radians(y)?,
        })
    }

    /// Returns the CIP $X$ coordinate.
    pub const fn x(self) -> Angle {
        self.x
    }

    /// Returns the CIP $Y$ coordinate.
    pub const fn y(self) -> Angle {
        self.y
    }
}

#[derive(Debug, Clone, Copy)]
struct EarthAttitudeRotations {
    precession_nutation: PrecessionNutation,
    modeled_cip: CelestialIntermediatePole,
    cip: CelestialIntermediatePole,
    modeled_cio_locator: Angle,
    cio_locator: Angle,
    tio_locator: Angle,
    earth_rotation_angle: HourAngle,
    greenwich_mean_sidereal_time: HourAngle,
    greenwich_apparent_sidereal_time: HourAngle,
    equation_of_origins: Angle,
    equation_of_equinoxes: Angle,
    gcrs_to_cirs: Rotation<Gcrs, Cirs>,
    cirs_to_tirs: Rotation<Cirs, Tirs>,
    tirs_to_itrs: Rotation<Tirs, Itrs>,
    gcrs_to_itrs: Rotation<Gcrs, Itrs>,
    modeled_cio_gcrs_to_tirs_matrix: Matrix3,
    equinox_gcrs_to_tirs_matrix: Matrix3,
}

impl EarthAttitudeRotations {
    fn at<S: TimeScale>(
        epoch: Instant<S>,
        terrestrial_time: JulianDate<Tt>,
        universal_time: JulianDate<Ut1>,
        polar_motion_x: Angle,
        polar_motion_y: Angle,
        celestial_pole_offset_x: Angle,
        celestial_pole_offset_y: Angle,
    ) -> Result<Self, Error> {
        let precession_nutation = PrecessionNutation::at(terrestrial_time)?;
        let sidereal_time = SiderealTimeSolution::from_dates(
            epoch,
            terrestrial_time,
            universal_time,
            precession_nutation,
        )?;
        let (tt_first, tt_second) = terrestrial_time.parts();
        let bias_precession_nutation = precession_nutation.bias_precession_nutation_matrix().rows();
        let (model_x, model_y) = sofars::pnp::bpn2xy(&bias_precession_nutation);
        let modeled_cip = CelestialIntermediatePole::from_radians(model_x, model_y)?;
        let observed_x = model_x + celestial_pole_offset_x.as_radians();
        let observed_y = model_y + celestial_pole_offset_y.as_radians();
        let cip = CelestialIntermediatePole::from_radians(observed_x, observed_y)?;
        let modeled_cio_locator = sofars::pnp::s06(tt_first, tt_second, model_x, model_y);
        let cio_locator = sofars::pnp::s06(tt_first, tt_second, observed_x, observed_y);
        let tio_locator = sofars::pnp::sp00(tt_first, tt_second);
        let earth_rotation_angle = sidereal_time.earth_rotation_angle().as_radians();
        let greenwich_apparent_sidereal_time = sidereal_time
            .greenwich_apparent_sidereal_time()
            .as_radians();
        let modeled_celestial_to_intermediate =
            sofars::pnp::c2ixys(model_x, model_y, modeled_cio_locator);
        let celestial_to_intermediate = sofars::pnp::c2ixys(observed_x, observed_y, cio_locator);
        let polar_motion = sofars::pnp::pom00(
            polar_motion_x.as_radians(),
            polar_motion_y.as_radians(),
            tio_locator,
        );
        let identity = Matrix3::identity().rows();
        let modeled_cio_gcrs_to_tirs = sofars::pnp::c2tcio(
            &modeled_celestial_to_intermediate,
            earth_rotation_angle,
            &identity,
        );
        let equinox_gcrs_to_tirs = sofars::pnp::c2teqx(
            &bias_precession_nutation,
            greenwich_apparent_sidereal_time,
            &identity,
        );
        let gcrs_to_itrs_matrix = sofars::pnp::c2tcio(
            &celestial_to_intermediate,
            earth_rotation_angle,
            &polar_motion,
        );
        let mut cirs_to_tirs_matrix = identity;
        sofars::vm::rz(earth_rotation_angle, &mut cirs_to_tirs_matrix);

        Ok(Self {
            precession_nutation,
            modeled_cip,
            cip,
            modeled_cio_locator: Angle::from_radians(modeled_cio_locator)?,
            cio_locator: Angle::from_radians(cio_locator)?,
            tio_locator: Angle::from_radians(tio_locator)?,
            earth_rotation_angle: sidereal_time.earth_rotation_angle(),
            greenwich_mean_sidereal_time: sidereal_time.greenwich_mean_sidereal_time(),
            greenwich_apparent_sidereal_time: sidereal_time.greenwich_apparent_sidereal_time(),
            equation_of_origins: sidereal_time.equation_of_origins(),
            equation_of_equinoxes: sidereal_time.equation_of_equinoxes(),
            gcrs_to_cirs: Iau2006And2000A::rotation_from_rows(celestial_to_intermediate)?,
            cirs_to_tirs: Iau2006And2000A::rotation_from_rows(cirs_to_tirs_matrix)?,
            tirs_to_itrs: Iau2006And2000A::rotation_from_rows(polar_motion)?,
            gcrs_to_itrs: Iau2006And2000A::rotation_from_rows(gcrs_to_itrs_matrix)?,
            modeled_cio_gcrs_to_tirs_matrix: Matrix3::try_from_rows(modeled_cio_gcrs_to_tirs)?,
            equinox_gcrs_to_tirs_matrix: Matrix3::try_from_rows(equinox_gcrs_to_tirs)?,
        })
    }
}

/// A coherent Earth-attitude solution at one physical epoch.
///
/// The solution includes Delta T, optional tabulated `UT1−UTC`, polar motion,
/// and celestial-pole corrections. It supplies direction rotations but makes
/// no claim about measured frame angular velocity.
#[derive(Debug, Clone, Copy)]
pub struct EarthAttitudeSolution<S: TimeScale> {
    epoch: Instant<S>,
    terrestrial_time: JulianDate<Tt>,
    universal_time: JulianDate<Ut1>,
    attitude: EarthAttitudeState<S>,
    rotations: EarthAttitudeRotations,
}

impl<S: TimeScale> EarthAttitudeSolution<S> {
    pub(super) fn at<E: EarthAttitudeModel>(
        epoch: Instant<S>,
        time: &TimeContext<'_, E>,
    ) -> Result<Self, Error> {
        let attitude = time.earth_attitude_state_at(epoch)?;
        let terrestrial_time = JulianDate::<Tt>::from_instant(epoch, time)?;
        let ut1_coordinate = terrestrial_time
            .checked_add_duration(attitude.delta_t().tt_minus_ut1().checked_neg()?)?;
        let (ut1_first, ut1_second) = ut1_coordinate.parts();
        let universal_time = JulianDate::<Ut1>::from_parts(ut1_first, ut1_second)?;
        let rotations = EarthAttitudeRotations::at(
            epoch,
            terrestrial_time,
            universal_time,
            attitude.polar_motion_x().as_angle(),
            attitude.polar_motion_y().as_angle(),
            attitude.celestial_pole_offset_x().as_angle(),
            attitude.celestial_pole_offset_y().as_angle(),
        )?;
        Ok(Self {
            epoch,
            terrestrial_time,
            universal_time,
            attitude,
            rotations,
        })
    }

    /// Returns the physical epoch represented by every result in this solution.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the two-part TT date used for precession and nutation.
    pub const fn terrestrial_time(self) -> JulianDate<Tt> {
        self.terrestrial_time
    }

    /// Returns the two-part UT1 date used for Earth rotation.
    pub const fn universal_time(self) -> JulianDate<Ut1> {
        self.universal_time
    }

    /// Returns the resolved tabulated or predicted Earth-attitude state.
    pub const fn earth_attitude(self) -> EarthAttitudeState<S> {
        self.attitude
    }

    /// Returns the IAU 2000 TIO locator $s'$.
    pub const fn tio_locator(self) -> Angle {
        self.rotations.tio_locator
    }

    /// Returns the IAU 2000 Earth Rotation Angle in $[0,2\pi)$.
    pub const fn earth_rotation_angle(self) -> HourAngle {
        self.rotations.earth_rotation_angle
    }

    /// Returns the IAU 2006/2000A precession-nutation result.
    pub const fn precession_nutation(self) -> PrecessionNutation {
        self.rotations.precession_nutation
    }

    /// Returns the CIP coordinates after applying the selected `dX,dY`.
    pub const fn cip(self) -> CelestialIntermediatePole {
        self.rotations.cip
    }

    /// Returns the selected GCRS-to-CIRS rotation.
    pub const fn gcrs_to_cirs(self) -> FrameRotation<Gcrs, Cirs, S> {
        FrameRotation::new(self.epoch, self.rotations.gcrs_to_cirs)
    }

    /// Returns the CIRS-to-TIRS Earth-rotation rotation.
    pub const fn cirs_to_tirs(self) -> FrameRotation<Cirs, Tirs, S> {
        FrameRotation::new(self.epoch, self.rotations.cirs_to_tirs)
    }

    /// Returns the selected TIRS-to-ITRS polar-motion rotation.
    pub const fn tirs_to_itrs(self) -> FrameRotation<Tirs, Itrs, S> {
        FrameRotation::new(self.epoch, self.rotations.tirs_to_itrs)
    }

    /// Returns the complete selected GCRS-to-ITRS direction rotation.
    pub const fn gcrs_to_itrs(self) -> FrameRotation<Gcrs, Itrs, S> {
        FrameRotation::new(self.epoch, self.rotations.gcrs_to_itrs)
    }

    /// Converts a GCRS direction to selected CIRS intermediate coordinates.
    pub fn intermediate_equatorial(
        self,
        source: EquatorialDirection<Gcrs>,
    ) -> Result<EquatorialDirectionAt<Cirs, S>, Error> {
        let direction = self
            .rotations
            .gcrs_to_cirs
            .apply_direction(source.to_direction()?)?;
        Ok(EquatorialDirectionAt::new(
            self.epoch,
            EquatorialDirection::from_direction(direction)?,
        ))
    }
}

/// A coherent IAU 2006/2000A Earth-orientation solution at one physical epoch.
///
/// TT drives precession and nutation, UT1 drives Earth rotation, and all EOP
/// values come from the same interpolated observation. The operational frame
/// rotations include observed $dX,dY,x_p,y_p$ corrections.
pub struct EarthOrientationSolution<S: TimeScale> {
    epoch: Instant<S>,
    terrestrial_time: JulianDate<Tt>,
    universal_time: JulianDate<Ut1>,
    observations: EarthOrientation<S>,
    precession_nutation: PrecessionNutation,
    modeled_cip: CelestialIntermediatePole,
    cip: CelestialIntermediatePole,
    modeled_cio_locator: Angle,
    cio_locator: Angle,
    tio_locator: Angle,
    earth_rotation_angle: HourAngle,
    greenwich_mean_sidereal_time: HourAngle,
    greenwich_apparent_sidereal_time: HourAngle,
    equation_of_origins: Angle,
    equation_of_equinoxes: Angle,
    gcrs_to_cirs: Rotation<Gcrs, Cirs>,
    cirs_to_tirs: Rotation<Cirs, Tirs>,
    tirs_to_itrs: Rotation<Tirs, Itrs>,
    gcrs_to_itrs: Rotation<Gcrs, Itrs>,
    modeled_cio_gcrs_to_tirs_matrix: Matrix3,
    equinox_gcrs_to_tirs_matrix: Matrix3,
}

impl<S: TimeScale> EarthOrientationSolution<S> {
    pub(super) fn at(
        epoch: Instant<S>,
        time: &TimeContext<'_, EarthOrientationTable<'_>>,
    ) -> Result<Self, Error> {
        let observations = time.earth_orientation_at(epoch)?;
        let terrestrial_time = JulianDate::<Tt>::from_instant(epoch, time)?;
        let universal_time = JulianDate::<Ut1>::from_instant(epoch, time)?;
        let rotations = EarthAttitudeRotations::at(
            epoch,
            terrestrial_time,
            universal_time,
            observations.polar_motion_x().as_angle(),
            observations.polar_motion_y().as_angle(),
            observations.celestial_pole_offset_x().as_angle(),
            observations.celestial_pole_offset_y().as_angle(),
        )?;
        Ok(Self {
            epoch,
            terrestrial_time,
            universal_time,
            observations,
            precession_nutation: rotations.precession_nutation,
            modeled_cip: rotations.modeled_cip,
            cip: rotations.cip,
            modeled_cio_locator: rotations.modeled_cio_locator,
            cio_locator: rotations.cio_locator,
            tio_locator: rotations.tio_locator,
            earth_rotation_angle: rotations.earth_rotation_angle,
            greenwich_mean_sidereal_time: rotations.greenwich_mean_sidereal_time,
            greenwich_apparent_sidereal_time: rotations.greenwich_apparent_sidereal_time,
            equation_of_origins: rotations.equation_of_origins,
            equation_of_equinoxes: rotations.equation_of_equinoxes,
            gcrs_to_cirs: rotations.gcrs_to_cirs,
            cirs_to_tirs: rotations.cirs_to_tirs,
            tirs_to_itrs: rotations.tirs_to_itrs,
            gcrs_to_itrs: rotations.gcrs_to_itrs,
            modeled_cio_gcrs_to_tirs_matrix: rotations.modeled_cio_gcrs_to_tirs_matrix,
            equinox_gcrs_to_tirs_matrix: rotations.equinox_gcrs_to_tirs_matrix,
        })
    }

    /// Returns the physical epoch represented by every result in this solution.
    pub const fn epoch(self) -> Instant<S> {
        self.epoch
    }

    /// Returns the two-part TT date used for precession and nutation.
    pub const fn terrestrial_time(self) -> JulianDate<Tt> {
        self.terrestrial_time
    }

    /// Returns the two-part UT1 date used for Earth rotation.
    pub const fn universal_time(self) -> JulianDate<Ut1> {
        self.universal_time
    }

    /// Returns the interpolated EOP observation used throughout the solution.
    pub const fn observations(self) -> EarthOrientation<S> {
        self.observations
    }

    /// Returns the IAU 2006/2000A precession-nutation results.
    pub const fn precession_nutation(self) -> PrecessionNutation {
        self.precession_nutation
    }

    /// Returns the uncorrected IAU 2006/2000A CIP coordinates.
    pub const fn modeled_cip(self) -> CelestialIntermediatePole {
        self.modeled_cip
    }

    /// Returns the CIP coordinates after applying observed $dX,dY$.
    pub const fn cip(self) -> CelestialIntermediatePole {
        self.cip
    }

    /// Returns the uncorrected IAU 2006 CIO locator $s$.
    pub const fn modeled_cio_locator(self) -> Angle {
        self.modeled_cio_locator
    }

    /// Returns the CIO locator $s$ recomputed after applying observed $dX,dY$.
    pub const fn cio_locator(self) -> Angle {
        self.cio_locator
    }

    /// Returns the IAU 2000 TIO locator $s'$.
    pub const fn tio_locator(self) -> Angle {
        self.tio_locator
    }

    /// Returns the IAU 2000 Earth Rotation Angle in $[0,2\pi)$.
    pub const fn earth_rotation_angle(self) -> HourAngle {
        self.earth_rotation_angle
    }

    /// Returns IAU 2006 Greenwich Mean Sidereal Time in $[0,2\pi)$.
    pub const fn greenwich_mean_sidereal_time(self) -> HourAngle {
        self.greenwich_mean_sidereal_time
    }

    /// Returns IAU 2006/2000A Greenwich Apparent Sidereal Time in $[0,2\pi)$.
    pub const fn greenwich_apparent_sidereal_time(self) -> HourAngle {
        self.greenwich_apparent_sidereal_time
    }

    /// Returns the equation of the origins, ERA minus GAST.
    pub const fn equation_of_origins(self) -> Angle {
        self.equation_of_origins
    }

    /// Returns the equation of the equinoxes, GAST minus GMST.
    pub const fn equation_of_equinoxes(self) -> Angle {
        self.equation_of_equinoxes
    }

    /// Returns the observed GCRS-to-CIRS rotation at the solution epoch.
    pub const fn gcrs_to_cirs(self) -> FrameRotation<Gcrs, Cirs, S> {
        FrameRotation::new(self.epoch, self.gcrs_to_cirs)
    }
    /// Converts a GCRS direction to observed CIRS intermediate right ascension and declination.
    pub fn intermediate_equatorial(
        self,
        source: EquatorialDirection<Gcrs>,
    ) -> Result<EquatorialDirectionAt<Cirs, S>, Error> {
        let direction = self.gcrs_to_cirs.apply_direction(source.to_direction()?)?;
        Ok(EquatorialDirectionAt::new(
            self.epoch,
            EquatorialDirection::from_direction(direction)?,
        ))
    }

    /// Converts observed CIRS intermediate coordinates back to GCRS.
    pub fn gcrs_from_intermediate_equatorial(
        self,
        source: EquatorialDirectionAt<Cirs, S>,
    ) -> Result<EquatorialDirection<Gcrs>, Error> {
        let source_epoch = source.epoch().tai_nanoseconds_since_1900();
        let solution_epoch = self.epoch.tai_nanoseconds_since_1900();
        if source_epoch != solution_epoch {
            return Err(Error::epoch_mismatch(solution_epoch, source_epoch));
        }
        let direction = self
            .gcrs_to_cirs
            .inverse()
            .apply_direction(source.coordinates().to_direction()?)?;
        EquatorialDirection::from_direction(direction).map_err(Error::from)
    }

    /// Returns the CIRS-to-TIRS rotation at the solution epoch.
    pub const fn cirs_to_tirs(self) -> FrameRotation<Cirs, Tirs, S> {
        FrameRotation::new(self.epoch, self.cirs_to_tirs)
    }

    /// Returns the observed TIRS-to-ITRS rotation at the solution epoch.
    pub const fn tirs_to_itrs(self) -> FrameRotation<Tirs, Itrs, S> {
        FrameRotation::new(self.epoch, self.tirs_to_itrs)
    }

    /// Returns the complete observed GCRS-to-ITRS rotation at the solution epoch.
    pub const fn gcrs_to_itrs(self) -> FrameRotation<Gcrs, Itrs, S> {
        FrameRotation::new(self.epoch, self.gcrs_to_itrs)
    }

    /// Returns the model-only CIO-based GCRS-to-TIRS matrix.
    ///
    /// This intentionally excludes observed $dX,dY$ so it is directly
    /// comparable with [`Self::equinox_gcrs_to_tirs_matrix`].
    pub const fn modeled_cio_gcrs_to_tirs_matrix(self) -> Matrix3 {
        self.modeled_cio_gcrs_to_tirs_matrix
    }

    /// Returns the model-only equinox-based GCRS-to-TIRS matrix.
    pub const fn equinox_gcrs_to_tirs_matrix(self) -> Matrix3 {
        self.equinox_gcrs_to_tirs_matrix
    }

    pub(super) fn gcrs_to_cirs_state_transform(
        self,
    ) -> Result<StateTransform<Gcrs, Cirs, S>, Error> {
        Iau2006And2000A::gcrs_to_cirs(self)?.state_transform(self.epoch)
    }

    pub(super) fn cirs_to_tirs_state_transform(
        self,
    ) -> Result<StateTransform<Cirs, Tirs, S>, Error> {
        let nominal_day_seconds = 86_400.0;
        let excess_day_seconds = self
            .observations
            .excess_length_of_day()
            .as_duration()
            .as_seconds_f64();
        let angular_speed = crate::constants::earth::NOMINAL_ANGULAR_SPEED_RADIANS_PER_SECOND
            * nominal_day_seconds
            / (nominal_day_seconds + excess_day_seconds);
        let angular_velocity = Vector3::from_array([
            AngularSpeed::from_radians_per_second(0.0)?,
            AngularSpeed::from_radians_per_second(0.0)?,
            AngularSpeed::from_radians_per_second(-angular_speed)?,
        ]);
        KinematicRotation {
            rotation: self.cirs_to_tirs,
            angular_velocity,
        }
        .state_transform(self.epoch)
    }

    pub(super) fn tirs_to_itrs_state_transform(
        self,
    ) -> Result<StateTransform<Tirs, Itrs, S>, Error> {
        IersPolarMotion::tirs_to_itrs(self)?.state_transform(self.epoch)
    }

    pub(crate) fn itrs_to_gcrs_state_transform(
        self,
    ) -> Result<StateTransform<Itrs, Gcrs, S>, Error> {
        self.gcrs_to_cirs_state_transform()?
            .then(self.cirs_to_tirs_state_transform()?)?
            .then(self.tirs_to_itrs_state_transform()?)?
            .inverse()
    }
}

impl<S: TimeScale> Copy for EarthOrientationSolution<S> {}

impl<S: TimeScale> Clone for EarthOrientationSolution<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> fmt::Debug for EarthOrientationSolution<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EarthOrientationSolution")
            .field("epoch", &self.epoch)
            .field("terrestrial_time", &self.terrestrial_time)
            .field("universal_time", &self.universal_time)
            .field("observations", &self.observations)
            .field("precession_nutation", &self.precession_nutation)
            .field("modeled_cip", &self.modeled_cip)
            .field("cip", &self.cip)
            .field("modeled_cio_locator", &self.modeled_cio_locator)
            .field("cio_locator", &self.cio_locator)
            .field("tio_locator", &self.tio_locator)
            .field("earth_rotation_angle", &self.earth_rotation_angle)
            .field(
                "greenwich_mean_sidereal_time",
                &self.greenwich_mean_sidereal_time,
            )
            .field(
                "greenwich_apparent_sidereal_time",
                &self.greenwich_apparent_sidereal_time,
            )
            .field("equation_of_origins", &self.equation_of_origins)
            .field("equation_of_equinoxes", &self.equation_of_equinoxes)
            .field("gcrs_to_cirs", &self.gcrs_to_cirs)
            .field("cirs_to_tirs", &self.cirs_to_tirs)
            .field("tirs_to_itrs", &self.tirs_to_itrs)
            .field("gcrs_to_itrs", &self.gcrs_to_itrs)
            .finish_non_exhaustive()
    }
}

struct KinematicRotation<From, To>
where
    From: CoordinateFrame,
    To: CoordinateFrame,
{
    rotation: Rotation<From, To>,
    angular_velocity: Vector3<To, AngularSpeed>,
}

impl<From, To> KinematicRotation<From, To>
where
    From: CoordinateFrame,
    To: CoordinateFrame,
{
    fn state_transform<S: TimeScale>(
        self,
        epoch: Instant<S>,
    ) -> Result<StateTransform<From, To, S>, Error> {
        Ok(StateTransform::new(
            epoch,
            self.rotation,
            self.angular_velocity,
            Vector3::new(
                Length::from_metres(0.0)?,
                Length::from_metres(0.0)?,
                Length::from_metres(0.0)?,
            ),
            Vector3::new(
                Speed::from_metres_per_second(0.0)?,
                Speed::from_metres_per_second(0.0)?,
                Speed::from_metres_per_second(0.0)?,
            ),
        ))
    }
}

struct Iau2006And2000A;

impl Iau2006And2000A {
    fn gcrs_to_cirs<S: TimeScale>(
        solution: EarthOrientationSolution<S>,
    ) -> Result<KinematicRotation<Gcrs, Cirs>, Error> {
        let offset_x = solution.observations.celestial_pole_offset_x().as_angle();
        let offset_y = solution.observations.celestial_pole_offset_y().as_angle();
        let rate_x = RotationRate::required_rate(
            solution.observations.celestial_pole_offset_rate_x(),
            "celestial-pole offset dX",
            solution.epoch,
        )?;
        let rate_y = RotationRate::required_rate(
            solution.observations.celestial_pole_offset_rate_y(),
            "celestial-pole offset dY",
            solution.epoch,
        )?;
        RotationRate::differentiate(solution.gcrs_to_cirs, |offset_seconds| {
            let shifted_tt = solution
                .terrestrial_time
                .checked_add_duration(Duration::from_seconds_f64(offset_seconds)?)?;
            let shifted_x = Angle::from_radians(
                offset_x.as_radians() + rate_x.as_radians_per_second() * offset_seconds,
            )?;
            let shifted_y = Angle::from_radians(
                offset_y.as_radians() + rate_y.as_radians_per_second() * offset_seconds,
            )?;
            Self::gcrs_to_cirs_matrix(shifted_tt, shifted_x, shifted_y)
        })
    }

    fn gcrs_to_cirs_matrix(
        tt: JulianDate<Tt>,
        offset_x: Angle,
        offset_y: Angle,
    ) -> Result<Matrix3, Error> {
        let (tt_first, tt_second) = tt.parts();
        let bias_precession_nutation = sofars::pnp::pnm06a(tt_first, tt_second);
        let (model_x, model_y) = sofars::pnp::bpn2xy(&bias_precession_nutation);
        let observed_x = model_x + offset_x.as_radians();
        let observed_y = model_y + offset_y.as_radians();
        let cio_locator = sofars::pnp::s06(tt_first, tt_second, observed_x, observed_y);
        Matrix3::try_from_rows(sofars::pnp::c2ixys(observed_x, observed_y, cio_locator))
            .map_err(Error::from)
    }

    fn rotation_from_rows<From, To>(rows: [[f64; 3]; 3]) -> Result<Rotation<From, To>, Error> {
        let matrix = Matrix3::try_from_rows(rows)?;
        let tolerance = RotationTolerance::new(
            ROTATION_ORTHOGONALITY_TOLERANCE,
            ROTATION_DETERMINANT_TOLERANCE,
        )?;
        Rotation::try_from_matrix(matrix, tolerance).map_err(Error::from)
    }
}

struct IersPolarMotion;

impl IersPolarMotion {
    fn tirs_to_itrs<S: TimeScale>(
        solution: EarthOrientationSolution<S>,
    ) -> Result<KinematicRotation<Tirs, Itrs>, Error> {
        let polar_motion_x = solution.observations.polar_motion_x().as_angle();
        let polar_motion_y = solution.observations.polar_motion_y().as_angle();
        let rate_x = RotationRate::required_rate(
            solution.observations.polar_motion_rate_x(),
            "polar motion xp",
            solution.epoch,
        )?;
        let rate_y = RotationRate::required_rate(
            solution.observations.polar_motion_rate_y(),
            "polar motion yp",
            solution.epoch,
        )?;
        RotationRate::differentiate(solution.tirs_to_itrs, |offset_seconds| {
            let shifted_tt = solution
                .terrestrial_time
                .checked_add_duration(Duration::from_seconds_f64(offset_seconds)?)?;
            let shifted_x = Angle::from_radians(
                polar_motion_x.as_radians() + rate_x.as_radians_per_second() * offset_seconds,
            )?;
            let shifted_y = Angle::from_radians(
                polar_motion_y.as_radians() + rate_y.as_radians_per_second() * offset_seconds,
            )?;
            Self::tirs_to_itrs_matrix(shifted_tt, shifted_x, shifted_y)
        })
    }

    fn tirs_to_itrs_matrix(
        tt: JulianDate<Tt>,
        polar_motion_x: Angle,
        polar_motion_y: Angle,
    ) -> Result<Matrix3, Error> {
        let (tt_first, tt_second) = tt.parts();
        let tio_locator = sofars::pnp::sp00(tt_first, tt_second);
        Matrix3::try_from_rows(sofars::pnp::pom00(
            polar_motion_x.as_radians(),
            polar_motion_y.as_radians(),
            tio_locator,
        ))
        .map_err(Error::from)
    }
}

struct RotationRate;

impl RotationRate {
    fn differentiate<From, To, Evaluate>(
        current_rotation: Rotation<From, To>,
        evaluate: Evaluate,
    ) -> Result<KinematicRotation<From, To>, Error>
    where
        From: CoordinateFrame,
        To: CoordinateFrame,
        Evaluate: Fn(f64) -> Result<Matrix3, Error>,
    {
        let current = current_rotation.matrix();
        let coarse = Self::central_derivative(
            evaluate(-ROTATION_RATE_DIFFERENCE_STEP_SECONDS)?,
            evaluate(ROTATION_RATE_DIFFERENCE_STEP_SECONDS)?,
            ROTATION_RATE_DIFFERENCE_STEP_SECONDS,
        )?;
        let fine_step = ROTATION_RATE_DIFFERENCE_STEP_SECONDS / 2.0;
        let fine =
            Self::central_derivative(evaluate(-fine_step)?, evaluate(fine_step)?, fine_step)?;
        let coarse_omega = Self::axial(current, coarse)?;
        let fine_omega = Self::axial(current, fine)?;
        let mut extrapolated = [0.0; 3];
        let mut residual = 0.0_f64;
        for index in 0..3 {
            let correction = (fine_omega[index] - coarse_omega[index]) / 3.0;
            extrapolated[index] = fine_omega[index] + correction;
            residual = residual.max(correction.abs());
        }

        let extrapolated_derivative = Self::richardson_derivative(coarse, fine)?;
        residual = residual.max(Self::skew_residual(current, extrapolated_derivative)?);
        if residual > ROTATION_RATE_CONVERGENCE_TOLERANCE_RADIANS_PER_SECOND {
            return Err(Error::RotationRateDidNotConverge {
                residual,
                tolerance: ROTATION_RATE_CONVERGENCE_TOLERANCE_RADIANS_PER_SECOND,
            });
        }

        let angular_velocity = Vector3::from_array([
            AngularSpeed::from_radians_per_second(extrapolated[0])?,
            AngularSpeed::from_radians_per_second(extrapolated[1])?,
            AngularSpeed::from_radians_per_second(extrapolated[2])?,
        ]);
        Ok(KinematicRotation {
            rotation: current_rotation,
            angular_velocity,
        })
    }

    fn required_rate<S: TimeScale>(
        rate: Option<AngularSpeed>,
        field: &'static str,
        epoch: Instant<S>,
    ) -> Result<AngularSpeed, Error> {
        rate.ok_or(Error::EarthOrientationRateUnavailable {
            field,
            epoch_tai_nanoseconds: epoch.tai_nanoseconds_since_1900(),
        })
    }

    fn central_derivative(
        minus: Matrix3,
        plus: Matrix3,
        half_span_seconds: f64,
    ) -> Result<Matrix3, Error> {
        let minus = minus.rows();
        let plus = plus.rows();
        let denominator = 2.0 * half_span_seconds;
        let mut rows = [[0.0; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                rows[row][column] = (plus[row][column] - minus[row][column]) / denominator;
            }
        }
        Matrix3::try_from_rows(rows).map_err(Error::from)
    }

    fn richardson_derivative(coarse: Matrix3, fine: Matrix3) -> Result<Matrix3, Error> {
        let coarse = coarse.rows();
        let fine = fine.rows();
        let mut rows = [[0.0; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                rows[row][column] =
                    fine[row][column] + (fine[row][column] - coarse[row][column]) / 3.0;
            }
        }
        Matrix3::try_from_rows(rows).map_err(Error::from)
    }

    fn axial(rotation: Matrix3, derivative: Matrix3) -> Result<[f64; 3], Error> {
        let product = derivative.checked_mul(rotation.transpose())?.rows();
        Ok([
            (product[2][1] - product[1][2]) / 2.0,
            (product[0][2] - product[2][0]) / 2.0,
            (product[1][0] - product[0][1]) / 2.0,
        ])
    }

    fn skew_residual(rotation: Matrix3, derivative: Matrix3) -> Result<f64, Error> {
        let product = derivative.checked_mul(rotation.transpose())?.rows();
        let mut residual = 0.0_f64;
        for (row, values) in product.iter().enumerate() {
            residual = residual.max(values[row].abs());
            for (column, value) in values.iter().enumerate().skip(row + 1) {
                residual = residual.max(((*value + product[column][row]) / 2.0).abs());
            }
        }
        Ok(residual)
    }
}
