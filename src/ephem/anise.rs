use core::fmt;
use std::path::{Path, PathBuf};

use anise::{
    almanac::Almanac,
    ephemerides::EphemerisError as AniseEphemerisError,
    frames::Frame,
    naif::daf::{DAFError, NAIFSummaryRecord},
};

use crate::{
    frame::Bcrs,
    math::{Length, Speed, Vector3},
    time::{Hifitime, Tdb, TimeScale},
};

use super::{
    CelestialBody, Coverage, EphemerisProvenance, EphemerisProvider, EphemerisQuery, Error,
    RelativeState,
};

const J2000_ORIENTATION_ID: i32 = 1;
const MAX_CENTER_CHAIN_DEPTH: usize = 8;

/// One explicitly ordered kernel file recorded in a [`KernelManifest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kernel {
    path: PathBuf,
    byte_len: u64,
}

impl Kernel {
    /// Returns the local kernel path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the file length recorded when the manifest was constructed.
    pub const fn byte_len(&self) -> u64 {
        self.byte_len
    }
}

/// A non-empty, frozen kernel load order.
///
/// Later entries have higher precedence when loaded SPK segments overlap. Construction and
/// loading perform local filesystem access only; no kernel is downloaded implicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelManifest {
    kernels: Vec<Kernel>,
}

impl KernelManifest {
    /// Inspects local kernel paths and freezes their iteration order.
    pub fn inspect<I, P>(paths: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let mut kernels = Vec::new();
        for (index, path) in paths.into_iter().enumerate() {
            let path = path.into();
            let metadata = std::fs::metadata(&path).map_err(|source| Error::KernelIo {
                index,
                path: path.clone(),
                kind: source.kind(),
            })?;
            kernels.push(Kernel {
                path,
                byte_len: metadata.len(),
            });
        }
        if kernels.is_empty() {
            return Err(Error::EmptyKernelManifest);
        }
        Ok(Self { kernels })
    }

    /// Returns the ordered kernel records.
    pub fn kernels(&self) -> &[Kernel] {
        &self.kernels
    }

    /// Returns the number of ordered kernels.
    pub fn kernel_count(&self) -> usize {
        self.kernels.len()
    }
}

/// A loaded, offline ephemeris backed by an explicitly supplied ANISE kernel manifest.
pub struct Ephemeris {
    manifest: KernelManifest,
    almanac: Almanac,
}

impl Ephemeris {
    /// Loads every local kernel in manifest order without network access.
    pub fn load(manifest: KernelManifest) -> Result<Self, Error> {
        let mut almanac = Almanac::default();
        for (index, kernel) in manifest.kernels.iter().enumerate() {
            let path = kernel
                .path
                .to_str()
                .ok_or_else(|| Error::InvalidKernelPath {
                    index,
                    path: kernel.path.clone(),
                })?;
            almanac = almanac.load(path).map_err(|source| Error::CorruptKernel {
                index,
                path: kernel.path.clone(),
                reason: source.to_string(),
            })?;
            let actual_bytes = std::fs::metadata(&kernel.path)
                .map_err(|source| Error::KernelIo {
                    index,
                    path: kernel.path.clone(),
                    kind: source.kind(),
                })?
                .len();
            if actual_bytes != kernel.byte_len {
                return Err(Error::KernelChanged {
                    index,
                    path: kernel.path.clone(),
                    expected_bytes: kernel.byte_len,
                    actual_bytes,
                });
            }
        }
        Ok(Self { manifest, almanac })
    }

    /// Returns the frozen manifest in effective low-to-high precedence order.
    pub const fn manifest(&self) -> &KernelManifest {
        &self.manifest
    }

