use core::{
    f64::consts::{FRAC_PI_2, TAU},
    fmt,
};

use std::vec::Vec;

use crate::{
    astro::{GeocentricApparentPlace, MoonPhaseAngle, SolarApparentPlace},
    ephem::CelestialBody,
    math::Angle,
    time::{
        CivilDateTime, Date, Duration, FixedUtcOffset, Gregorian, Instant, TimeInterval, TimeOfDay,
        TimeScale, Utc,
    },
};

use super::{AngularEventSearchOptions, Error, EventEvidence, Events, search::BracketedRootSearch};

/// One of the four primary lunar phases, defined by apparent geocentric longitude difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoonPhase {
    /// New Moon, where the Moon's apparent ecliptic longitude exceeds the Sun's by 0°.
    NewMoon,
    /// First quarter, at a 90° apparent ecliptic-longitude difference.
    FirstQuarter,
    /// Full Moon, at a 180° apparent ecliptic-longitude difference.
    FullMoon,
    /// Last quarter, at a 270° apparent ecliptic-longitude difference.
    LastQuarter,
}

impl MoonPhase {
    /// All primary phases in chronological order within one lunation.
    pub const ALL: [Self; 4] = [
        Self::NewMoon,
        Self::FirstQuarter,
        Self::FullMoon,
        Self::LastQuarter,
    ];

    /// Returns the stable English phase name.
    pub const fn english_name(self) -> &'static str {
        match self {
            Self::NewMoon => "New Moon",
            Self::FirstQuarter => "First Quarter",
            Self::FullMoon => "Full Moon",
            Self::LastQuarter => "Last Quarter",
        }
    }

    /// Returns the defining directed apparent lunar phase-cycle angle.
    pub const fn target_longitude_difference(self) -> MoonPhaseAngle {
        MoonPhaseAngle::from_validated_radians(self.longitude_index() as f64 * FRAC_PI_2)
    }

    const fn longitude_index(self) -> u8 {
        match self {
            Self::NewMoon => 0,
            Self::FirstQuarter => 1,
            Self::FullMoon => 2,
            Self::LastQuarter => 3,
        }
    }

    const fn from_longitude_index(index: i64) -> Self {
        match index.rem_euclid(4) {
            0 => Self::NewMoon,
            1 => Self::FirstQuarter,
            2 => Self::FullMoon,
            _ => Self::LastQuarter,
        }
    }
}

/// One converged crossing of an arbitrary directed lunar phase-cycle angle.
pub struct MoonPhaseAngleEvent<S: TimeScale> {
    target: MoonPhaseAngle,
    apparent_moon: GeocentricApparentPlace<S>,
    apparent_sun: SolarApparentPlace<S>,
    longitude_difference: MoonPhaseAngle,
    evidence: EventEvidence<S>,
}

impl<S: TimeScale> MoonPhaseAngleEvent<S> {
    const fn new(
        target: MoonPhaseAngle,
        apparent_moon: GeocentricApparentPlace<S>,
        apparent_sun: SolarApparentPlace<S>,
        longitude_difference: MoonPhaseAngle,
        evidence: EventEvidence<S>,
    ) -> Self {
        Self {
            target,
            apparent_moon,
            apparent_sun,
            longitude_difference,
            evidence,
        }
    }

    /// Returns the directed phase-cycle angle that defines this event.
    pub const fn target(self) -> MoonPhaseAngle {
        self.target
    }

    /// Returns the common geocentric reception instant of the apparent positions.
    pub const fn instant(self) -> Instant<S> {
        self.apparent_moon.reception_epoch()
    }

    /// Returns the fully evaluated apparent geocentric place of the Moon.
    pub const fn apparent_moon(self) -> GeocentricApparentPlace<S> {
        self.apparent_moon
    }

    /// Returns the fully evaluated apparent geocentric place of the Sun.
    pub const fn apparent_sun(self) -> SolarApparentPlace<S> {
        self.apparent_sun
    }

    /// Returns the refined apparent lunar-minus-solar longitude in `[0, 2π)`.
    pub const fn longitude_difference(self) -> MoonPhaseAngle {
        self.longitude_difference
    }

