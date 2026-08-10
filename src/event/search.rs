use core::fmt;
use std::vec::Vec;

use crate::{
    astro::{Astrometry, ReceptionLightTimeOptions},
    ephem::EphemerisProvider,
    math::{Angle, RootOptions},
    time::{Duration, Instant, TimeScale},
};

use super::Error;

/// Explicit scanning, refinement, and evaluation controls for angular events.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AngularEventSearchOptions {
    scan_step: Duration,
    time_tolerance: Duration,
    angular_tolerance: Angle,
    max_refinement_iterations: u32,
    max_evaluations: u32,
    light_time: ReceptionLightTimeOptions,
}

impl AngularEventSearchOptions {
    /// Largest supported coarse step for monotonic angular-event scanning.
    pub const MAX_SCAN_STEP: Duration =
        Duration::from_nanoseconds(7 * Duration::NANOSECONDS_PER_DAY);

    /// Constructs validated controls for scanning and refining angular events.
    pub fn new(
        scan_step: Duration,
        time_tolerance: Duration,
        angular_tolerance: Angle,
        max_refinement_iterations: u32,
        max_evaluations: u32,
        light_time: ReceptionLightTimeOptions,
    ) -> Result<Self, Error> {
        if scan_step <= Duration::ZERO || scan_step > Self::MAX_SCAN_STEP {
            return Err(Error::InvalidSearchDuration {
                field: "angular-event scan step",
                nanoseconds: scan_step.as_nanoseconds(),
                maximum_nanoseconds: Self::MAX_SCAN_STEP.as_nanoseconds(),
            });
        }
        if time_tolerance <= Duration::ZERO || time_tolerance > scan_step {
            return Err(Error::InvalidSearchDuration {
                field: "angular-event time tolerance",
                nanoseconds: time_tolerance.as_nanoseconds(),
                maximum_nanoseconds: scan_step.as_nanoseconds(),
            });
        }
        let maximum_angular_tolerance = 7.5_f64.to_radians();
        if angular_tolerance.as_radians() <= 0.0
            || angular_tolerance.as_radians() > maximum_angular_tolerance
        {
            return Err(Error::InvalidAngularTolerance {
                field: "angular-event tolerance",
                radians: angular_tolerance.as_radians(),
                maximum_radians: maximum_angular_tolerance,
            });
        }
        if max_refinement_iterations == 0 {
            return Err(Error::InvalidSearchLimit {
                field: "angular-event refinement iterations",
                value: max_refinement_iterations,
            });
        }
        if max_evaluations == 0 {
            return Err(Error::InvalidSearchLimit {
                field: "angular-event evaluations",
                value: max_evaluations,
            });
        }
        Ok(Self {
            scan_step,
            time_tolerance,
            angular_tolerance,
            max_refinement_iterations,
            max_evaluations,
            light_time,
        })
    }

    /// Returns seven-day scanning, one-millisecond timing, and ten-picoradian angular tolerances.
    pub const fn standard() -> Self {
        Self {
            scan_step: Self::MAX_SCAN_STEP,
            time_tolerance: Duration::from_nanoseconds(1_000_000),
            angular_tolerance: Angle::from_finite(1.0e-11),
            max_refinement_iterations: 64,
            max_evaluations: 4_096,
            light_time: ReceptionLightTimeOptions::standard(),
        }
    }

    /// Returns the maximum physical duration between coarse samples.
    pub const fn scan_step(self) -> Duration {
        self.scan_step
    }

    /// Returns the required final time-bracket width.
    pub const fn time_tolerance(self) -> Duration {
        self.time_tolerance
    }

    /// Returns the maximum accepted angular residual.
    pub const fn angular_tolerance(self) -> Angle {
        self.angular_tolerance
    }

    /// Returns the maximum Brent refinement iterations per event.
    pub const fn max_refinement_iterations(self) -> u32 {
        self.max_refinement_iterations
    }

    /// Returns the maximum astrometric evaluations for one search.
    pub const fn max_evaluations(self) -> u32 {
        self.max_evaluations
    }

    /// Returns the reception light-time controls used for each apparent position.
    pub const fn light_time(self) -> ReceptionLightTimeOptions {
        self.light_time
    }
}

/// Explicit scanning, refinement, and evaluation controls for bounded astronomical extrema.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtremumSearchOptions {
    scan_step: Duration,
    time_tolerance: Duration,
    max_refinement_iterations: u32,
    max_evaluations: u32,
    light_time: ReceptionLightTimeOptions,
}

