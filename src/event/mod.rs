//! Bracketed astronomical event searches and fixed-offset solar-term calendars.

mod error;
mod search;
mod solar_term;

pub use error::Error;
pub use search::{EventEvidence, Events, SolarTermSearchOptions};
pub use solar_term::{SolarTerm, SolarTermEvent, SolarTermYear, SolarTermYearEntry};
