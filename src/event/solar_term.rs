use core::{f64::consts::TAU, fmt};

use std::vec::Vec;

use crate::{
    astro::SolarApparentPlace,
    ephem::EphemerisProvider,
    frame::EclipticLongitude,
    math::Angle,
    time::{
        CivilDateTime, Date, Duration, FixedUtcOffset, Gregorian, Instant, TimeInterval, TimeOfDay,
        TimeScale, Utc,
    },
};

use super::{AngularEventSearchOptions, Error, EventEvidence, Events, search::BracketedRootSearch};

/// One of the 24 conventional solar terms, each separated by 15° of apparent solar longitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolarTerm {
    /// 小寒, apparent solar longitude 285°.
    MinorCold,
    /// 大寒, apparent solar longitude 300°.
    MajorCold,
    /// 立春, apparent solar longitude 315°.
    StartOfSpring,
    /// 雨水, apparent solar longitude 330°.
    RainWater,
    /// 惊蛰, apparent solar longitude 345°.
    AwakeningOfInsects,
    /// 春分, apparent solar longitude 0°.
    SpringEquinox,
    /// 清明, apparent solar longitude 15°.
    ClearAndBright,
    /// 谷雨, apparent solar longitude 30°.
    GrainRain,
    /// 立夏, apparent solar longitude 45°.
    StartOfSummer,
    /// 小满, apparent solar longitude 60°.
    GrainFull,
    /// 芒种, apparent solar longitude 75°.
    GrainInEar,
    /// 夏至, apparent solar longitude 90°.
    SummerSolstice,
    /// 小暑, apparent solar longitude 105°.
    MinorHeat,
    /// 大暑, apparent solar longitude 120°.
    MajorHeat,
    /// 立秋, apparent solar longitude 135°.
    StartOfAutumn,
    /// 处暑, apparent solar longitude 150°.
    EndOfHeat,
    /// 白露, apparent solar longitude 165°.
    WhiteDew,
    /// 秋分, apparent solar longitude 180°.
    AutumnEquinox,
    /// 寒露, apparent solar longitude 195°.
    ColdDew,
    /// 霜降, apparent solar longitude 210°.
    FrostDescent,
    /// 立冬, apparent solar longitude 225°.
    StartOfWinter,
    /// 小雪, apparent solar longitude 240°.
    MinorSnow,
    /// 大雪, apparent solar longitude 255°.
    MajorSnow,
    /// 冬至, apparent solar longitude 270°.
    WinterSolstice,
}

impl SolarTerm {
    /// All solar terms in their chronological order within a fixed-offset Gregorian year.
    pub const ALL: [Self; 24] = [
        Self::MinorCold,
        Self::MajorCold,
        Self::StartOfSpring,
        Self::RainWater,
        Self::AwakeningOfInsects,
        Self::SpringEquinox,
        Self::ClearAndBright,
        Self::GrainRain,
        Self::StartOfSummer,
        Self::GrainFull,
        Self::GrainInEar,
        Self::SummerSolstice,
        Self::MinorHeat,
        Self::MajorHeat,
        Self::StartOfAutumn,
        Self::EndOfHeat,
        Self::WhiteDew,
        Self::AutumnEquinox,
        Self::ColdDew,
        Self::FrostDescent,
        Self::StartOfWinter,
        Self::MinorSnow,
        Self::MajorSnow,
        Self::WinterSolstice,
    ];