    /// Returns numerical convergence evidence for the event.
    pub const fn evidence(self) -> EventEvidence<S> {
        self.evidence
    }
}

impl<S: TimeScale> Copy for MoonPhaseAngleEvent<S> {}

impl<S: TimeScale> Clone for MoonPhaseAngleEvent<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for MoonPhaseAngleEvent<S> {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target
            && self.apparent_moon == other.apparent_moon
            && self.apparent_sun == other.apparent_sun
            && self.longitude_difference == other.longitude_difference
            && self.evidence == other.evidence
    }
}

impl<S: TimeScale> fmt::Debug for MoonPhaseAngleEvent<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MoonPhaseAngleEvent")
            .field("target", &self.target)
            .field("apparent_moon", &self.apparent_moon)
            .field("apparent_sun", &self.apparent_sun)
            .field("longitude_difference", &self.longitude_difference)
            .field("evidence", &self.evidence)
            .finish()
    }
}

/// One converged primary lunar-phase event.
pub struct MoonPhaseEvent<S: TimeScale> {
    phase: MoonPhase,
    angle_event: MoonPhaseAngleEvent<S>,
}

impl<S: TimeScale> MoonPhaseEvent<S> {
    const fn new(phase: MoonPhase, angle_event: MoonPhaseAngleEvent<S>) -> Self {
        Self { phase, angle_event }
    }

    /// Returns the identified primary phase.
    pub const fn phase(self) -> MoonPhase {
        self.phase
    }

    /// Returns the underlying directed phase-angle event.
    pub const fn angle_event(self) -> MoonPhaseAngleEvent<S> {
        self.angle_event
    }

    /// Returns the common geocentric reception instant of the apparent positions.
    pub const fn instant(self) -> Instant<S> {
        self.angle_event.instant()
    }

    /// Returns the fully evaluated apparent geocentric place of the Moon.
    pub const fn apparent_moon(self) -> GeocentricApparentPlace<S> {
        self.angle_event.apparent_moon()
    }

    /// Returns the fully evaluated apparent geocentric place of the Sun.
    pub const fn apparent_sun(self) -> SolarApparentPlace<S> {
        self.angle_event.apparent_sun()
    }

    /// Returns the refined directed apparent lunar phase-cycle angle.
    pub const fn longitude_difference(self) -> MoonPhaseAngle {
        self.angle_event.longitude_difference()
    }

    /// Returns numerical convergence evidence for the event.
    pub const fn evidence(self) -> EventEvidence<S> {
        self.angle_event.evidence()
    }
}

impl<S: TimeScale> Copy for MoonPhaseEvent<S> {}

impl<S: TimeScale> Clone for MoonPhaseEvent<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for MoonPhaseEvent<S> {
    fn eq(&self, other: &Self) -> bool {
        self.phase == other.phase && self.angle_event == other.angle_event
    }
}

impl<S: TimeScale> fmt::Debug for MoonPhaseEvent<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MoonPhaseEvent")
            .field("phase", &self.phase)
            .field("angle_event", &self.angle_event)
            .finish()
    }
}

/// One primary lunar phase paired with its fixed-offset Gregorian civil label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoonPhaseYearEntry {
    event: MoonPhaseEvent<Utc>,
    local_time: CivilDateTime<Gregorian>,
}

impl MoonPhaseYearEntry {
    /// Returns the astronomical event in physical UTC time.
    pub const fn event(self) -> MoonPhaseEvent<Utc> {
        self.event
    }

    /// Returns the event's civil label in the requested fixed UTC offset.
    pub const fn local_time(self) -> CivilDateTime<Gregorian> {
        self.local_time
    }
}

/// Chronologically ordered primary lunar phases belonging to one fixed-offset Gregorian year.
#[derive(Debug, Clone, PartialEq)]
pub struct MoonPhaseYear {
    year: i32,
    offset: FixedUtcOffset,
    entries: Vec<MoonPhaseYearEntry>,
}

impl MoonPhaseYear {
    /// Returns the selected local Gregorian year.
    pub const fn year(&self) -> i32 {
        self.year
    }

    /// Returns the fixed UTC offset used to select and label the year.
    pub const fn offset(&self) -> FixedUtcOffset {
        self.offset
    }

