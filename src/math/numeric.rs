use super::Error;

/// Explicit convergence controls for bracketed root finding.
#[cfg_attr(feature = "std", derive(garde::Validate))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootOptions {
    #[cfg_attr(
        feature = "std",
        garde(range(min = f64::MIN_POSITIVE, max = f64::MAX))
    )]
    x_tolerance: f64,
    #[cfg_attr(
        feature = "std",
        garde(range(min = f64::MIN_POSITIVE, max = f64::MAX))
    )]
    residual_tolerance: f64,
    #[cfg_attr(feature = "std", garde(range(min = 1)))]
    max_iterations: u32,
}

impl RootOptions {
    /// Constructs normal positive tolerances and a non-zero iteration budget.
    pub fn new(
        x_tolerance: f64,
        residual_tolerance: f64,
        max_iterations: u32,
    ) -> Result<Self, Error> {
        let candidate = Self {
            x_tolerance,
            residual_tolerance,
            max_iterations,
        };
        #[cfg(feature = "std")]
        {
            use garde::Validate as _;
            if candidate.validate().is_ok() {
                return Ok(candidate);
            }
        }
        Error::ensure_positive_tolerance("root x", x_tolerance)?;
        Error::ensure_positive_tolerance("root residual", residual_tolerance)?;
        if max_iterations == 0 {
            return Err(Error::OutOfRange {
                field: "root maximum iterations",
                value: 0.0,
                interval: "[1, 2³² - 1]",
                unit: "iterations",
            });
        }
        Ok(candidate)
    }

    /// Returns the absolute root-coordinate tolerance.
    pub const fn x_tolerance(self) -> f64 {
        self.x_tolerance
    }

    /// Returns the absolute function-residual tolerance.
    pub const fn residual_tolerance(self) -> f64 {
        self.residual_tolerance
    }

    /// Returns the maximum number of bisection iterations.
    pub const fn max_iterations(self) -> u32 {
        self.max_iterations
    }

    /// Finds a root of a fallible-free scalar function inside a sign-changing bracket.
    pub fn bisect<F>(
        self,
        mut lower: f64,
        mut upper: f64,
        mut function: F,
    ) -> Result<RootResult, Error>
    where
        F: FnMut(f64) -> f64,
    {
        Error::ensure_finite("root lower bound", lower)?;
        Error::ensure_finite("root upper bound", upper)?;
        if lower >= upper {
            return Err(Error::InvalidInterval { lower, upper });
        }

        let mut f_lower = function(lower);
        let f_upper = function(upper);
        Error::ensure_finite("function value at lower bound", f_lower)?;
        Error::ensure_finite("function value at upper bound", f_upper)?;
        if f_lower == 0.0 {
            return Ok(RootResult::new(lower, 0.0, 0, lower, upper));
        }
        if f_upper == 0.0 {
            return Ok(RootResult::new(upper, 0.0, 0, lower, upper));
        }
        if f_lower.is_sign_positive() == f_upper.is_sign_positive() {
            return Err(Error::NotBracketed {
                lower,
                upper,
                f_lower,
                f_upper,
            });
        }

        let mut root = lower + (upper - lower) * 0.5;
        let mut residual = function(root);
        Error::ensure_finite("function value at midpoint", residual)?;
        for iteration in 1..=self.max_iterations {
            if residual.abs() <= self.residual_tolerance
                || (upper - lower) * 0.5 <= self.x_tolerance
            {
                return Ok(RootResult::new(root, residual, iteration, lower, upper));
            }

            if residual.is_sign_positive() == f_lower.is_sign_positive() {
                lower = root;
                f_lower = residual;
            } else {
                upper = root;
            }
            root = lower + (upper - lower) * 0.5;
            residual = function(root);
            Error::ensure_finite("function value at midpoint", residual)?;
        }

        Err(Error::NonConvergent {
            iterations: self.max_iterations,
            residual: residual.abs(),
            lower,
            upper,
        })
    }
}

/// A converged bracketed root and its numerical evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootResult {
    root: f64,
    residual: f64,
    iterations: u32,
    lower: f64,
    upper: f64,
}

impl RootResult {
    fn new(root: f64, residual: f64, iterations: u32, lower: f64, upper: f64) -> Self {
        Self {
            root,
            residual,
            iterations,
            lower,
            upper,
        }
    }

    /// Returns the estimated root.
    pub const fn root(self) -> f64 {
        self.root
    }

    /// Returns the signed function residual at the estimated root.
    pub const fn residual(self) -> f64 {
        self.residual
    }

    /// Returns the number of completed iterations.
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns the final lower bracket bound.
    pub const fn lower(self) -> f64 {
        self.lower
    }

    /// Returns the final upper bracket bound.
    pub const fn upper(self) -> f64 {
        self.upper
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use garde::Validate as _;

    use super::RootOptions;

    #[test]
    fn garde_rejects_invalid_raw_options() {
        let invalid = RootOptions {
            x_tolerance: 0.0,
            residual_tolerance: f64::INFINITY,
            max_iterations: 0,
        };

        assert!(invalid.validate().is_err());
    }
}