impl ExtremumSearchOptions {
    /// Largest supported coarse step for detecting local extrema.
    pub const MAX_SCAN_STEP: Duration = AngularEventSearchOptions::MAX_SCAN_STEP;

    /// Constructs validated controls for scanning and refining extrema.
    pub fn new(
        scan_step: Duration,
        time_tolerance: Duration,
        max_refinement_iterations: u32,
        max_evaluations: u32,
        light_time: ReceptionLightTimeOptions,
    ) -> Result<Self, Error> {
        if scan_step <= Duration::ZERO || scan_step > Self::MAX_SCAN_STEP {
            return Err(Error::InvalidSearchDuration {
                field: "extremum scan step",
                nanoseconds: scan_step.as_nanoseconds(),
                maximum_nanoseconds: Self::MAX_SCAN_STEP.as_nanoseconds(),
            });
        }
        if time_tolerance <= Duration::ZERO || time_tolerance > scan_step {
            return Err(Error::InvalidSearchDuration {
                field: "extremum time tolerance",
                nanoseconds: time_tolerance.as_nanoseconds(),
                maximum_nanoseconds: scan_step.as_nanoseconds(),
            });
        }
        if max_refinement_iterations == 0 {
            return Err(Error::InvalidSearchLimit {
                field: "extremum refinement iterations",
                value: max_refinement_iterations,
            });
        }
        if max_evaluations == 0 {
            return Err(Error::InvalidSearchLimit {
                field: "extremum evaluations",
                value: max_evaluations,
            });
        }
        Ok(Self {
            scan_step,
            time_tolerance,
            max_refinement_iterations,
            max_evaluations,
            light_time,
        })
    }

    /// Returns seven-day scanning, one-millisecond timing, and a 4096-evaluation budget.
    pub const fn standard() -> Self {
        Self {
            scan_step: Self::MAX_SCAN_STEP,
            time_tolerance: Duration::from_nanoseconds(1_000_000),
            max_refinement_iterations: 64,
            max_evaluations: 4_096,
            light_time: ReceptionLightTimeOptions::standard(),
        }
    }

    /// Returns the maximum physical duration between coarse samples.
    pub const fn scan_step(self) -> Duration {
        self.scan_step
    }

    /// Returns the required final time-bracket width.
    pub const fn time_tolerance(self) -> Duration {
        self.time_tolerance
    }

    /// Returns the maximum bounded-Brent refinement iterations per extremum.
    pub const fn max_refinement_iterations(self) -> u32 {
        self.max_refinement_iterations
    }

    /// Returns the maximum astrometric evaluations for one search.
    pub const fn max_evaluations(self) -> u32 {
        self.max_evaluations
    }

    /// Returns the reception light-time controls used for each apparent position.
    pub const fn light_time(self) -> ReceptionLightTimeOptions {
        self.light_time
    }
}

/// Numerical evidence retained for one converged bounded extremum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtremumEvidence<S: TimeScale> {
    bracket_start: Instant<S>,
    bracket_end: Instant<S>,
    time_uncertainty: Duration,
    iterations: u32,
    evaluations: u32,
}

impl<S: TimeScale> ExtremumEvidence<S> {
    pub(super) const fn new(
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        time_uncertainty: Duration,
        iterations: u32,
        evaluations: u32,
    ) -> Self {
        Self {
            bracket_start,
            bracket_end,
            time_uncertainty,
            iterations,
            evaluations,
        }
    }

    /// Returns the final inclusive bracket start.
    pub const fn bracket_start(self) -> Instant<S> {
        self.bracket_start
    }

    /// Returns the final inclusive bracket end.
    pub const fn bracket_end(self) -> Instant<S> {
        self.bracket_end
    }

    /// Returns half the final bracket width.
    pub const fn time_uncertainty(self) -> Duration {
        self.time_uncertainty
    }

    /// Returns the completed bounded-Brent iterations.
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns the cumulative astrometric evaluations consumed by the search.
    pub const fn evaluations(self) -> u32 {
        self.evaluations
    }
}

/// Numerical evidence retained for one converged astronomical event.
pub struct EventEvidence<S: TimeScale> {
    bracket_start: Instant<S>,
    bracket_end: Instant<S>,
    time_uncertainty: Duration,
    residual: Angle,
    iterations: u32,
    evaluations: u32,
}