    /// Returns the conventional Chinese name.
    pub const fn chinese_name(self) -> &'static str {
        match self {
            Self::MinorCold => "小寒",
            Self::MajorCold => "大寒",
            Self::StartOfSpring => "立春",
            Self::RainWater => "雨水",
            Self::AwakeningOfInsects => "惊蛰",
            Self::SpringEquinox => "春分",
            Self::ClearAndBright => "清明",
            Self::GrainRain => "谷雨",
            Self::StartOfSummer => "立夏",
            Self::GrainFull => "小满",
            Self::GrainInEar => "芒种",
            Self::SummerSolstice => "夏至",
            Self::MinorHeat => "小暑",
            Self::MajorHeat => "大暑",
            Self::StartOfAutumn => "立秋",
            Self::EndOfHeat => "处暑",
            Self::WhiteDew => "白露",
            Self::AutumnEquinox => "秋分",
            Self::ColdDew => "寒露",
            Self::FrostDescent => "霜降",
            Self::StartOfWinter => "立冬",
            Self::MinorSnow => "小雪",
            Self::MajorSnow => "大雪",
            Self::WinterSolstice => "冬至",
        }
    }

    /// Returns the stable English name.
    pub const fn english_name(self) -> &'static str {
        match self {
            Self::MinorCold => "Minor Cold",
            Self::MajorCold => "Major Cold",
            Self::StartOfSpring => "Start of Spring",
            Self::RainWater => "Rain Water",
            Self::AwakeningOfInsects => "Awakening of Insects",
            Self::SpringEquinox => "Spring Equinox",
            Self::ClearAndBright => "Clear and Bright",
            Self::GrainRain => "Grain Rain",
            Self::StartOfSummer => "Start of Summer",
            Self::GrainFull => "Grain Full",
            Self::GrainInEar => "Grain in Ear",
            Self::SummerSolstice => "Summer Solstice",
            Self::MinorHeat => "Minor Heat",
            Self::MajorHeat => "Major Heat",
            Self::StartOfAutumn => "Start of Autumn",
            Self::EndOfHeat => "End of Heat",
            Self::WhiteDew => "White Dew",
            Self::AutumnEquinox => "Autumn Equinox",
            Self::ColdDew => "Cold Dew",
            Self::FrostDescent => "Frost Descent",
            Self::StartOfWinter => "Start of Winter",
            Self::MinorSnow => "Minor Snow",
            Self::MajorSnow => "Major Snow",
            Self::WinterSolstice => "Winter Solstice",
        }
    }

    /// Returns the defining apparent geocentric true-ecliptic longitude.
    pub const fn target_longitude(self) -> EclipticLongitude {
        EclipticLongitude::from_validated_radians(self.longitude_index() as f64 * TAU / 24.0)
    }

    const fn longitude_index(self) -> u8 {
        match self {
            Self::SpringEquinox => 0,
            Self::ClearAndBright => 1,
            Self::GrainRain => 2,
            Self::StartOfSummer => 3,
            Self::GrainFull => 4,
            Self::GrainInEar => 5,
            Self::SummerSolstice => 6,
            Self::MinorHeat => 7,
            Self::MajorHeat => 8,
            Self::StartOfAutumn => 9,
            Self::EndOfHeat => 10,
            Self::WhiteDew => 11,
            Self::AutumnEquinox => 12,
            Self::ColdDew => 13,
            Self::FrostDescent => 14,
            Self::StartOfWinter => 15,
            Self::MinorSnow => 16,
            Self::MajorSnow => 17,
            Self::WinterSolstice => 18,
            Self::MinorCold => 19,
            Self::MajorCold => 20,
            Self::StartOfSpring => 21,
            Self::RainWater => 22,
            Self::AwakeningOfInsects => 23,
        }
    }

    const fn from_longitude_index(index: i64) -> Self {
        match index.rem_euclid(24) {
            0 => Self::SpringEquinox,
            1 => Self::ClearAndBright,
            2 => Self::GrainRain,
            3 => Self::StartOfSummer,
            4 => Self::GrainFull,
            5 => Self::GrainInEar,
            6 => Self::SummerSolstice,
            7 => Self::MinorHeat,
            8 => Self::MajorHeat,
            9 => Self::StartOfAutumn,
            10 => Self::EndOfHeat,
            11 => Self::WhiteDew,
            12 => Self::AutumnEquinox,
            13 => Self::ColdDew,
            14 => Self::FrostDescent,
            15 => Self::StartOfWinter,
            16 => Self::MinorSnow,
            17 => Self::MajorSnow,
            18 => Self::WinterSolstice,
            19 => Self::MinorCold,
            20 => Self::MajorCold,
            21 => Self::StartOfSpring,
            22 => Self::RainWater,
            _ => Self::AwakeningOfInsects,
        }
    }
}

/// One converged apparent-solar-longitude crossing.
pub struct SolarTermEvent<S: TimeScale> {
    term: SolarTerm,
    apparent_sun: SolarApparentPlace<S>,
    evidence: EventEvidence<S>,
}

impl<S: TimeScale> SolarTermEvent<S> {
    /// Returns the identified solar term.
    pub const fn term(self) -> SolarTerm {
        self.term
    }

    /// Returns the geocentric reception instant of the longitude crossing.
    pub const fn instant(self) -> Instant<S> {
        self.apparent_sun.reception_epoch()
    }

    /// Returns the fully evaluated apparent solar coordinates at the event.
    pub const fn apparent_sun(self) -> SolarApparentPlace<S> {
        self.apparent_sun
    }

    /// Returns numerical convergence evidence for the event.
    pub const fn evidence(self) -> EventEvidence<S> {
        self.evidence
    }
}

impl<S: TimeScale> Copy for SolarTermEvent<S> {}

impl<S: TimeScale> Clone for SolarTermEvent<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for SolarTermEvent<S> {
    fn eq(&self, other: &Self) -> bool {
        self.term == other.term
            && self.apparent_sun == other.apparent_sun
            && self.evidence == other.evidence
    }
}

impl<S: TimeScale> fmt::Debug for SolarTermEvent<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SolarTermEvent")
            .field("term", &self.term)
            .field("apparent_sun", &self.apparent_sun)
            .field("evidence", &self.evidence)
            .finish()
    }
}

