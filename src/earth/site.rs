use crate::{
    frame::{
        CoordinateFrame, EarthOrientationSolution, Frames, Gcrs, HorizontalDirection, Itrs, State,
        StateTransform,
    },
    math::{Direction, Point3, Speed, Vector3},
    time::{EarthOrientationTable, Instant, TimeScale},
};

use super::{Earth, Error, GeodeticPosition, ReferenceEllipsoid};

/// Right-handed local east-north-up directions expressed in a typed frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EastNorthUp<F: CoordinateFrame> {
    east: Direction<F>,
    north: Direction<F>,
    up: Direction<F>,
}

impl<F: CoordinateFrame> EastNorthUp<F> {
    /// Returns the local east direction.
    pub const fn east(self) -> Direction<F> {
        self.east
    }

    /// Returns the local north direction.
    pub const fn north(self) -> Direction<F> {
        self.north
    }

    /// Returns the outward ellipsoid-normal direction.
    pub const fn up(self) -> Direction<F> {
        self.up
    }

    /// Rotates the three local directions into another frame at the transform epoch.
    pub fn transformed<To, S>(
        self,
        transform: StateTransform<F, To, S>,
    ) -> Result<EastNorthUp<To>, Error>
    where
        To: CoordinateFrame,
        S: TimeScale,
    {
        Ok(EastNorthUp {
            east: transform.apply_direction(self.east)?,
            north: transform.apply_direction(self.north)?,
            up: transform.apply_direction(self.up)?,
        })
    }
}

/// Right-handed local north-east-down directions expressed in a typed frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NorthEastDown<F: CoordinateFrame> {
    north: Direction<F>,
    east: Direction<F>,
    down: Direction<F>,
}

impl<F: CoordinateFrame> NorthEastDown<F> {
    /// Returns the local north direction.
    pub const fn north(self) -> Direction<F> {
        self.north
    }

    /// Returns the local east direction.
    pub const fn east(self) -> Direction<F> {
        self.east
    }

    /// Returns the inward ellipsoid-normal direction.
    pub const fn down(self) -> Direction<F> {
        self.down
    }

    /// Rotates the three local directions into another frame at the transform epoch.
    pub fn transformed<To, S>(
        self,
        transform: StateTransform<F, To, S>,
    ) -> Result<NorthEastDown<To>, Error>
    where
        To: CoordinateFrame,
        S: TimeScale,
    {
        Ok(NorthEastDown {
            north: transform.apply_direction(self.north)?,
            east: transform.apply_direction(self.east)?,
            down: transform.apply_direction(self.down)?,
        })
    }
}

/// A fixed site's runtime topocentric frame at one physical epoch.
///
/// The value carries its site geometry, GCRS observer state, and local ENU
/// axes together. It is deliberately not a static [`CoordinateFrame`] marker:
/// two sites at the same epoch have different origins and axis orientations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TopocentricFrame<S: TimeScale> {
    earth: Earth,
    geodetic_position: GeodeticPosition,
    itrs_position: Point3<Itrs>,
    observer_state: State<Gcrs, S>,
    east_north_up: EastNorthUp<Gcrs>,
}

impl<S: TimeScale> TopocentricFrame<S> {
    /// Returns the physical epoch shared by the observer state and local axes.
    pub const fn epoch(self) -> Instant<S> {
        self.observer_state.epoch()
    }

    /// Returns the Earth model used to construct the site.
    pub const fn earth(self) -> Earth {
        self.earth
    }

    /// Returns the site's geodetic position.
    pub const fn geodetic_position(self) -> GeodeticPosition {
        self.geodetic_position
    }

    /// Returns the site's fixed ITRS Cartesian position.
    pub const fn itrs_position(self) -> Point3<Itrs> {
        self.itrs_position
    }

    /// Returns the site's geocentric state expressed in GCRS.
    pub const fn observer_state(self) -> State<Gcrs, S> {
        self.observer_state
    }

