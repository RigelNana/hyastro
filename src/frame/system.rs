use core::fmt::Debug;

mod sealed {
    pub trait Sealed {}
}

/// Stable identity of a supported astronomical reference system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReferenceSystemId {
    /// International Celestial Reference System.
    Icrs,
    /// Barycentric Celestial Reference System.
    Bcrs,
    /// Geocentric Celestial Reference System.
    Gcrs,
    /// Celestial Intermediate Reference System.
    Cirs,
    /// Terrestrial Intermediate Reference System.
    Tirs,
    /// International Terrestrial Reference System.
    Itrs,
}

/// Stable identity of an origin carried by a coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OriginId {
    /// Solar-system barycentre.
    SolarSystemBarycenter,
    /// Centre of mass of the Earth system.
    EarthCenter,
    /// Not Applicable, i.e. ICRS
    NotApplicable,
}

/// Orientation family used by a coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Axes {
    /// Axes aligned with the International Celestial Reference System.
    IcrsAligned,
    /// Kinematically non-rotating geocentric celestial axes.
    Gcrs,
    /// Celestial intermediate axes defined by the CIP and CIO.
    CelestialIntermediate,
    /// Terrestrial intermediate axes after applying Earth rotation.
    TerrestrialIntermediate,
    /// Earth-fixed terrestrial axes.
    Terrestrial,
}

/// Handedness of a coordinate frame's Cartesian axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Handedness {
    /// Right-handed Cartesian axes.
    Right,
    /// Left-handed Cartesian axes.
    Left,
}

/// Reference epoch attached to a frame definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReferenceEpoch {
    /// No reference epoch is part of the definition.
    NotApplicable,
    /// Julian epoch J2000.0.
    J2000,
    /// Besselian epoch B1950.0.
    B1950,
    /// The definition varies with the requested observation date.
    OfDate,
}

/// Equinox attached to a frame definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Equinox {
    /// The frame is not equinox based.
    NotApplicable,
    /// J2000.0 equinox.
    J2000,
    /// B1950.0 equinox.
    B1950,
    /// Equinox of the requested observation date.
    OfDate,
}

/// Queryable metadata that completely identifies a static coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameDefinition {
    name: &'static str,
    system: ReferenceSystemId,
    origin: OriginId,
    axes: Axes,
    handedness: Handedness,
    reference_epoch: ReferenceEpoch,
    equinox: Equinox,
}

impl FrameDefinition {
    pub(crate) const fn new(
        name: &'static str,
        system: ReferenceSystemId,
        origin: OriginId,
        axes: Axes,
        handedness: Handedness,
        reference_epoch: ReferenceEpoch,
        equinox: Equinox,
    ) -> Self {
        Self {
            name,
            system,
            origin,
            axes,
            handedness,
            reference_epoch,
            equinox,
        }
    }

    /// Returns the conventional short name of the coordinate frame.
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns the ideal reference system represented by the frame.
    pub const fn system(self) -> ReferenceSystemId {
        self.system
    }

    /// Returns the frame origin.
    pub const fn origin(self) -> OriginId {
        self.origin
    }

    /// Returns the frame's axis orientation family.
    pub const fn axes(self) -> Axes {
        self.axes
    }

    /// Returns the handedness of the Cartesian axes.
    pub const fn handedness(self) -> Handedness {
        self.handedness
    }

    /// Returns the reference epoch attached to the frame definition.
    pub const fn reference_epoch(self) -> ReferenceEpoch {
        self.reference_epoch
    }

    /// Returns the equinox attached to the frame definition.
    pub const fn equinox(self) -> Equinox {
        self.equinox
    }
}

/// A sealed marker for a supported coordinate origin.
pub trait Origin: sealed::Sealed + Copy + Clone + Debug + Eq {
    /// Stable origin identity.
    const ID: OriginId;

    /// Conventional origin name.
    const NAME: &'static str;
}

/// A sealed marker carrying complete static coordinate-frame semantics.
pub trait CoordinateFrame: sealed::Sealed + Copy + Clone + Debug + Eq {
    /// Ideal reference system underlying this frame.
    type System: ReferenceSystem;

    /// Origin fixed by this frame.
    type Origin: Origin;

    /// Complete static metadata for this frame.
    const DEFINITION: FrameDefinition;

