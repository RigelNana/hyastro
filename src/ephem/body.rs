use core::fmt;

/// A solar-system body or system barycentre with a stable astronomical identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CelestialBody {
    /// Solar-system barycentre, NAIF ID 0.
    SolarSystemBarycenter,
    /// Mercury system barycentre, NAIF ID 1.
    MercuryBarycenter,
    /// Venus system barycentre, NAIF ID 2.
    VenusBarycenter,
    /// Earth-Moon barycentre, NAIF ID 3.
    EarthMoonBarycenter,
    /// Mars system barycentre, NAIF ID 4.
    MarsBarycenter,
    /// Jupiter system barycentre, NAIF ID 5.
    JupiterBarycenter,
    /// Saturn system barycentre, NAIF ID 6.
    SaturnBarycenter,
    /// Uranus system barycentre, NAIF ID 7.
    UranusBarycenter,
    /// Neptune system barycentre, NAIF ID 8.
    NeptuneBarycenter,
    /// Pluto system barycentre, NAIF ID 9.
    PlutoBarycenter,
    /// Sun, NAIF ID 10.
    Sun,
    /// Mercury, NAIF ID 199.
    Mercury,
    /// Venus, NAIF ID 299.
    Venus,
    /// Moon, NAIF ID 301.
    Moon,
    /// Earth, NAIF ID 399.
    Earth,
    /// Mars, NAIF ID 499.
    Mars,
    /// Jupiter, NAIF ID 599.
    Jupiter,
    /// Saturn, NAIF ID 699.
    Saturn,
    /// Uranus, NAIF ID 799.
    Uranus,
    /// Neptune, NAIF ID 899.
    Neptune,
    /// Pluto, NAIF ID 999.
    Pluto,
}

impl CelestialBody {
    /// Returns the conventional English name of the body.
    pub const fn name(self) -> &'static str {
        match self {
            Self::SolarSystemBarycenter => "solar-system barycentre",
            Self::MercuryBarycenter => "Mercury barycentre",
            Self::VenusBarycenter => "Venus barycentre",
            Self::EarthMoonBarycenter => "Earth-Moon barycentre",
            Self::MarsBarycenter => "Mars barycentre",
            Self::JupiterBarycenter => "Jupiter barycentre",
            Self::SaturnBarycenter => "Saturn barycentre",
            Self::UranusBarycenter => "Uranus barycentre",
            Self::NeptuneBarycenter => "Neptune barycentre",
            Self::PlutoBarycenter => "Pluto barycentre",
            Self::Sun => "Sun",
            Self::Mercury => "Mercury",
            Self::Venus => "Venus",
            Self::Moon => "Moon",
            Self::Earth => "Earth",
            Self::Mars => "Mars",
            Self::Jupiter => "Jupiter",
            Self::Saturn => "Saturn",
            Self::Uranus => "Uranus",
            Self::Neptune => "Neptune",
            Self::Pluto => "Pluto",
        }
    }

    #[cfg(feature = "anise")]
    pub(crate) const fn naif_id(self) -> i32 {
        match self {
            Self::SolarSystemBarycenter => 0,
            Self::MercuryBarycenter => 1,
            Self::VenusBarycenter => 2,
            Self::EarthMoonBarycenter => 3,
            Self::MarsBarycenter => 4,
            Self::JupiterBarycenter => 5,
            Self::SaturnBarycenter => 6,
            Self::UranusBarycenter => 7,
            Self::NeptuneBarycenter => 8,
            Self::PlutoBarycenter => 9,
            Self::Sun => 10,
            Self::Mercury => 199,
            Self::Venus => 299,
            Self::Moon => 301,
            Self::Earth => 399,
            Self::Mars => 499,
            Self::Jupiter => 599,
            Self::Saturn => 699,
            Self::Uranus => 799,
            Self::Neptune => 899,
            Self::Pluto => 999,
        }
    }
}

impl fmt::Display for CelestialBody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