impl<S: TimeScale> EventEvidence<S> {
    pub(super) const fn new(
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        time_uncertainty: Duration,
        residual: Angle,
        iterations: u32,
        evaluations: u32,
    ) -> Self {
        Self {
            bracket_start,
            bracket_end,
            time_uncertainty,
            residual,
            iterations,
            evaluations,
        }
    }

    /// Returns the final inclusive bracket start.
    pub const fn bracket_start(self) -> Instant<S> {
        self.bracket_start
    }

    /// Returns the final inclusive bracket end.
    pub const fn bracket_end(self) -> Instant<S> {
        self.bracket_end
    }

    /// Returns half the final bracket width, rounded to the nearest nanosecond.
    pub const fn time_uncertainty(self) -> Duration {
        self.time_uncertainty
    }

    /// Returns the final signed residual of the event's defining angular criterion.
    pub const fn residual(self) -> Angle {
        self.residual
    }

    /// Returns the completed Brent iterations.
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns astrometric evaluations consumed while refining this event.
    pub const fn evaluations(self) -> u32 {
        self.evaluations
    }
}

impl<S: TimeScale> Copy for EventEvidence<S> {}

impl<S: TimeScale> Clone for EventEvidence<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TimeScale> PartialEq for EventEvidence<S> {
    fn eq(&self, other: &Self) -> bool {
        self.bracket_start == other.bracket_start
            && self.bracket_end == other.bracket_end
            && self.time_uncertainty == other.time_uncertainty
            && self.residual == other.residual
            && self.iterations == other.iterations
            && self.evaluations == other.evaluations
    }
}

impl<S: TimeScale> fmt::Debug for EventEvidence<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EventEvidence")
            .field("bracket_start", &self.bracket_start)
            .field("bracket_end", &self.bracket_end)
            .field("time_uncertainty", &self.time_uncertainty)
            .field("residual", &self.residual)
            .field("iterations", &self.iterations)
            .field("evaluations", &self.evaluations)
            .finish()
    }
}

/// Astronomical event algorithms backed by one immutable astrometry context.
pub struct Events<'context, 'data, E, P: EphemerisProvider + ?Sized> {
    pub(super) astrometry: Astrometry<'context, 'data, E, P>,
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Events<'context, 'data, E, P> {
    /// Constructs event algorithms from an existing astrometry context.
    pub const fn new(astrometry: Astrometry<'context, 'data, E, P>) -> Self {
        Self { astrometry }
    }

    /// Returns the astrometry context used for event evaluations.
    pub const fn astrometry(&self) -> Astrometry<'context, 'data, E, P> {
        self.astrometry
    }
}

pub(super) struct BracketedRoot<S: TimeScale> {
    instant: Instant<S>,
    bracket_start: Instant<S>,
    bracket_end: Instant<S>,
    time_uncertainty: Duration,
    iterations: u32,
}

impl<S: TimeScale> BracketedRoot<S> {
    pub(super) const fn instant(&self) -> Instant<S> {
        self.instant
    }

    pub(super) const fn bracket_start(&self) -> Instant<S> {
        self.bracket_start
    }

    pub(super) const fn bracket_end(&self) -> Instant<S> {
        self.bracket_end
    }

    pub(super) const fn time_uncertainty(&self) -> Duration {
        self.time_uncertainty
    }

