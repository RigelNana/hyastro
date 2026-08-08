use core::fmt;

use crate::{
    astro::{Astrometry, ReceptionLightTimeOptions},
    math::Angle,
    time::{Duration, Instant, TimeScale},
};

use super::Error;

/// Explicit scanning, refinement, and evaluation controls for solar-term searches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarTermSearchOptions {
    scan_step: Duration,
    time_tolerance: Duration,
    longitude_tolerance: Angle,
    max_refinement_iterations: u32,
    max_evaluations: u32,
    light_time: ReceptionLightTimeOptions,
}

impl SolarTermSearchOptions {
    /// Largest supported coarse step for monotonic apparent-solar longitude scanning.
    pub const MAX_SCAN_STEP: Duration =
        Duration::from_nanoseconds(7 * Duration::NANOSECONDS_PER_DAY);

    /// Constructs validated controls for scanning and refining solar terms.
    pub fn new(
        scan_step: Duration,
        time_tolerance: Duration,
        longitude_tolerance: Angle,
        max_refinement_iterations: u32,
        max_evaluations: u32,
        light_time: ReceptionLightTimeOptions,
    ) -> Result<Self, Error> {
        if scan_step <= Duration::ZERO || scan_step > Self::MAX_SCAN_STEP {
            return Err(Error::InvalidSearchDuration {
                field: "solar-term scan step",
                nanoseconds: scan_step.as_nanoseconds(),
                maximum_nanoseconds: Self::MAX_SCAN_STEP.as_nanoseconds(),
            });
        }
        if time_tolerance <= Duration::ZERO || time_tolerance > scan_step {
            return Err(Error::InvalidSearchDuration {
                field: "solar-term time tolerance",
                nanoseconds: time_tolerance.as_nanoseconds(),
                maximum_nanoseconds: scan_step.as_nanoseconds(),
            });
        }
        let maximum_longitude_tolerance = 7.5_f64.to_radians();
        if longitude_tolerance.as_radians() <= 0.0
            || longitude_tolerance.as_radians() > maximum_longitude_tolerance
        {
            return Err(Error::InvalidLongitudeTolerance {
                radians: longitude_tolerance.as_radians(),
                maximum_radians: maximum_longitude_tolerance,
            });
        }
        if max_refinement_iterations == 0 {
            return Err(Error::InvalidSearchLimit {
                field: "solar-term refinement iterations",
                value: max_refinement_iterations,
            });
        }
        if max_evaluations == 0 {
            return Err(Error::InvalidSearchLimit {
                field: "solar-term evaluations",
                value: max_evaluations,
            });
        }
        Ok(Self {
            scan_step,
            time_tolerance,
            longitude_tolerance,
            max_refinement_iterations,
            max_evaluations,
            light_time,
        })
    }

    /// Returns one-day scanning, one-millisecond timing, and picoradian angular tolerances.
    pub const fn standard() -> Self {
        Self {
            scan_step: Duration::from_nanoseconds(Duration::NANOSECONDS_PER_DAY),
            time_tolerance: Duration::from_nanoseconds(1_000_000),
            longitude_tolerance: Angle::from_finite(1.0e-12),
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

    /// Returns the maximum accepted apparent-longitude residual.
    pub const fn longitude_tolerance(self) -> Angle {
        self.longitude_tolerance
    }

    /// Returns the maximum Brent refinement iterations per event.
    pub const fn max_refinement_iterations(self) -> u32 {
        self.max_refinement_iterations
    }

    /// Returns the maximum astrometric evaluations for one search.
    pub const fn max_evaluations(self) -> u32 {
        self.max_evaluations
    }

    /// Returns the reception light-time controls used for each solar position.
    pub const fn light_time(self) -> ReceptionLightTimeOptions {
        self.light_time
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

    /// Returns the final signed apparent-longitude residual.
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
pub struct Events<'context, 'data, E> {
    pub(super) astrometry: Astrometry<'context, 'data, E>,
}

impl<'context, 'data, E> Events<'context, 'data, E> {
    /// Constructs event algorithms from an existing astrometry context.
    pub const fn new(astrometry: Astrometry<'context, 'data, E>) -> Self {
        Self { astrometry }
    }

    /// Returns the astrometry context used for event evaluations.
    pub const fn astrometry(self) -> Astrometry<'context, 'data, E> {
        self.astrometry
    }
}