/// One solar-term event paired with its fixed-offset Gregorian civil label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarTermYearEntry {
    event: SolarTermEvent<Utc>,
    local_time: CivilDateTime<Gregorian>,
}

impl SolarTermYearEntry {
    /// Returns the astronomical event in physical UTC time.
    pub const fn event(self) -> SolarTermEvent<Utc> {
        self.event
    }

    /// Returns the conventional local label in the year's fixed offset.
    pub const fn local_time(self) -> CivilDateTime<Gregorian> {
        self.local_time
    }
}

/// Exactly 24 chronologically ordered solar terms in one fixed-offset Gregorian year.
#[derive(Debug, Clone, PartialEq)]
pub struct SolarTermYear {
    year: i32,
    offset: FixedUtcOffset,
    entries: [SolarTermYearEntry; 24],
}

impl SolarTermYear {
    /// Returns the local Gregorian year.
    pub const fn year(&self) -> i32 {
        self.year
    }

    /// Returns the fixed offset used to select and label the year.
    pub const fn offset(&self) -> FixedUtcOffset {
        self.offset
    }

    /// Returns the 24 entries from Minor Cold through Winter Solstice.
    pub const fn entries(&self) -> &[SolarTermYearEntry; 24] {
        &self.entries
    }
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Events<'context, 'data, E, P> {
    /// Finds every solar-term crossing in a closed physical-time interval.
    pub fn solar_terms_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<SolarTermEvent<S>>, Error> {
        const TERM_STEP: f64 = TAU / 24.0;

        let mut evaluations = 0_u32;
        let mut previous_epoch = interval.start();
        let initial_apparent = self.evaluate_sun(previous_epoch, options, &mut evaluations)?;
        let mut previous_wrapped = initial_apparent.longitude().as_radians();
        let mut previous_unwrapped = previous_wrapped;
        let mut events = Vec::new();

        let nearest_index = libm::round(previous_wrapped / TERM_STEP) as i64;
        let nearest_term = SolarTerm::from_longitude_index(nearest_index);
        let initial_residual = Self::longitude_residual(
            previous_wrapped,
            nearest_term.target_longitude().as_radians(),
        )?;
        if initial_residual.abs() <= options.angular_tolerance().as_radians() {
            events.push(SolarTermEvent {
                term: nearest_term,
                apparent_sun: initial_apparent,
                evidence: EventEvidence::new(
                    previous_epoch,
                    previous_epoch,
                    Duration::ZERO,
                    Angle::from_radians(initial_residual)?,
                    0,
                    1,
                ),
            });
        }

        while previous_epoch < interval.end() {
            let remaining = interval.end().duration_since(previous_epoch)?;
            let step = if remaining < options.scan_step() {
                remaining
            } else {
                options.scan_step()
            };
            let current_epoch = previous_epoch.checked_add(step)?;
            let current_apparent = self.evaluate_sun(current_epoch, options, &mut evaluations)?;
            let current_wrapped = current_apparent.longitude().as_radians();
            let advance = Self::longitude_residual(current_wrapped, previous_wrapped)?;
            if advance <= 0.0 {
                return Err(Error::SolarLongitudeNotIncreasing {
                    previous_radians: previous_wrapped,
                    current_radians: current_wrapped,
                    previous_tai_nanoseconds: previous_epoch.tai_nanoseconds_since_1900(),
                    current_tai_nanoseconds: current_epoch.tai_nanoseconds_since_1900(),
                });
            }
            let current_unwrapped = previous_unwrapped + advance;
            let mut boundary_index = libm::floor(previous_unwrapped / TERM_STEP) as i64 + 1;
            let mut boundary = boundary_index as f64 * TERM_STEP;
            while boundary <= current_unwrapped + options.angular_tolerance().as_radians() {
                if boundary > previous_unwrapped + options.angular_tolerance().as_radians() {
                    let term = SolarTerm::from_longitude_index(boundary_index);
                    let event = self.refine_solar_term(
                        term,
                        previous_epoch,
                        current_epoch,
                        options,
                        &mut evaluations,
                    )?;
                    Self::push_unique(&mut events, event, options.time_tolerance())?;
                }
                boundary_index += 1;
                boundary = boundary_index as f64 * TERM_STEP;
            }

            previous_epoch = current_epoch;
            previous_wrapped = current_wrapped;
            previous_unwrapped = current_unwrapped;
        }

        Ok(events)
    }

    /// Computes the 24 solar terms belonging to one fixed-offset Gregorian year.
    pub fn solar_term_year(
        &self,
        year: i32,
        offset: FixedUtcOffset,
        options: AngularEventSearchOptions,
    ) -> Result<SolarTermYear, Error> {
        let next_year = year.checked_add(1).ok_or(crate::time::Error::Overflow {
            operation: "advancing a solar-term Gregorian year",
        })?;
        let start_label = CivilDateTime::new(
            Date::<Gregorian>::new(year, 1, 1)?,
            TimeOfDay::MIDNIGHT,
            offset,
        )?;
        let end_label = CivilDateTime::new(
            Date::<Gregorian>::new(next_year, 1, 1)?,
            TimeOfDay::MIDNIGHT,
            offset,
        )?;
        let time = self.astrometry.time_context();
        let interval = TimeInterval::new(
            time.resolve_fixed(start_label)?,
            time.resolve_fixed(end_label)?,
        )?;
        let events = self.solar_terms_in(interval, options)?;
        let mut entries = Vec::with_capacity(24);
        for event in events {
            let local_time = time.represent_fixed::<Gregorian, _>(event.instant(), offset)?;
            if local_time.date().year() == year {
                entries.push(SolarTermYearEntry { event, local_time });
            }
        }
        if entries.len() != 24 {
            return Err(Error::IncompleteSolarTermYear {
                year,
                found: entries.len(),
            });
        }
        for (index, entry) in entries.iter().enumerate() {
            let expected = SolarTerm::ALL[index];
            let actual = entry.event.term();
            if actual != expected {
                return Err(Error::UnexpectedSolarTermSequence {
                    year,
                    index,
                    expected: expected.english_name(),
                    actual: actual.english_name(),
                });
            }
        }
        let entries: [SolarTermYearEntry; 24] =
            entries
                .try_into()
                .map_err(
                    |entries: Vec<SolarTermYearEntry>| Error::IncompleteSolarTermYear {
                        year,
                        found: entries.len(),
                    },
                )?;
        Ok(SolarTermYear {
            year,
            offset,
            entries,
        })
    }

    fn evaluate_sun<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        options: AngularEventSearchOptions,
        evaluations: &mut u32,
    ) -> Result<SolarApparentPlace<S>, Error> {
        if *evaluations >= options.max_evaluations() {
            return Err(Error::EvaluationLimitExceeded {
                maximum: options.max_evaluations(),
            });
        }
        *evaluations += 1;
        self.astrometry
            .solar_apparent_place(epoch, options.light_time())
            .map_err(Error::from)
    }