    pub(super) const fn iterations(&self) -> u32 {
        self.iterations
    }
}

pub(super) struct BracketedRootSearch;

impl BracketedRootSearch {
    pub(super) fn refine<S, F>(
        bracket_start: Instant<S>,
        bracket_end: Instant<S>,
        time_tolerance: Duration,
        max_iterations: u32,
        mut evaluate: F,
    ) -> Result<BracketedRoot<S>, Error>
    where
        S: TimeScale,
        F: FnMut(Instant<S>) -> Result<f64, Error>,
    {
        let upper_seconds = bracket_end.duration_since(bracket_start)?.as_seconds_f64();
        let root_options = RootOptions::new(
            time_tolerance.as_seconds_f64(),
            f64::MIN_POSITIVE,
            max_iterations,
        )?;
        let mut evaluation_error = None;
        let root = root_options.brent(0.0, upper_seconds, |seconds| {
            if evaluation_error.is_some() {
                return f64::NAN;
            }
            let evaluated = Duration::from_seconds_f64(seconds)
                .and_then(|offset| bracket_start.checked_add(offset))
                .map_err(Error::from)
                .and_then(&mut evaluate);
            match evaluated {
                Ok(value) => value,
                Err(error) => {
                    evaluation_error = Some(error);
                    f64::NAN
                }
            }
        });
        if let Some(error) = evaluation_error {
            return Err(error);
        }
        let root = root?;
        let instant = bracket_start.checked_add(Duration::from_seconds_f64(root.root())?)?;
        let (final_start, final_end, uncertainty) = if root.residual() == 0.0 {
            (instant, instant, Duration::ZERO)
        } else {
            let final_start =
                bracket_start.checked_add(Duration::from_seconds_f64(root.lower())?)?;
            let final_end = bracket_start.checked_add(Duration::from_seconds_f64(root.upper())?)?;
            let uncertainty = Duration::from_seconds_f64((root.upper() - root.lower()) * 0.5)?;
            (final_start, final_end, uncertainty)
        };
        Ok(BracketedRoot {
            instant,
            bracket_start: final_start,
            bracket_end: final_end,
            time_uncertainty: uncertainty,
            iterations: root.iterations(),
        })
    }
}

pub(super) struct BracketedExtremum<S: TimeScale> {
    instant: Instant<S>,
    bracket_start: Instant<S>,
    bracket_end: Instant<S>,
    time_uncertainty: Duration,
    iterations: u32,
}

impl<S: TimeScale> BracketedExtremum<S> {
    pub(super) const fn instant(&self) -> Instant<S> {
        self.instant
    }

    pub(super) const fn bracket_start(&self) -> Instant<S> {
        self.bracket_start
    }

    pub(super) const fn bracket_end(&self) -> Instant<S> {
        self.bracket_end
    }

    pub(super) const fn time_uncertainty(&self) -> Duration {
        self.time_uncertainty
    }

    pub(super) const fn iterations(&self) -> u32 {
        self.iterations
    }
}

pub(super) struct BracketedExtremumSearch;

impl BracketedExtremumSearch {
    const GOLDEN_SECTION_COMPLEMENT: f64 = 0.381_966_011_250_105_1;