    /// Returns every primary phase whose civil label belongs to the selected year.
    pub fn entries(&self) -> &[MoonPhaseYearEntry] {
        &self.entries
    }
}

struct PhaseEvaluation<S: TimeScale> {
    apparent_moon: GeocentricApparentPlace<S>,
    apparent_sun: SolarApparentPlace<S>,
    longitude_difference: f64,
}

impl<'context, 'data, E> Events<'context, 'data, E> {
    /// Finds every crossing of one directed apparent lunar phase angle in a closed interval.
    ///
    /// The target is the Moon-minus-Sun apparent geocentric longitude difference on true ecliptic
    /// and equinox of date axes. One event normally occurs per synodic month. This directed
    /// criterion distinguishes, for example, 45° on the waxing branch from 315° on the waning
    /// branch; it is not the unsigned physical phase angle measured at the Moon.
    pub fn moon_phase_angle_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        target: MoonPhaseAngle,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MoonPhaseAngleEvent<S>>, Error> {
        let mut evaluations = 0_u32;
        let mut previous_epoch = interval.start();
        let initial = self.evaluate_phase(previous_epoch, options, &mut evaluations)?;
        let mut previous_wrapped = initial.longitude_difference;
        let mut previous_unwrapped = previous_wrapped;
        let mut events = Vec::new();

        let initial_residual = Self::phase_residual(previous_wrapped, target.as_radians())?;
        if initial_residual.abs() <= options.angular_tolerance().as_radians() {
            events.push(MoonPhaseAngleEvent::new(
                target,
                initial.apparent_moon,
                initial.apparent_sun,
                MoonPhaseAngle::from_validated_radians(initial.longitude_difference),
                EventEvidence::new(
                    previous_epoch,
                    previous_epoch,
                    Duration::ZERO,
                    Angle::from_radians(initial_residual)?,
                    0,
                    evaluations,
                ),
            ));
        }

        while previous_epoch < interval.end() {
            let remaining = interval.end().duration_since(previous_epoch)?;
            let step = if remaining < options.scan_step() {
                remaining
            } else {
                options.scan_step()
            };
            let current_epoch = previous_epoch.checked_add(step)?;
            let current = self.evaluate_phase(current_epoch, options, &mut evaluations)?;
            let current_wrapped = current.longitude_difference;
            let advance = Self::phase_residual(current_wrapped, previous_wrapped)?;
            if advance <= 0.0 {
                return Err(Error::MoonElongationNotIncreasing {
                    previous_radians: previous_wrapped,
                    current_radians: current_wrapped,
                    previous_tai_nanoseconds: previous_epoch.tai_nanoseconds_since_1900(),
                    current_tai_nanoseconds: current_epoch.tai_nanoseconds_since_1900(),
                });
            }
            let current_unwrapped = previous_unwrapped + advance;
            let cycle_index =
                libm::floor((previous_unwrapped - target.as_radians()) / TAU) as i64 + 1;
            let boundary = target.as_radians() + cycle_index as f64 * TAU;
            if boundary <= current_unwrapped + options.angular_tolerance().as_radians()
                && boundary > previous_unwrapped + options.angular_tolerance().as_radians()
            {
                let event = self.refine_phase_angle(
                    target,
                    "directed Moon phase angle",
                    previous_epoch,
                    current_epoch,
                    options,
                    &mut evaluations,
                )?;
                Self::push_unique_phase_angle(&mut events, event, options.time_tolerance())?;
            }

            previous_epoch = current_epoch;
            previous_wrapped = current_wrapped;
            previous_unwrapped = current_unwrapped;
        }

        Ok(events)
    }

