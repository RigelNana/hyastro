use thiserror::Error;

use super::CelestialBody;

/// Errors produced by ephemeris values, kernel loading, and state queries.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A mathematical value or operation was invalid.
    #[error(transparent)]
    Math(#[from] crate::math::Error),

    /// A time-scale model could not represent the requested epoch.
    #[error(transparent)]
    Time(#[from] crate::time::Error),

    /// A state for one body relative to itself contained a non-zero value.
    #[error("state for {body} relative to itself must have zero position and velocity")]
    NonZeroIdentityState {
        /// Body used as both target and centre.
        body: CelestialBody,
    },

    /// Two relative states did not form a connected target-centre chain.
    #[error("cannot chain state centred on {left_center} with state targeting {right_target}")]
    DisconnectedChain {
        /// Centre of the left state.
        left_center: CelestialBody,
        /// Target of the right state.
        right_target: CelestialBody,
    },

    /// Relative states from different physical epochs were combined.
    #[error(
        "ephemeris state epoch {left_tai_nanoseconds} TAI ns does not match {right_tai_nanoseconds} TAI ns"
    )]
    EpochMismatch {
        /// Left epoch as TAI nanoseconds since 1900-01-01 TAI.
        left_tai_nanoseconds: i128,
        /// Right epoch in the same representation.
        right_tai_nanoseconds: i128,
    },

    /// A spherical body-figure model had no identifier.
    #[error("spherical body-figure identifier must not be empty")]
    EmptyBodyFigureIdentifier,

    /// A system barycentre was assigned a physical surface.
    #[error("{body} is a system barycentre and has no physical surface")]
    BodyHasNoPhysicalSurface {
        /// Identity that cannot own a body figure.
        body: CelestialBody,
    },

    /// A spherical body-figure radius was zero or negative.
    #[error("spherical figure radius for {body} must be positive, got {metres} m")]
    InvalidSphericalBodyRadius {
        /// Body whose figure was rejected.
        body: CelestialBody,
        /// Rejected radius in metres.
        metres: f64,
    },

    /// An ephemeris provider supplied an empty stable model identifier.
    #[error("ephemeris model identifier must not be empty")]
    EmptyModelIdentifier,

    /// A provider does not model one body required by a query.
    #[error("{provider} does not provide a state for {body}")]
    UnsupportedBody {
        /// Body unavailable from the selected provider.
        body: CelestialBody,
        /// Stable provider or model identifier.
        provider: &'static str,
    },

    /// An analytical ephemeris failed while evaluating one supported body.
    #[error("{provider} failed while evaluating {body} (status {status})")]
    AnalyticalModelFailure {
        /// Body whose analytical state could not be evaluated.
        body: CelestialBody,
        /// Stable provider or model identifier.
        provider: &'static str,
        /// Stable status reported by the analytical implementation.
        status: i32,
    },

    /// No kernels were supplied to an ephemeris.
    #[cfg(feature = "anise")]
    #[error("an ephemeris kernel manifest must not be empty")]
    EmptyKernelManifest,

    /// A kernel path could not be represented for the backend.
    #[cfg(feature = "anise")]
    #[error("kernel #{index} path is not valid UTF-8: {path:?}")]
    InvalidKernelPath {
        /// Zero-based manifest index.
        index: usize,
        /// Rejected path.
        path: std::path::PathBuf,
    },

    /// Kernel metadata or contents could not be read.
    #[cfg(feature = "anise")]
    #[error("could not read kernel #{index} at {path:?}: {kind:?}")]
    KernelIo {
        /// Zero-based manifest index.
        index: usize,
        /// Kernel path.
        path: std::path::PathBuf,
        /// Stable I/O error category.
        kind: std::io::ErrorKind,
    },

    /// A kernel changed length between manifest construction and loading.
    #[cfg(feature = "anise")]
    #[error(
        "kernel #{index} at {path:?} changed length from {expected_bytes} to {actual_bytes} bytes"
    )]
    KernelChanged {
        /// Zero-based manifest index.
        index: usize,
        /// Kernel path.
        path: std::path::PathBuf,
        /// Length recorded by the manifest.
        expected_bytes: u64,
        /// Length observed immediately after loading.
        actual_bytes: u64,
    },

    /// A kernel was malformed or unsupported by the selected backend.
    #[cfg(feature = "anise")]
    #[error("could not load kernel #{index} at {path:?}: {reason}")]
    CorruptKernel {
        /// Zero-based manifest index.
        index: usize,
        /// Kernel path.
        path: std::path::PathBuf,
        /// Backend-independent diagnostic text.
        reason: String,
    },

    /// The target was absent from the loaded ephemeris tree.
    #[cfg(feature = "anise")]
    #[error("ephemeris target {target} is unavailable")]
    UnknownTarget {
        /// Requested target.
        target: CelestialBody,
    },

    /// The centre was absent from the loaded ephemeris tree.
    #[cfg(feature = "anise")]
    #[error("ephemeris centre {center} is unavailable")]
    UnknownCenter {
        /// Requested centre.
        center: CelestialBody,
    },

    /// The requested physical epoch was outside provider coverage.
    #[error(
        "no ephemeris coverage for {target} relative to {center} at {epoch_tai_nanoseconds} TAI ns"
    )]
    Coverage {
        /// Requested target.
        target: CelestialBody,
        /// Requested centre.
        center: CelestialBody,
        /// Requested physical epoch as TAI nanoseconds since 1900-01-01 TAI.
        epoch_tai_nanoseconds: i128,
    },

    /// A provider reported a coverage interval whose end precedes its start.
    #[error(
        "invalid ephemeris coverage interval: {start_tai_nanoseconds}..={end_tai_nanoseconds} TAI ns"
    )]
    InvalidCoverageInterval {
        /// Inclusive start as TAI nanoseconds since 1900-01-01 TAI.
        start_tai_nanoseconds: i128,
        /// Inclusive end as TAI nanoseconds since 1900-01-01 TAI.
        end_tai_nanoseconds: i128,
    },

    /// A selected SPK segment used axes that cannot be represented as BCRS.
    #[cfg(feature = "anise")]
    #[error(
        "unsupported SPK reference frame {frame_id} while querying {target} relative to {center}"
    )]
    UnsupportedFrame {
        /// Requested target.
        target: CelestialBody,
        /// Requested centre.
        center: CelestialBody,
        /// NAIF reference-frame identifier found in the selected segment.
        frame_id: i32,
    },

    /// A selected SPK segment type is not supported.
    #[cfg(feature = "anise")]
    #[error("unsupported ephemeris segment while querying {target} relative to {center}: {reason}")]
    UnsupportedSegment {
        /// Requested target.
        target: CelestialBody,
        /// Requested centre.
        center: CelestialBody,
        /// Backend-independent diagnostic text.
        reason: String,
    },

    /// The loaded target-centre graph contained a cycle or exceeded its supported depth.
    #[cfg(feature = "anise")]
    #[error(
        "ephemeris centre chain for {target} relative to {center} contains a cycle or is too deep"
    )]
    CenterCycle {
        /// Requested target.
        target: CelestialBody,
        /// Requested centre.
        center: CelestialBody,
    },

    /// The backend failed in a way without a more specific stable classification.
    #[cfg(feature = "anise")]
    #[error("ephemeris backend failed while {operation}: {reason}")]
    Backend {
        /// Operation being performed.
        operation: &'static str,
        /// Backend-independent diagnostic text.
        reason: String,
    },
}