    pub(super) fn refine_minimum<S, F>(
        bracket_start: Instant<S>,
        initial: Instant<S>,
        bracket_end: Instant<S>,
        time_tolerance: Duration,
        max_iterations: u32,
        mut evaluate: F,
    ) -> Result<BracketedExtremum<S>, Error>
    where
        S: TimeScale,
        F: FnMut(Instant<S>) -> Result<f64, Error>,
    {
        let mut lower = 0.0;
        let mut upper = bracket_end.duration_since(bracket_start)?.as_seconds_f64();
        let mut best = initial.duration_since(bracket_start)?.as_seconds_f64();
        let mut previous_best = best;
        let mut third_best = best;
        let mut best_value = evaluate(initial)?;
        let mut previous_value = best_value;
        let mut third_value = best_value;
        let mut proposed_step = 0.0_f64;
        let mut previous_step = 0.0_f64;
        let required_uncertainty = time_tolerance.as_seconds_f64();

        for iteration in 1..=max_iterations {
            let midpoint = 0.5 * (lower + upper);
            let minimum_step = f64::EPSILON * best.abs() + required_uncertainty / 3.0;
            let convergence_width = 2.0 * minimum_step;
            if (best - midpoint).abs() <= convergence_width - 0.5 * (upper - lower) {
                let instant = bracket_start.checked_add(Duration::from_seconds_f64(best)?)?;
                let final_lower = lower.max(best - convergence_width);
                let final_upper = upper.min(best + convergence_width);
                let final_start =
                    bracket_start.checked_add(Duration::from_seconds_f64(final_lower)?)?;
                let final_end =
                    bracket_start.checked_add(Duration::from_seconds_f64(final_upper)?)?;
                return Ok(BracketedExtremum {
                    instant,
                    bracket_start: final_start,
                    bracket_end: final_end,
                    time_uncertainty: Duration::from_seconds_f64(
                        (final_upper - final_lower) * 0.5,
                    )?,
                    iterations: iteration - 1,
                });
            }
            if previous_step.abs() > minimum_step {
                let first = (best - previous_best) * (best_value - third_value);
                let second = (best - third_best) * (best_value - previous_value);
                let mut numerator = (best - third_best) * second - (best - previous_best) * first;
                let mut denominator = 2.0 * (second - first);
                if denominator > 0.0 {
                    numerator = -numerator;
                } else {
                    denominator = -denominator;
                }
                let saved_step = previous_step;
                previous_step = proposed_step;
                if denominator != 0.0
                    && numerator.abs() < (0.5 * denominator * saved_step).abs()
                    && numerator > denominator * (lower - best)
                    && numerator < denominator * (upper - best)
                {
                    proposed_step = numerator / denominator;
                    let candidate = best + proposed_step;
                    if candidate - lower < 2.0 * minimum_step
                        || upper - candidate < 2.0 * minimum_step
                    {
                        proposed_step = minimum_step.copysign(midpoint - best);
                    }
                } else {
                    previous_step = if best >= midpoint {
                        lower - best
                    } else {
                        upper - best
                    };
                    proposed_step = Self::GOLDEN_SECTION_COMPLEMENT * previous_step;
                }
            } else {
                previous_step = if best >= midpoint {
                    lower - best
                } else {
                    upper - best
                };
                proposed_step = Self::GOLDEN_SECTION_COMPLEMENT * previous_step;
            }

            let candidate = if proposed_step.abs() >= minimum_step {
                best + proposed_step
            } else {
                best + minimum_step.copysign(proposed_step)
            };
            let candidate_epoch =
                bracket_start.checked_add(Duration::from_seconds_f64(candidate)?)?;
            let candidate_value = evaluate(candidate_epoch)?;
            if candidate_value <= best_value {
                if candidate >= best {
                    lower = best;
                } else {
                    upper = best;
                }
                third_best = previous_best;
                third_value = previous_value;
                previous_best = best;
                previous_value = best_value;
                best = candidate;
                best_value = candidate_value;
            } else {
                if candidate < best {
                    lower = candidate;
                } else {
                    upper = candidate;
                }
                if candidate_value <= previous_value || previous_best == best {
                    third_best = previous_best;
                    third_value = previous_value;
                    previous_best = candidate;
                    previous_value = candidate_value;
                } else if candidate_value <= third_value
                    || third_best == best
                    || third_best == previous_best
                {
                    third_best = candidate;
                    third_value = candidate_value;
                }
            }
        }

        Err(Error::ExtremumSearchDidNotConverge {
            iterations: max_iterations,
        })
    }
}

#[derive(Clone, Copy)]
pub(super) enum ExtremumSense {
    Minimum,
    Maximum,
}

impl ExtremumSense {
    fn objective(self, value: f64) -> f64 {
        match self {
            Self::Minimum => value,
            Self::Maximum => -value,
        }
    }
}

pub(super) struct LocatedExtremum<S: TimeScale, T> {
    data: T,
    evidence: ExtremumEvidence<S>,
}

impl<S: TimeScale, T> LocatedExtremum<S, T> {
    pub(super) fn into_parts(self) -> (T, ExtremumEvidence<S>) {
        (self.data, self.evidence)
    }
}

pub(super) struct SampledExtremumSearch<S: TimeScale> {
    interval_start: Instant<S>,
    interval_end: Instant<S>,
    options: ExtremumSearchOptions,
    evaluations: u32,
}

impl<S: TimeScale> SampledExtremumSearch<S> {
    pub(super) const fn new(
        interval_start: Instant<S>,
        interval_end: Instant<S>,
        options: ExtremumSearchOptions,
    ) -> Self {
        Self {
            interval_start,
            interval_end,
            options,
            evaluations: 0,
        }
    }