    /// Finds every primary lunar phase in a closed physical-time interval.
    ///
    /// The phase criterion is the excess of the Moon's apparent geocentric longitude over the
    /// Sun's on true ecliptic and equinox of date axes. New Moon, first quarter, Full Moon, and
    /// last quarter occur at directed differences of 0°, 90°, 180°, and 270°, respectively.
    pub fn moon_phases_in<S: TimeScale>(
        &self,
        interval: TimeInterval<S>,
        options: AngularEventSearchOptions,
    ) -> Result<Vec<MoonPhaseEvent<S>>, Error> {
        let mut evaluations = 0_u32;
        let mut previous_epoch = interval.start();
        let initial = self.evaluate_phase(previous_epoch, options, &mut evaluations)?;
        let mut previous_wrapped = initial.longitude_difference;
        let mut previous_unwrapped = previous_wrapped;
        let mut events = Vec::new();

        let nearest_index = libm::round(previous_wrapped / FRAC_PI_2) as i64;
        let nearest_phase = MoonPhase::from_longitude_index(nearest_index);
        let initial_residual = Self::phase_residual(
            previous_wrapped,
            nearest_phase.target_longitude_difference().as_radians(),
        )?;
        if initial_residual.abs() <= options.angular_tolerance().as_radians() {
            events.push(MoonPhaseEvent::new(
                nearest_phase,
                MoonPhaseAngleEvent::new(
                    nearest_phase.target_longitude_difference(),
                    initial.apparent_moon,
                    initial.apparent_sun,
                    MoonPhaseAngle::from_validated_radians(initial.longitude_difference),
                    EventEvidence::new(
                        previous_epoch,
                        previous_epoch,
                        Duration::ZERO,
                        Angle::from_radians(initial_residual)?,
                        0,
                        evaluations,
                    ),
                ),
            ));
        }

        while previous_epoch < interval.end() {
            let remaining = interval.end().duration_since(previous_epoch)?;
            let step = if remaining < options.scan_step() {
                remaining
            } else {
                options.scan_step()
            };
            let current_epoch = previous_epoch.checked_add(step)?;
            let current = self.evaluate_phase(current_epoch, options, &mut evaluations)?;
            let current_wrapped = current.longitude_difference;
            let advance = Self::phase_residual(current_wrapped, previous_wrapped)?;
            if advance <= 0.0 {
                return Err(Error::MoonElongationNotIncreasing {
                    previous_radians: previous_wrapped,
                    current_radians: current_wrapped,
                    previous_tai_nanoseconds: previous_epoch.tai_nanoseconds_since_1900(),
                    current_tai_nanoseconds: current_epoch.tai_nanoseconds_since_1900(),
                });
            }
            let current_unwrapped = previous_unwrapped + advance;
            let mut boundary_index = libm::floor(previous_unwrapped / FRAC_PI_2) as i64 + 1;
            let mut boundary = boundary_index as f64 * FRAC_PI_2;
            while boundary <= current_unwrapped + options.angular_tolerance().as_radians() {
                if boundary > previous_unwrapped + options.angular_tolerance().as_radians() {
                    let phase = MoonPhase::from_longitude_index(boundary_index);
                    let event = self.refine_moon_phase(
                        phase,
                        previous_epoch,
                        current_epoch,
                        options,
                        &mut evaluations,
                    )?;
                    Self::push_unique_phase(&mut events, event, options.time_tolerance())?;
                }
                boundary_index += 1;
                boundary = boundary_index as f64 * FRAC_PI_2;
            }

            previous_epoch = current_epoch;
            previous_wrapped = current_wrapped;
            previous_unwrapped = current_unwrapped;
        }

        Ok(events)
    }