    /// Evaluates a geometric BCRS-axis state with no light-time or stellar-aberration correction.
    pub fn state<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<RelativeState<Bcrs, S>, Error> {
        if query.target() == query.center() {
            return RelativeState::zero(query.target(), query.epoch());
        }

        let backend_epoch = Hifitime::new().export(query.epoch().retag::<Tdb>());
        self.validate_query_segments(backend_epoch, query)?;
        let target_frame = Self::j2000_frame(query.target());
        let center_frame = Self::j2000_frame(query.center());
        let state = self
            .almanac
            .translate_geometric(target_frame, center_frame, backend_epoch)
            .map_err(|source| Self::map_query_error(source, query))?;

        RelativeState::try_new(
            query.target(),
            query.center(),
            Vector3::new(
                Length::from_kilometres(state.radius_km[0])?,
                Length::from_kilometres(state.radius_km[1])?,
                Length::from_kilometres(state.radius_km[2])?,
            ),
            Vector3::new(
                Speed::from_kilometres_per_second(state.velocity_km_s[0])?,
                Speed::from_kilometres_per_second(state.velocity_km_s[1])?,
                Speed::from_kilometres_per_second(state.velocity_km_s[2])?,
            ),
            query.epoch(),
        )
    }

    /// Returns the inclusive continuous coverage of the segment chain selected at the query epoch.
    ///
    /// A later overlapping segment can take precedence inside this interval, but the requested
    /// target-centre state remains evaluable throughout it.
    pub fn coverage<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<Coverage<Bcrs, S>, Error> {
        if query.target() == query.center() {
            return Ok(Coverage::from_ordered(
                query.target(),
                query.center(),
                query.epoch(),
                query.epoch(),
            ));
        }

        let backend_epoch = Hifitime::new().export(query.epoch().retag::<Tdb>());
        let root = self
            .almanac
            .try_find_ephemeris_root()
            .map_err(|source| Self::map_query_error(source, query))?;
        let mut start = None;
        let mut end = None;
        self.intersect_path_coverage(
            query.target(),
            root,
            backend_epoch,
            query,
            &mut start,
            &mut end,
        )?;
        self.intersect_path_coverage(
            query.center(),
            root,
            backend_epoch,
            query,
            &mut start,
            &mut end,
        )?;

        let start = start.ok_or_else(|| Self::coverage_error(query))?;
        let end = end.ok_or_else(|| Self::coverage_error(query))?;
        Ok(Coverage::from_ordered(
            query.target(),
            query.center(),
            Hifitime::new().import::<S>(start),
            Hifitime::new().import::<S>(end),
        ))
    }

