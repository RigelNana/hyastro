//! Bracketed astronomical event searches and fixed-offset event calendars.

mod configuration;
mod error;
mod extremum;
mod horizon;
mod lunar_phase;
mod period;
mod relative;
mod search;
mod solar_term;
mod twilight;
pub use configuration::{
    ConfigurationCoordinate, ConfigurationEvent, ConfigurationKind, ConfigurationQuery,
    ElongationSide, GreatestElongationEvent, SolarConjunctionKind, StationEvent, StationEvidence,
    StationKind, StationQuery,
};

pub use error::Error;
pub use extremum::{
    AngularSeparationExtremumEvent, AngularSeparationExtremumQuery, CoordinateCrossingEvent,
    CoordinateCrossingKind, CoordinateCrossingQuery, CoordinateExtremumEvent,
    CoordinateExtremumQuery, DistanceExtremumEvent, DistanceExtremumQuery, EventCoordinate,
    EventCoordinateValue, ExtremumKind,
};
pub use horizon::{
    HorizonCriterion, HorizonDiskPoint, HorizonEvent, HorizonEventKind, HorizonEventSearch,
    HorizonReference, HorizonSearchOptions, HorizonVisibility,
};
pub use lunar_phase::{
    MoonPhase, MoonPhaseAngleEvent, MoonPhaseEvent, MoonPhaseYear, MoonPhaseYearEntry,
};
pub use period::{
    AnomalisticMonth, AnomalisticYear, CycleBoundary, CycleEvent, CycleEvidence, CycleKind,
    CycleModel, CycleResidual, CycleStatistics, DraconicMonth, DraconicYear, EquinoxKind,
    EquinoxYear, LunarNode, MeasuredCycle, ModelValidity, ModeledCycle, SiderealMonth,
    SiderealYear, SynodicMonth, TropicalMonth, TropicalYear,
};
pub use relative::{AstrometricMode, EventBodyPosition, ObservationOrigin, RelativeBodyQuery};
pub use search::{
    AngularEventSearchOptions, EventEvidence, Events, ExtremumEvidence, ExtremumSearchOptions,
};
pub use solar_term::{SolarTerm, SolarTermEvent, SolarTermYear, SolarTermYearEntry};
pub use twilight::{
    TwilightEvent, TwilightEventKind, TwilightLevel, TwilightSearch, TwilightState,
};