    /// Computes all primary lunar phases belonging to one fixed-offset Gregorian year.
    pub fn moon_phase_year(
        &self,
        year: i32,
        offset: FixedUtcOffset,
        options: AngularEventSearchOptions,
    ) -> Result<MoonPhaseYear, Error> {
        let next_year = year.checked_add(1).ok_or(crate::time::Error::Overflow {
            operation: "advancing a lunar-phase Gregorian year",
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
        let events = self.moon_phases_in(interval, options)?;
        let mut entries = Vec::with_capacity(events.len());
        for event in events {
            let local_time = time.represent_fixed::<Gregorian, _>(event.instant(), offset)?;
            if local_time.date().year() == year {
                entries.push(MoonPhaseYearEntry { event, local_time });
            }
        }
        Ok(MoonPhaseYear {
            year,
            offset,
            entries,
        })
    }

    fn evaluate_phase<S: TimeScale>(
        &self,
        epoch: Instant<S>,
        options: AngularEventSearchOptions,
        evaluations: &mut u32,
    ) -> Result<PhaseEvaluation<S>, Error> {
        if options.max_evaluations().saturating_sub(*evaluations) < 2 {
            return Err(Error::EvaluationLimitExceeded {
                maximum: options.max_evaluations(),
            });
        }
        *evaluations += 2;
        let apparent_moon = self.astrometry.geocentric_apparent_place(
            CelestialBody::Moon,
            epoch,
            options.light_time(),
        )?;
        let apparent_sun = self
            .astrometry
            .solar_apparent_place(epoch, options.light_time())?;
        let longitude_difference = Angle::wrap_zero_tau(
            apparent_moon.longitude().as_radians() - apparent_sun.longitude().as_radians(),
            "apparent lunar-minus-solar longitude",
        )?;
        Ok(PhaseEvaluation {
            apparent_moon,
            apparent_sun,
            longitude_difference,
        })
    }

    fn refine_moon_phase<S: TimeScale>(
        &self,
        phase: MoonPhase,
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        options: AngularEventSearchOptions,
        evaluations: &mut u32,
    ) -> Result<MoonPhaseEvent<S>, Error> {
        self.refine_phase_angle(
            phase.target_longitude_difference(),
            phase.english_name(),
            bracket_start,
            bracket_end,
            options,
            evaluations,
        )
        .map(|event| MoonPhaseEvent::new(phase, event))
    }

    fn refine_phase_angle<S: TimeScale>(
        &self,
        target: MoonPhaseAngle,
        event_name: &'static str,
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        options: AngularEventSearchOptions,
        evaluations: &mut u32,
    ) -> Result<MoonPhaseAngleEvent<S>, Error> {
        let evaluations_before = *evaluations;
        // A millisecond-wide root can leave the fast lunar elongation just outside the
        // standard ten-picoradian residual. A microsecond is still caller-compatible and
        // avoids the cost of refining every event all the way to nanosecond resolution.
        let microsecond = Duration::from_nanoseconds(1_000);
        let refinement_time_tolerance = if options.time_tolerance() < microsecond {
            options.time_tolerance()
        } else {
            microsecond
        };
        let root = BracketedRootSearch::refine(
            bracket_start,
            bracket_end,
            refinement_time_tolerance,
            options.max_refinement_iterations(),
            |epoch| {
                let evaluated = self.evaluate_phase(epoch, options, evaluations)?;
                Self::phase_residual(evaluated.longitude_difference, target.as_radians())
                    .map_err(Error::from)
            },
        )?;
        let evaluated = self.evaluate_phase(root.instant(), options, evaluations)?;
        let residual = Self::phase_residual(evaluated.longitude_difference, target.as_radians())?;
        if residual.abs() > options.angular_tolerance().as_radians() {
            return Err(Error::AngularResidualExceeded {
                event: event_name,
                residual_radians: residual.abs(),
                tolerance_radians: options.angular_tolerance().as_radians(),
            });
        }

        Ok(MoonPhaseAngleEvent::new(
            target,
            evaluated.apparent_moon,
            evaluated.apparent_sun,
            MoonPhaseAngle::from_validated_radians(evaluated.longitude_difference),
            EventEvidence::new(
                root.bracket_start(),
                root.bracket_end(),
                root.time_uncertainty(),
                Angle::from_radians(residual)?,
                root.iterations(),
                *evaluations - evaluations_before,
            ),
        ))
    }

    fn phase_residual(longitude_difference: f64, target: f64) -> Result<f64, crate::math::Error> {
        Angle::wrap_signed(
            longitude_difference - target,
            "lunar-phase longitude residual",
        )
    }

    fn push_unique_phase_angle<S: TimeScale>(
        events: &mut Vec<MoonPhaseAngleEvent<S>>,
        candidate: MoonPhaseAngleEvent<S>,
        tolerance: Duration,
    ) -> Result<(), Error> {
        if let Some(previous) = events.last_mut()
            && previous.target() == candidate.target()
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

    fn push_unique_phase<S: TimeScale>(
        events: &mut Vec<MoonPhaseEvent<S>>,
        candidate: MoonPhaseEvent<S>,
        tolerance: Duration,
    ) -> Result<(), Error> {
        if let Some(previous) = events.last_mut()
            && previous.phase() == candidate.phase()
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
