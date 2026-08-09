use std::vec::Vec;

use crate::{
    earth::FixedSite,
    ephem::CelestialBody,
    math::Altitude,
    time::{EarthOrientationTable, Instant, TimeInterval, TimeScale},
};

use super::{
    Error, EventEvidence, Events, HorizonCriterion, HorizonEvent, HorizonEventKind,
    HorizonEventSearch, HorizonSearchOptions, HorizonVisibility,
};

/// Standard or caller-defined solar-centre altitude defining twilight.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum TwilightLevel {
    /// Civil twilight at a solar-centre altitude of −6°.
    Civil,
    /// Nautical twilight at a solar-centre altitude of −12°.
    Nautical,
    /// Astronomical twilight at a solar-centre altitude of −18°.
    Astronomical,
    /// A caller-defined solar-centre altitude.
    Custom(Altitude),
}

impl TwilightLevel {
    /// Returns the defining topocentric vacuum solar-centre altitude.
    pub const fn solar_altitude(self) -> Altitude {
        match self {
            Self::Civil => Altitude::from_finite(-6.0_f64.to_radians()),
            Self::Nautical => Altitude::from_finite(-12.0_f64.to_radians()),
            Self::Astronomical => Altitude::from_finite(-18.0_f64.to_radians()),
            Self::Custom(altitude) => altitude,
        }
    }
}

/// Direction of one twilight-level crossing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TwilightEventKind {
    /// The Sun crosses upward through the twilight altitude.
    Dawn,
    /// The Sun crosses downward through the twilight altitude.
    Dusk,
}

/// Illumination-state classification over the requested closed interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TwilightState {
    /// Both dawn and dusk occur in the interval.
    DawnAndDusk,
    /// The Sun remains above the selected twilight altitude.
    ContinuousLight,
    /// The Sun remains below the selected twilight altitude.
    ContinuousDark,
    /// The interval cuts off one member of a dawn/dusk pair.
    TruncatedByInterval,
    /// More than one dawn/dusk cycle occurs in the interval.
    MultipleCycles,
    /// The solar path reaches the criterion without a resolved sign-changing crossing.
    GrazesLevel,
}

/// One refined dawn or dusk event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwilightEvent<S: TimeScale> {
    kind: TwilightEventKind,
    horizon_event: HorizonEvent<S>,
}

impl<S: TimeScale> TwilightEvent<S> {
    /// Returns whether this crossing is dawn or dusk.
    pub const fn kind(self) -> TwilightEventKind {
        self.kind
    }

    /// Returns the refined physical event instant.
    pub const fn instant(self) -> Instant<S> {
        self.horizon_event.instant()
    }

    /// Returns the underlying solar altitude-crossing result.
    pub const fn horizon_event(self) -> HorizonEvent<S> {
        self.horizon_event
    }

    /// Returns the numerical evidence retained for the crossing.
    pub const fn evidence(self) -> EventEvidence<S> {
        self.horizon_event.evidence()
    }
}

/// Complete twilight result over one closed physical-time interval.
#[derive(Debug, Clone, PartialEq)]
pub struct TwilightSearch<S: TimeScale> {
    level: TwilightLevel,
    state: TwilightState,
    horizon_search: HorizonEventSearch<S>,
    events: Vec<TwilightEvent<S>>,
}

impl<S: TimeScale> TwilightSearch<S> {
    /// Returns the selected standard or custom twilight level.
    pub const fn level(&self) -> TwilightLevel {
        self.level
    }

    /// Returns the illumination-state classification over the interval.
    pub const fn state(&self) -> TwilightState {
        self.state
    }

    /// Returns dawn and dusk crossings in time order.
    pub fn events(&self) -> &[TwilightEvent<S>] {
        &self.events
    }

    /// Returns the complete underlying solar horizon search, including transits.
    pub const fn horizon_search(&self) -> &HorizonEventSearch<S> {
        &self.horizon_search
    }
}

impl<'context, 'data, 'eop> Events<'context, 'data, EarthOrientationTable<'eop>> {
    /// Finds every dawn and dusk crossing for one twilight level.
    pub fn twilight_events_in<S: TimeScale>(
        &self,
        site: &FixedSite,
        interval: TimeInterval<S>,
        level: TwilightLevel,
        options: HorizonSearchOptions,
    ) -> Result<TwilightSearch<S>, Error> {
        let horizon_search = self.horizon_events_in(
            site,
            CelestialBody::Sun,
            interval,
            HorizonCriterion::vacuum_altitude(level.solar_altitude()),
            options,
        )?;
        let state = match horizon_search.visibility() {
            HorizonVisibility::RisesAndSets => TwilightState::DawnAndDusk,
            HorizonVisibility::CircumpolarOverInterval => TwilightState::ContinuousLight,
            HorizonVisibility::NeverRisesOverInterval => TwilightState::ContinuousDark,
            HorizonVisibility::TruncatedByInterval => TwilightState::TruncatedByInterval,
            HorizonVisibility::MultipleCycles => TwilightState::MultipleCycles,
            HorizonVisibility::GrazesCriterion => TwilightState::GrazesLevel,
        };
        let events = horizon_search
            .events()
            .iter()
            .filter_map(|event| match event.kind() {
                HorizonEventKind::Rise => Some(TwilightEvent {
                    kind: TwilightEventKind::Dawn,
                    horizon_event: *event,
                }),
                HorizonEventKind::Set => Some(TwilightEvent {
                    kind: TwilightEventKind::Dusk,
                    horizon_event: *event,
                }),
                HorizonEventKind::UpperTransit | HorizonEventKind::LowerTransit => None,
            })
            .collect();
        Ok(TwilightSearch {
            level,
            state,
            horizon_search,
            events,
        })
    }
}