    fn validate_query_segments<S: TimeScale>(
        &self,
        epoch: anise::time::Epoch,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<(), Error> {
        let root = self
            .almanac
            .try_find_ephemeris_root()
            .map_err(|source| Self::map_query_error(source, query))?;
        self.validate_path_segments(query.target(), root, epoch, query)?;
        self.validate_path_segments(query.center(), root, epoch, query)
    }

    fn validate_path_segments<S: TimeScale>(
        &self,
        body: CelestialBody,
        root: i32,
        epoch: anise::time::Epoch,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<(), Error> {
        let mut current = body.naif_id();
        if current == root {
            return Ok(());
        }
        for _ in 0..MAX_CENTER_CHAIN_DEPTH {
            let summary = self
                .almanac
                .spk_summary_at_epoch(current, epoch)
                .map_err(|source| Self::map_query_error(source, query))?
                .0;
            Self::ensure_supported_segment(summary.frame_id, summary.data_type_i, query)?;
            current = summary.center_id;
            if current == root {
                return Ok(());
            }
        }
        Err(Error::CenterCycle {
            target: query.target(),
            center: query.center(),
        })
    }

    fn ensure_supported_segment<S: TimeScale>(
        frame_id: i32,
        data_type: i32,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<(), Error> {
        if frame_id != J2000_ORIENTATION_ID {
            return Err(Error::UnsupportedFrame {
                target: query.target(),
                center: query.center(),
                frame_id,
            });
        }
        if matches!(data_type, 1 | 2 | 3 | 9 | 13) {
            Ok(())
        } else {
            Err(Error::UnsupportedSegment {
                target: query.target(),
                center: query.center(),
                reason: format!("SPK type {data_type}"),
            })
        }
    }

    fn intersect_path_coverage<S: TimeScale>(
        &self,
        body: CelestialBody,
        root: i32,
        epoch: anise::time::Epoch,
        query: EphemerisQuery<Bcrs, S>,
        start: &mut Option<anise::time::Epoch>,
        end: &mut Option<anise::time::Epoch>,
    ) -> Result<(), Error> {
        let mut current = body.naif_id();
        if current == root {
            return Ok(());
        }

        for _ in 0..MAX_CENTER_CHAIN_DEPTH {
            let summary = self
                .almanac
                .spk_summary_at_epoch(current, epoch)
                .map_err(|source| Self::map_query_error(source, query))?
                .0;
            Self::ensure_supported_segment(summary.frame_id, summary.data_type_i, query)?;
            let summary_start = summary.start_epoch();
            let summary_end = summary.end_epoch();
            *start = Some(start.map_or(summary_start, |value| value.max(summary_start)));
            *end = Some(end.map_or(summary_end, |value| value.min(summary_end)));
            current = summary.center_id;
            if current == root {
                return Ok(());
            }
        }

        Err(Error::CenterCycle {
            target: query.target(),
            center: query.center(),
        })
    }

    fn j2000_frame(body: CelestialBody) -> Frame {
        Frame::new(body.naif_id(), J2000_ORIENTATION_ID)
    }

    fn coverage_error<F, S: TimeScale>(query: EphemerisQuery<F, S>) -> Error {
        Error::Coverage {
            target: query.target(),
            center: query.center(),
            epoch_tai_nanoseconds: query.epoch().tai_nanoseconds_since_1900(),
        }
    }

    fn map_query_error<S: TimeScale>(
        source: AniseEphemerisError,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Error {
        match source {
            AniseEphemerisError::SPK {
                source: DAFError::SummaryIdError { id, .. },
                ..
            } if id == query.target().naif_id() => Error::UnknownTarget {
                target: query.target(),
            },
            AniseEphemerisError::SPK {
                source: DAFError::SummaryIdError { id, .. },
                ..
            } if id == query.center().naif_id() => Error::UnknownCenter {
                center: query.center(),
            },
            AniseEphemerisError::SPK {
                source:
                    DAFError::SummaryIdAtEpochError { .. }
                    | DAFError::SummaryNameAtEpochError { .. }
                    | DAFError::InterpolationDataErrorFromId { .. }
                    | DAFError::InterpolationDataErrorFromName { .. },
                ..
            } => Self::coverage_error(query),
            AniseEphemerisError::SPK {
                source: DAFError::UnsupportedDatatype { .. },
                ..
            } => Error::UnsupportedSegment {
                target: query.target(),
                center: query.center(),
                reason: source.to_string(),
            },
            AniseEphemerisError::SPK {
                source: DAFError::MaxRecursionDepth,
                ..
            } => Error::CenterCycle {
                target: query.target(),
                center: query.center(),
            },
            _ => Error::Backend {
                operation: "evaluating a geometric state",
                reason: source.to_string(),
            },
        }
    }
}

impl fmt::Debug for Ephemeris {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Ephemeris")
            .field("manifest", &self.manifest)
            .finish_non_exhaustive()
    }
}

impl EphemerisProvider for Ephemeris {
    fn state<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<RelativeState<Bcrs, S>, Error> {
        Ephemeris::state(self, query)
    }

    fn coverage<S: TimeScale>(
        &self,
        query: EphemerisQuery<Bcrs, S>,
    ) -> Result<Coverage<Bcrs, S>, Error> {
        Ephemeris::coverage(self, query)
    }

    fn provenance(&self) -> Result<EphemerisProvenance, Error> {
        Ok(EphemerisProvenance::anise(
            "ANISE SPK",
            self.manifest.clone(),
        ))
    }
}