    /// Returns the site's local ENU directions expressed in GCRS.
    pub const fn east_north_up(self) -> EastNorthUp<Gcrs> {
        self.east_north_up
    }

    /// Returns the site's local NED directions expressed in GCRS.
    pub fn north_east_down(self) -> Result<NorthEastDown<Gcrs>, Error> {
        let down = Direction::try_from_components(
            self.east_north_up
                .up()
                .components()
                .map(|component| -component),
        )?;
        Ok(NorthEastDown {
            north: self.east_north_up.north(),
            east: self.east_north_up.east(),
            down,
        })
    }

    /// Projects a GCRS unit direction onto the local horizontal axes.
    pub fn horizontal_direction(
        self,
        direction: Direction<Gcrs>,
    ) -> Result<HorizontalDirection, Error> {
        HorizontalDirection::from_enu_components([
            direction.dot(self.east_north_up.east()),
            direction.dot(self.east_north_up.north()),
            direction.dot(self.east_north_up.up()),
        ])
        .map_err(Error::from)
    }

    /// Converts a local horizontal direction back to a GCRS unit direction.
    pub fn gcrs_direction(self, horizontal: HorizontalDirection) -> Result<Direction<Gcrs>, Error> {
        let [east, north, up] = horizontal.enu_components();
        let east_axis = self.east_north_up.east().components();
        let north_axis = self.east_north_up.north().components();
        let up_axis = self.east_north_up.up().components();
        Direction::try_from_components([
            east * east_axis[0] + north * north_axis[0] + up * up_axis[0],
            east * east_axis[1] + north * north_axis[1] + up * up_axis[1],
            east * east_axis[2] + north * north_axis[2] + up * up_axis[2],
        ])
        .map_err(Error::from)
    }
}

/// A site fixed at one geodetic position in ITRS.
///
/// "Fixed" means the ITRS coordinates have zero velocity in this model. The
/// type does not claim a concrete ITRF realization, tectonic motion model,
/// loading correction, geoid height, or atmospheric state.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedSite {
    identifier: String,
    earth: Earth,
    geodetic_position: GeodeticPosition,
    itrs_position: crate::math::Point3<Itrs>,
    east_north_up: EastNorthUp<Itrs>,
    north_east_down: NorthEastDown<Itrs>,
}

impl Earth {
    /// Constructs an identified site fixed at one geodetic position in ITRS.
    pub fn fixed_site(
        self,
        identifier: impl Into<String>,
        position: GeodeticPosition,
    ) -> Result<FixedSite, Error> {
        let identifier = identifier.into();
        if identifier.trim().is_empty() {
            return Err(Error::EmptySiteIdentifier);
        }

        let itrs_position = self.itrs_position(position)?;
        let (longitude_sine, longitude_cosine) = position.longitude().as_radians().sin_cos();
        let (latitude_sine, latitude_cosine) = position.latitude().as_radians().sin_cos();

        let east = Direction::try_from_components([-longitude_sine, longitude_cosine, 0.0])?;
        let north = Direction::try_from_components([
            -latitude_sine * longitude_cosine,
            -latitude_sine * longitude_sine,
            latitude_cosine,
        ])?;
        let up = Direction::try_from_components([
            latitude_cosine * longitude_cosine,
            latitude_cosine * longitude_sine,
            latitude_sine,
        ])?;
        let down_components = up.components().map(|component| -component);
        let down = Direction::try_from_components(down_components)?;

        Ok(FixedSite {
            identifier,
            earth: self,
            geodetic_position: position,
            itrs_position,
            east_north_up: EastNorthUp { east, north, up },
            north_east_down: NorthEastDown { north, east, down },
        })
    }
}

impl FixedSite {
    /// Returns the site identifier.
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// Returns the Earth model used for this site.
    pub const fn earth(&self) -> Earth {
        self.earth
    }

    /// Returns the site's reference ellipsoid.
    pub const fn reference_ellipsoid(&self) -> ReferenceEllipsoid {
        self.earth.reference_ellipsoid()
    }

