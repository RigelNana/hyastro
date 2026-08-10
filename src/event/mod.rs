//! Bracketed astronomical event searches and fixed-offset event calendars.

mod configuration;
mod error;
mod extremum;
mod global_solar_eclipse;
mod horizon;
mod lunar_eclipse;
mod lunar_phase;
mod period;
mod relative;
mod search;
mod solar_eclipse;
mod solar_eclipse_path;
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
pub use global_solar_eclipse::{
    BesselianAnglePolynomial, BesselianDerivativeMethod, BesselianElementDerivatives,
    BesselianElementRate, BesselianElements, BesselianElementsOptions, BesselianElementsPolynomial,
    BesselianFundamentalPlane, BesselianLimbModel, BesselianLunarRadiusRatio,
    BesselianPlaneCoordinate, BesselianPolynomialOptions, BesselianPolynomialResiduals,
    BesselianScalarPolynomial, BesselianShadowRadius, CentralSolarEclipseCharacter,
    CentralSolarEclipsePathLimit, CentralSolarEclipsePathLimitKind, GlobalSolarEclipse,
    GlobalSolarEclipseCentralPath, GlobalSolarEclipseKind, GlobalSolarEclipseMaximum,
    HybridSolarEclipseTransition, SolarEclipseGamma, SolarShadowRadius,
};
pub use horizon::{
    HorizonCriterion, HorizonDiskPoint, HorizonEvent, HorizonEventKind, HorizonEventSearch,
    HorizonReference, HorizonSearchOptions, HorizonVisibility,
};
pub use lunar_eclipse::{
    GlobalLunarEclipse, LocalLunarEclipseSample, LocalLunarEclipseVisibility, LunarEclipseContact,
    LunarEclipseContactKind, LunarEclipseKind, LunarEclipseMagnitude, LunarEclipseMaximum,
    LunarEclipseModel, LunarEclipsePhaseInterval, LunarEclipseSearchOptions,
    LunarEclipseSkyBackground, LunarEclipseVisibilityOptions, LunarEclipseVisibilityStage,
    LunarShadowConvention, LunarShadowGeometry, VisibleLunarEclipsePhase,
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
pub use solar_eclipse::{
    LocalSolarEclipse, LocalSolarEclipseKind, LocalSolarEclipseMaximum,
    LocalSolarEclipseObservation, SolarEclipseContact, SolarEclipseContactKind,
    SolarEclipseEarthAttitudeProvenance, SolarEclipseMagnitude, SolarEclipseModel,
    SolarEclipseSearchOptions, SolarObscuration,
};
pub use solar_eclipse_path::{
    GlobalSolarEclipsePath, GlobalSolarEclipsePathOptions, GlobalSolarEclipsePathPoint,
    SolarEclipseCentralPhase,
};
pub use solar_term::{SolarTerm, SolarTermEvent, SolarTermYear, SolarTermYearEntry};
pub use twilight::{
    TwilightEvent, TwilightEventKind, TwilightLevel, TwilightSearch, TwilightState,
};