    pub(super) fn search<T, F>(
        mut self,
        sense: ExtremumSense,
        mut evaluate: F,
    ) -> Result<Vec<LocatedExtremum<S, T>>, Error>
    where
        F: FnMut(Instant<S>, &mut u32) -> Result<(f64, T), Error>,
    {
        let mut left_epoch = self.interval_start;
        let (left_value, _) = evaluate(left_epoch, &mut self.evaluations)?;
        let first_remaining = self.interval_end.duration_since(left_epoch)?;
        let first_step = first_remaining.min(self.options.scan_step());
        let mut middle_epoch = left_epoch.checked_add(first_step)?;
        if middle_epoch == self.interval_end {
            return Ok(Vec::new());
        }
        let (middle_value, _) = evaluate(middle_epoch, &mut self.evaluations)?;
        let mut left_objective = sense.objective(left_value);
        let mut middle_objective = sense.objective(middle_value);
        let mut extrema = Vec::new();

        while middle_epoch < self.interval_end {
            let remaining = self.interval_end.duration_since(middle_epoch)?;
            let step = remaining.min(self.options.scan_step());
            let right_epoch = middle_epoch.checked_add(step)?;
            let (right_value, _) = evaluate(right_epoch, &mut self.evaluations)?;
            let right_objective = sense.objective(right_value);
            if middle_objective <= left_objective
                && middle_objective <= right_objective
                && (middle_objective < left_objective || middle_objective < right_objective)
            {
                let refined = BracketedExtremumSearch::refine_minimum(
                    left_epoch,
                    middle_epoch,
                    right_epoch,
                    self.options.time_tolerance(),
                    self.options.max_refinement_iterations(),
                    |epoch| {
                        evaluate(epoch, &mut self.evaluations)
                            .map(|(value, _)| sense.objective(value))
                    },
                )?;
                let (_, data) = evaluate(refined.instant(), &mut self.evaluations)?;
                let located = LocatedExtremum {
                    data,
                    evidence: ExtremumEvidence::new(
                        refined.bracket_start(),
                        refined.bracket_end(),
                        refined.time_uncertainty(),
                        refined.iterations(),
                        self.evaluations,
                    ),
                };
                Self::push_unique(&mut extrema, located, self.options.time_tolerance())?;
            }
            left_epoch = middle_epoch;
            left_objective = middle_objective;
            middle_epoch = right_epoch;
            middle_objective = right_objective;
        }
        Ok(extrema)
    }

    fn push_unique<T>(
        extrema: &mut Vec<LocatedExtremum<S, T>>,
        candidate: LocatedExtremum<S, T>,
        tolerance: Duration,
    ) -> Result<(), Error> {
        if let Some(previous) = extrema.last()
            && candidate
                .evidence
                .bracket_start()
                .duration_since(previous.evidence.bracket_start())?
                .checked_abs()?
                <= tolerance
        {
            return Ok(());
        }
        extrema.push(candidate);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Tai;

    #[test]
    fn bounded_brent_refines_an_interior_parabolic_minimum() {
        let start = Instant::<Tai>::from_tai_nanoseconds_since_1900(0);
        let initial = start
            .checked_add(Duration::from_seconds_f64(4.0).unwrap())
            .unwrap();
        let end = start
            .checked_add(Duration::from_seconds_f64(10.0).unwrap())
            .unwrap();
        let refined = BracketedExtremumSearch::refine_minimum(
            start,
            initial,
            end,
            Duration::from_microseconds(1).unwrap(),
            64,
            |epoch| {
                let seconds = epoch.duration_since(start)?.as_seconds_f64();
                Ok((seconds - 3.25).powi(2))
            },
        )
        .unwrap();
        let seconds = refined
            .instant()
            .duration_since(start)
            .unwrap()
            .as_seconds_f64();
        assert!((seconds - 3.25).abs() < 2.0e-6, "{seconds}");
        assert!(
            refined.time_uncertainty() <= Duration::from_microseconds(2).unwrap(),
            "{:?}",
            refined.time_uncertainty()
        );
    }

    #[test]
    fn sampled_extremum_search_retains_data_and_evaluation_evidence() {
        let start = Instant::<Tai>::from_tai_nanoseconds_since_1900(0);
        let end = start
            .checked_add(Duration::from_seconds_f64(30.0).unwrap())
            .unwrap();
        let options = ExtremumSearchOptions::new(
            Duration::from_seconds_f64(5.0).unwrap(),
            Duration::from_microseconds(1).unwrap(),
            64,
            100,
            ReceptionLightTimeOptions::standard(),
        )
        .unwrap();
        let extrema = SampledExtremumSearch::new(start, end, options)
            .search(ExtremumSense::Maximum, |epoch, evaluations| {
                *evaluations += 1;
                let seconds = epoch.duration_since(start)?.as_seconds_f64();
                Ok((10.0 - (seconds - 12.5).powi(2), seconds))
            })
            .unwrap();
        assert_eq!(extrema.len(), 1);
        let (sampled_seconds, evidence) = extrema.into_iter().next().unwrap().into_parts();
        assert!((sampled_seconds - 12.5).abs() < 2.0e-6);
        assert!(evidence.evaluations() > 3);
        assert!(evidence.bracket_start() <= evidence.bracket_end());
    }
}
