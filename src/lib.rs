//! Strongly typed astronomical algorithms.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod constants;

#[cfg(feature = "anise")]
pub mod astro;
pub mod catalog;
#[cfg(feature = "std")]
pub mod earth;
pub mod ephem;
#[cfg(feature = "anise")]
pub mod event;
pub mod frame;
pub mod math;
pub mod time;
pub mod uncertainty;
