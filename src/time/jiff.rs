use super::{Date, DateTime, Error, Gregorian, TimeOfDay, UnixTimestamp, Utc};

/// Adapter for Jiff civil values and Unix timestamps.
#[derive(Debug, Clone, Copy, Default)]
pub struct Jiff;

impl Jiff {
    /// Constructs the stateless adapter.
    pub const fn new() -> Self {
        Self
    }

    /// Imports Jiff's proleptic Gregorian date.
    pub fn import_date(self, value: jiff::civil::Date) -> Result<Date<Gregorian>, Error> {
        Date::new(
            i32::from(value.year()),
            value.month() as u8,
            value.day() as u8,
        )
    }

    /// Exports a Gregorian date when its year fits Jiff's supported range.
    pub fn export_date(self, value: Date<Gregorian>) -> Result<jiff::civil::Date, Error> {
        let year = i16::try_from(value.year()).map_err(|_| {
            Error::component(
                "Jiff year",
                i128::from(value.year()),
                i128::from(i16::MIN),
                i128::from(i16::MAX),
            )
        })?;
        jiff::civil::Date::new(year, value.month() as i8, value.day() as i8).map_err(|source| {
            Error::Jiff {
                operation: "exporting Gregorian date",
                reason: source,
            }
        })
    }

    /// Imports a scale-free Jiff civil datetime under an explicit UTC-label contract.
    ///
    /// This does not resolve the label to a physical instant. Pass the result
    /// to [`TimeContext::resolve`](super::TimeContext::resolve).
    pub fn import_utc_label(
        self,
        value: jiff::civil::DateTime,
    ) -> Result<DateTime<Gregorian, Utc>, Error> {
        DateTime::new(
            Date::new(
                i32::from(value.year()),
                value.month() as u8,
                value.day() as u8,
            )?,
            TimeOfDay::new(
                value.hour() as u8,
                value.minute() as u8,
                value.second() as u8,
                value.subsec_nanosecond() as u32,
            )?,
        )
    }

    /// Exports a UTC civil label when it is representable by Jiff.
    pub fn export_utc_label(
        self,
        value: DateTime<Gregorian, Utc>,
    ) -> Result<jiff::civil::DateTime, Error> {
        if value.time().is_leap_second() {
            return Err(Error::LeapSecondNotRepresentable { target: "jiff" });
        }
        let year = i16::try_from(value.date().year()).map_err(|_| {
            Error::component(
                "Jiff year",
                i128::from(value.date().year()),
                i128::from(i16::MIN),
                i128::from(i16::MAX),
            )
        })?;
        jiff::civil::DateTime::new(
            year,
            value.date().month() as i8,
            value.date().day() as i8,
            value.time().hour() as i8,
            value.time().minute() as i8,
            value.time().second() as i8,
            value.time().nanosecond() as i32,
        )
        .map_err(|source| Error::Jiff {
            operation: "exporting UTC civil label",
            reason: source,
        })
    }

    /// Imports Jiff's exact POSIX timestamp without assigning UTC leap semantics.
    pub fn import_timestamp(self, value: jiff::Timestamp) -> UnixTimestamp {
        UnixTimestamp::from_nanoseconds(value.as_nanosecond())
    }

    /// Exports an exact POSIX timestamp within Jiff's supported range.
    pub fn export_timestamp(self, value: UnixTimestamp) -> Result<jiff::Timestamp, Error> {
        jiff::Timestamp::from_nanosecond(value.as_nanoseconds()).map_err(|source| Error::Jiff {
            operation: "exporting Unix timestamp",
            reason: source,
        })
    }
}