    /// Returns the frame's complete static metadata.
    fn definition() -> FrameDefinition {
        Self::DEFINITION
    }
}

/// A coordinate frame that is itself an ideal reference system.
pub trait ReferenceSystem: CoordinateFrame<System = Self> {}

/// Solar-system barycentre origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SolarSystemBarycenter;

impl sealed::Sealed for SolarSystemBarycenter {}

impl Origin for SolarSystemBarycenter {
    const ID: OriginId = OriginId::SolarSystemBarycenter;
    const NAME: &'static str = "solar-system barycentre";
}

/// Centre-of-mass origin of the Earth system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EarthCenter;

impl sealed::Sealed for EarthCenter {}

impl Origin for EarthCenter {
    const ID: OriginId = OriginId::EarthCenter;
    const NAME: &'static str = "Earth centre";
}

/// International Celestial Reference System coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Icrs;

impl sealed::Sealed for Icrs {}

impl CoordinateFrame for Icrs {
    type System = Self;
    type Origin = SolarSystemBarycenter;

    const DEFINITION: FrameDefinition = FrameDefinition::new(
        "ICRS",
        ReferenceSystemId::Icrs,
        OriginId::NotApplicable,
        Axes::IcrsAligned,
        Handedness::Right,
        ReferenceEpoch::NotApplicable,
        Equinox::NotApplicable,
    );
}

impl ReferenceSystem for Icrs {}

/// Barycentric Celestial Reference System coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bcrs;

impl sealed::Sealed for Bcrs {}

impl CoordinateFrame for Bcrs {
    type System = Self;
    type Origin = SolarSystemBarycenter;

    const DEFINITION: FrameDefinition = FrameDefinition::new(
        "BCRS",
        ReferenceSystemId::Bcrs,
        OriginId::SolarSystemBarycenter,
        Axes::IcrsAligned,
        Handedness::Right,
        ReferenceEpoch::NotApplicable,
        Equinox::NotApplicable,
    );
}

impl ReferenceSystem for Bcrs {}

/// Geocentric Celestial Reference System coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gcrs;

impl sealed::Sealed for Gcrs {}

impl CoordinateFrame for Gcrs {
    type System = Self;
    type Origin = EarthCenter;

    const DEFINITION: FrameDefinition = FrameDefinition::new(
        "GCRS",
        ReferenceSystemId::Gcrs,
        OriginId::EarthCenter,
        Axes::Gcrs,
        Handedness::Right,
        ReferenceEpoch::NotApplicable,
        Equinox::NotApplicable,
    );
}

impl ReferenceSystem for Gcrs {}

/// Celestial Intermediate Reference System coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cirs;

impl sealed::Sealed for Cirs {}

impl CoordinateFrame for Cirs {
    type System = Self;
    type Origin = EarthCenter;

    const DEFINITION: FrameDefinition = FrameDefinition::new(
        "CIRS",
        ReferenceSystemId::Cirs,
        OriginId::EarthCenter,
        Axes::CelestialIntermediate,
        Handedness::Right,
        ReferenceEpoch::OfDate,
        Equinox::NotApplicable,
    );
}

impl ReferenceSystem for Cirs {}

/// Terrestrial Intermediate Reference System coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tirs;

impl sealed::Sealed for Tirs {}

impl CoordinateFrame for Tirs {
    type System = Self;
    type Origin = EarthCenter;

    const DEFINITION: FrameDefinition = FrameDefinition::new(
        "TIRS",
        ReferenceSystemId::Tirs,
        OriginId::EarthCenter,
        Axes::TerrestrialIntermediate,
        Handedness::Right,
        ReferenceEpoch::OfDate,
        Equinox::NotApplicable,
    );
}

impl ReferenceSystem for Tirs {}

/// International Terrestrial Reference System coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Itrs;

impl sealed::Sealed for Itrs {}

impl CoordinateFrame for Itrs {
    type System = Self;
    type Origin = EarthCenter;

    const DEFINITION: FrameDefinition = FrameDefinition::new(
        "ITRS",
        ReferenceSystemId::Itrs,
        OriginId::EarthCenter,
        Axes::Terrestrial,
        Handedness::Right,
        ReferenceEpoch::NotApplicable,
        Equinox::NotApplicable,
    );
}

impl ReferenceSystem for Itrs {}