    fn refine_solar_term<S: TimeScale>(
        &self,
        term: SolarTerm,
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        options: AngularEventSearchOptions,
        evaluations: &mut u32,
    ) -> Result<SolarTermEvent<S>, Error> {
        let evaluations_before = *evaluations;
        let root = BracketedRootSearch::refine(
            bracket_start,
            bracket_end,
            options.time_tolerance(),
            options.max_refinement_iterations(),
            |epoch| {
                let apparent = self.evaluate_sun(epoch, options, evaluations)?;
                Self::longitude_residual(
                    apparent.longitude().as_radians(),
                    term.target_longitude().as_radians(),
                )
                .map_err(Error::from)
            },
        )?;
        let root_epoch = root.instant();
        let apparent_sun = self.evaluate_sun(root_epoch, options, evaluations)?;
        let residual = Self::longitude_residual(
            apparent_sun.longitude().as_radians(),
            term.target_longitude().as_radians(),
        )?;
        if residual.abs() > options.angular_tolerance().as_radians() {
            return Err(Error::SolarTermResidualExceeded {
                term: term.english_name(),
                residual_radians: residual.abs(),
                tolerance_radians: options.angular_tolerance().as_radians(),
            });
        }

        let final_start = root.bracket_start();
        let final_end = root.bracket_end();
        let uncertainty = root.time_uncertainty();
        Ok(SolarTermEvent {
            term,
            apparent_sun,
            evidence: EventEvidence::new(
                final_start,
                final_end,
                uncertainty,
                Angle::from_radians(residual)?,
                root.iterations(),
                *evaluations - evaluations_before,
            ),
        })
    }

    fn longitude_residual(longitude: f64, target: f64) -> Result<f64, crate::math::Error> {
        Angle::wrap_signed(longitude - target, "solar-term longitude residual")
    }

    fn push_unique<S: TimeScale>(
        events: &mut Vec<SolarTermEvent<S>>,
        candidate: SolarTermEvent<S>,
        tolerance: Duration,
    ) -> Result<(), Error> {
        if let Some(previous) = events.last_mut()
            && previous.term() == candidate.term()
            && candidate
                .instant()
                .duration_since(previous.instant())?
                .checked_abs()?
                <= tolerance
        {
            if candidate.evidence().residual().as_radians().abs()
                < previous.evidence().residual().as_radians().abs()
            {
                *previous = candidate;
            }
            return Ok(());
        }
        events.push(candidate);
        Ok(())
    }
}
