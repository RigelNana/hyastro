use crate::{
    frame::{CoordinateFrame, Frames, Gcrs, Itrs, State, StateTransform},
    math::{Direction, Speed, Vector3},
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

    /// Transforms the fixed ITRS site into its GCRS position and velocity.
    ///
    /// The resulting velocity includes Earth rotation and the EOP-dependent
    /// ITRS-to-GCRS transformation at the requested physical epoch.
    pub fn gcrs_state<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        frames: &Frames<'_, '_, EarthOrientationTable<'_>>,
    ) -> Result<State<Gcrs, S>, Error> {
        frames
            .transform(self.itrs_state(epoch))
            .map_err(Error::from)
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
        let transform = frames.at::<Itrs, Gcrs, S>(epoch)?;
        self.east_north_up.transformed(transform)
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
        let transform = frames.at::<Itrs, Gcrs, S>(epoch)?;
        self.north_east_down.transformed(transform)
    }
}