    /// Returns the site's geodetic position.
    pub const fn geodetic_position(&self) -> GeodeticPosition {
        self.geodetic_position
    }

    /// Returns the site's fixed ITRS Cartesian position.
    pub const fn itrs_position(&self) -> crate::math::Point3<Itrs> {
        self.itrs_position
    }

    /// Returns the site's local east-north-up directions in ITRS.
    pub const fn east_north_up(&self) -> EastNorthUp<Itrs> {
        self.east_north_up
    }

    /// Returns the site's local north-east-down directions in ITRS.
    pub const fn north_east_down(&self) -> NorthEastDown<Itrs> {
        self.north_east_down
    }

    /// Returns the site's zero-ITRS-velocity state at an epoch.
    pub fn itrs_state<S: TimeScale>(&self, epoch: Instant<S>) -> State<Itrs, S> {
        let zero = Speed::from_finite(0.0);
        State::new(self.itrs_position, Vector3::new(zero, zero, zero), epoch)
    }

    /// Evaluates the site's complete runtime topocentric frame at an epoch.
    ///
    /// One coherent Earth-orientation snapshot drives both the GCRS observer
    /// state and local axes, avoiding mismatched or repeated EOP evaluation.
    pub fn topocentric_frame_at<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        frames: &Frames<'_, '_, EarthOrientationTable<'_>>,
    ) -> Result<TopocentricFrame<S>, Error> {
        self.topocentric_frame_from_orientation(frames.earth_orientation_at(epoch)?)
    }

    pub(crate) fn topocentric_frame_from_orientation<S: TimeScale>(
        &self,
        orientation: EarthOrientationSolution<S>,
    ) -> Result<TopocentricFrame<S>, Error> {
        let epoch = orientation.epoch();
        let observer_state = orientation
            .itrs_to_gcrs_state_transform()?
            .apply_state(self.itrs_state(epoch))?;
        let itrs_to_gcrs = orientation.gcrs_to_itrs().inverse();
        let east_north_up = EastNorthUp {
            east: itrs_to_gcrs.apply_direction(self.east_north_up.east())?,
            north: itrs_to_gcrs.apply_direction(self.east_north_up.north())?,
            up: itrs_to_gcrs.apply_direction(self.east_north_up.up())?,
        };
        Ok(TopocentricFrame {
            earth: self.earth,
            geodetic_position: self.geodetic_position,
            itrs_position: self.itrs_position,
            observer_state,
            east_north_up,
        })
    }

    /// Transforms the fixed ITRS site into its GCRS position and velocity.
    ///
    /// The resulting velocity includes Earth rotation and the EOP-dependent
    /// ITRS-to-GCRS transformation at the requested physical epoch.
    pub fn gcrs_state<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        frames: &Frames<'_, '_, EarthOrientationTable<'_>>,
    ) -> Result<State<Gcrs, S>, Error> {
        Ok(self.topocentric_frame_at(epoch, frames)?.observer_state())
    }

    /// Returns the actual local east-north-up directions expressed in GCRS.
    ///
    /// The transformation includes precession-nutation, Earth rotation, and
    /// observed polar motion for the requested epoch.
    pub fn gcrs_east_north_up<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        frames: &Frames<'_, '_, EarthOrientationTable<'_>>,
    ) -> Result<EastNorthUp<Gcrs>, Error> {
        Ok(self.topocentric_frame_at(epoch, frames)?.east_north_up())
    }

    /// Returns the actual local north-east-down directions expressed in GCRS.
    ///
    /// The transformation includes precession-nutation, Earth rotation, and
    /// observed polar motion for the requested epoch.
    pub fn gcrs_north_east_down<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        frames: &Frames<'_, '_, EarthOrientationTable<'_>>,
    ) -> Result<NorthEastDown<Gcrs>, Error> {
        self.topocentric_frame_at(epoch, frames)?.north_east_down()
    }
}
