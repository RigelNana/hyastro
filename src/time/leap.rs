use super::{
    Calendar, Date, DateTime, Duration, Error, Gregorian, Instant, JulianDate, Tai, TimeOfDay, Utc,
};

/// Whether a UTC leap second inserts or removes one labeled second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeapKind {
    /// Inserts `23:59:60` at the end of a UTC day.
    Positive,
    /// Removes `23:59:59` from the end of a UTC day.
    Negative,
}

impl LeapKind {
    /// Returns the change in cumulative `TAI−UTC`, in whole seconds.
    pub const fn value(self) -> i8 {
        match self {
            Self::Positive => 1,
            Self::Negative => -1,
        }
    }
}

/// One UTC leap-second event.
///
/// `effective` is the UTC date whose midnight begins with the new `TAI−UTC`
/// offset. A positive leap at the end of 2016-12-31 is therefore effective on
/// 2017-01-01.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LeapSecond {
    effective: Date<Gregorian>,
    kind: LeapKind,
}

impl LeapSecond {
    /// Constructs a leap-second event from its offset-effective UTC date.
    pub const fn new(effective: Date<Gregorian>, kind: LeapKind) -> Self {
        Self { effective, kind }
    }

    /// Returns the UTC date whose midnight begins with the new offset.
    pub const fn effective(self) -> Date<Gregorian> {
        self.effective
    }

    /// Returns whether this event inserts or removes a UTC second.
    pub const fn kind(self) -> LeapKind {
        self.kind
    }
}

/// Immutable, versioned leap-second data with explicit validity dates.
///
/// The table borrows its event slice, so custom data requires no allocator and
/// remains usable in `no_std` builds. `expires` is an exclusive upper bound:
/// labels and instants at or beyond that UTC date are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeapSeconds<'a> {
    version: &'a str,
    valid_from: Date<Gregorian>,
    expires: Date<Gregorian>,
    initial_offset: i16,
    entries: &'a [LeapSecond],
}

impl<'a> LeapSeconds<'a> {
    /// Validates and constructs leap-second data.
    ///
    /// `initial_offset` is `TAI−UTC` at midnight on `valid_from`. Entries must
    /// be strictly ordered, fall after `valid_from` and before `expires`, and
    /// each changes the offset by exactly the value of its [`LeapKind`].
    pub fn new(
        version: &'a str,
        valid_from: Date<Gregorian>,
        expires: Date<Gregorian>,
        initial_offset: i16,
        entries: &'a [LeapSecond],
    ) -> Result<Self, Error> {
        if version.is_empty() {
            return Err(Error::InvalidLeapSeconds {
                reason: "version must not be empty",
            });
        }

        let valid_from_day = valid_from.to_julian_day_number().value();
        let expires_day = expires.to_julian_day_number().value();
        if valid_from_day >= expires_day {
            return Err(Error::InvalidLeapSeconds {
                reason: "valid-from date must precede expiration date",
            });
        }

        let mut previous_day = valid_from_day;
        let mut offset = initial_offset;
        for (index, entry) in entries.iter().enumerate() {
            let effective_day = entry.effective.to_julian_day_number().value();
            if effective_day <= previous_day {
                return Err(Error::InvalidLeapSecond {
                    index,
                    reason: "effective dates must be strictly increasing after valid-from",
                });
            }
            if effective_day >= expires_day {
                return Err(Error::InvalidLeapSecond {
                    index,
                    reason: "effective date must precede expiration",
                });
            }
            offset = offset.checked_add(i16::from(entry.kind.value())).ok_or(
                Error::InvalidLeapSecond {
                    index,
                    reason: "cumulative TAI−UTC offset overflowed",
                },
            )?;
            previous_day = effective_day;
        }

        Ok(Self {
            version,
            valid_from,
            expires,
            initial_offset,
            entries,
        })
    }

    /// Returns the bundled IERS Bulletin C 72 data.
    ///
    /// It covers UTC from 1972-01-01 and expires on 2027-06-28. The data is
    /// sourced from the public-domain IANA `leap-seconds.list` updated through
    /// IERS Bulletin C 72.
    pub const fn builtin() -> LeapSeconds<'static> {
        LeapSeconds {
            version: "IERS Bulletin C 72",
            valid_from: Date::from_valid_components(1972, 1, 1),
            expires: Date::from_valid_components(2027, 6, 28),
            initial_offset: 10,
            entries: &BUILTIN_LEAP_SECONDS,
        }
    }

    /// Returns the data release identifier.
    pub const fn version(self) -> &'a str {
        self.version
    }

    /// Returns the covered half-open UTC date interval `[start, expires)`.
    pub const fn coverage(self) -> (Date<Gregorian>, Date<Gregorian>) {
        (self.valid_from, self.expires)
    }

    /// Returns the exclusive UTC expiration date.
    pub const fn expires(self) -> Date<Gregorian> {
        self.expires
    }

    /// Returns `TAI−UTC` at a physical instant.
    pub fn offset(self, instant: Instant<Tai>) -> Result<Duration, Error> {
        Duration::from_seconds(i64::from(self.offset_at(instant)?))
    }

    /// Returns whether a leap second occurs at the end of a UTC date.
    pub fn is_leap(self, date: Date<Gregorian>) -> Result<bool, Error> {
        Ok(self.kind_on(date)?.is_some())
    }

    /// Resolves a covered UTC label to an exact physical instant.
    pub fn resolve<C: Calendar>(self, value: DateTime<C, Utc>) -> Result<Instant<Utc>, Error> {
        let date: Date<Gregorian> = value.date().convert()?;
        let time = value.time();
        self.ensure_date(date)?;
        let leap = self.kind_on(date)?;

        if time.is_leap_second() {
            if leap != Some(LeapKind::Positive) {
                return Err(Error::InvalidLeapSecondDate {
                    year: date.year(),
                    month: date.month(),
                    day: date.day(),
                });
            }
            let effective = date.checked_add_days(1)?;
            let offset = self.offset_on(effective)?;
            let boundary = self.tai_midnight(effective, offset)?;
            let leap_start = boundary
                .checked_sub(Duration::NANOSECONDS_PER_SECOND)
                .ok_or(Error::Overflow {
                    operation: "resolving UTC leap second",
                })?;
            let tai_nanoseconds = leap_start
                .checked_add(i128::from(time.nanosecond()))
                .ok_or(Error::Overflow {
                    operation: "resolving UTC leap-second fraction",
                })?;
            return Ok(Instant::from_tai_nanoseconds(tai_nanoseconds));
        }

        if leap == Some(LeapKind::Negative)
            && time.hour() == 23
            && time.minute() == 59
            && time.second() == 59
        {
            return Err(Error::NonexistentUtcLabel {
                year: date.year(),
                month: date.month(),
                day: date.day(),
                hour: time.hour(),
                minute: time.minute(),
                second: time.second(),
            });
        }

        let nominal = self.nominal_nanoseconds(date, time)?;
        let offset = i128::from(self.offset_on(date)?)
            .checked_mul(Duration::NANOSECONDS_PER_SECOND)
            .ok_or(Error::Overflow {
                operation: "applying TAI−UTC offset",
            })?;
        let tai_nanoseconds = nominal.checked_add(offset).ok_or(Error::Overflow {
            operation: "resolving UTC label",
        })?;
        Ok(Instant::from_tai_nanoseconds(tai_nanoseconds))
    }

    /// Represents a covered physical instant as a UTC calendar label.
    pub fn represent<C: Calendar>(self, instant: Instant<Utc>) -> Result<DateTime<C, Utc>, Error> {
        let tai =
            Instant::<Tai>::from_tai_nanoseconds_since_1900(instant.tai_nanoseconds_since_1900());
        let offset = self.offset_at(tai)?;

        if let Some((date, nanosecond)) = self.positive_leap_label(tai)? {
            let date = date.convert()?;
            let time = TimeOfDay::utc_leap_second_label(nanosecond)?;
            return DateTime::new(date, time);
        }

        let offset_nanoseconds = i128::from(offset)
            .checked_mul(Duration::NANOSECONDS_PER_SECOND)
            .ok_or(Error::Overflow {
                operation: "removing TAI−UTC offset",
            })?;
        let nominal = tai
            .tai_nanoseconds_since_1900()
            .checked_sub(offset_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "representing UTC instant",
            })?;
        self.label_from_nominal(nominal)
    }

    pub(crate) fn julian_date(self, instant: Instant<Utc>) -> Result<JulianDate<Utc>, Error> {
        let label = self.represent::<Gregorian>(instant)?;
        let day_adjustment = self.kind_on(label.date())?.map_or(0, LeapKind::value);
        let day_nanoseconds = Duration::NANOSECONDS_PER_DAY
            + i128::from(day_adjustment) * Duration::NANOSECONDS_PER_SECOND;
        let first = label.date().to_julian_day_number().value() as f64 - 0.5;
        let second = label.time().nanoseconds_since_midnight() as f64 / day_nanoseconds as f64;
        JulianDate::from_parts(first, second)
    }

    fn ensure_date(self, date: Date<Gregorian>) -> Result<(), Error> {
        let day = date.to_julian_day_number().value();
        if day < self.valid_from.to_julian_day_number().value() {
            return Err(Error::LeapSecondsUnavailable {
                year: self.valid_from.year(),
                month: self.valid_from.month(),
                day: self.valid_from.day(),
            });
        }
        if day >= self.expires.to_julian_day_number().value() {
            return Err(Error::LeapSecondsExpired {
                year: self.expires.year(),
                month: self.expires.month(),
                day: self.expires.day(),
            });
        }
        Ok(())
    }

    fn kind_on(self, date: Date<Gregorian>) -> Result<Option<LeapKind>, Error> {
        self.ensure_date(date)?;
        let effective = date.checked_add_days(1)?.to_julian_day_number().value();
        for entry in self.entries {
            let entry_day = entry.effective.to_julian_day_number().value();
            if entry_day == effective {
                return Ok(Some(entry.kind));
            }
            if entry_day > effective {
                break;
            }
        }
        Ok(None)
    }

    fn offset_on(self, date: Date<Gregorian>) -> Result<i16, Error> {
        self.ensure_date(date)?;
        let day = date.to_julian_day_number().value();
        let mut offset = self.initial_offset;
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.effective.to_julian_day_number().value() > day {
                break;
            }
            offset = offset.checked_add(i16::from(entry.kind.value())).ok_or(
                Error::InvalidLeapSecond {
                    index,
                    reason: "cumulative TAI−UTC offset overflowed",
                },
            )?;
        }
        Ok(offset)
    }

    fn offset_at(self, instant: Instant<Tai>) -> Result<i16, Error> {
        let tai_nanoseconds = instant.tai_nanoseconds_since_1900();
        let start = self.tai_midnight(self.valid_from, self.initial_offset)?;
        if tai_nanoseconds < start {
            return Err(Error::LeapSecondsUnavailable {
                year: self.valid_from.year(),
                month: self.valid_from.month(),
                day: self.valid_from.day(),
            });
        }

        let final_offset = self.final_offset()?;
        let end = self.tai_midnight(self.expires, final_offset)?;
        if tai_nanoseconds >= end {
            return Err(Error::LeapSecondsExpired {
                year: self.expires.year(),
                month: self.expires.month(),
                day: self.expires.day(),
            });
        }

        let mut offset = self.initial_offset;
        for (index, entry) in self.entries.iter().enumerate() {
            let next = offset.checked_add(i16::from(entry.kind.value())).ok_or(
                Error::InvalidLeapSecond {
                    index,
                    reason: "cumulative TAI−UTC offset overflowed",
                },
            )?;
            let boundary = self.tai_midnight(entry.effective, next)?;
            if tai_nanoseconds < boundary {
                break;
            }
            offset = next;
        }
        Ok(offset)
    }

    fn positive_leap_label(
        self,
        instant: Instant<Tai>,
    ) -> Result<Option<(Date<Gregorian>, u32)>, Error> {
        let tai_nanoseconds = instant.tai_nanoseconds_since_1900();
        let mut offset = self.initial_offset;
        for (index, entry) in self.entries.iter().enumerate() {
            let next = offset.checked_add(i16::from(entry.kind.value())).ok_or(
                Error::InvalidLeapSecond {
                    index,
                    reason: "cumulative TAI−UTC offset overflowed",
                },
            )?;
            let boundary = self.tai_midnight(entry.effective, next)?;
            if entry.kind == LeapKind::Positive {
                let start = boundary
                    .checked_sub(Duration::NANOSECONDS_PER_SECOND)
                    .ok_or(Error::Overflow {
                        operation: "locating UTC leap second",
                    })?;
                if tai_nanoseconds >= start && tai_nanoseconds < boundary {
                    let elapsed = tai_nanoseconds - start;
                    let nanosecond = u32::try_from(elapsed).map_err(|_| Error::Overflow {
                        operation: "representing UTC leap-second fraction",
                    })?;
                    return Ok(Some((entry.effective.checked_add_days(-1)?, nanosecond)));
                }
            }
            if tai_nanoseconds < boundary {
                break;
            }
            offset = next;
        }
        Ok(None)
    }

    fn final_offset(self) -> Result<i16, Error> {
        let mut offset = self.initial_offset;
        for (index, entry) in self.entries.iter().enumerate() {
            offset = offset.checked_add(i16::from(entry.kind.value())).ok_or(
                Error::InvalidLeapSecond {
                    index,
                    reason: "cumulative TAI−UTC offset overflowed",
                },
            )?;
        }
        Ok(offset)
    }

    fn tai_midnight(self, date: Date<Gregorian>, offset: i16) -> Result<i128, Error> {
        let nominal = self.nominal_nanoseconds(date, TimeOfDay::MIDNIGHT)?;
        let offset_nanoseconds = i128::from(offset)
            .checked_mul(Duration::NANOSECONDS_PER_SECOND)
            .ok_or(Error::Overflow {
                operation: "converting TAI−UTC offset to nanoseconds",
            })?;
        nominal
            .checked_add(offset_nanoseconds)
            .ok_or(Error::Overflow {
                operation: "constructing UTC midnight on the TAI timeline",
            })
    }

    fn nominal_nanoseconds(self, date: Date<Gregorian>, time: TimeOfDay) -> Result<i128, Error> {
        let epoch = Date::<Gregorian>::from_valid_components(1900, 1, 1);
        let days = date.days_since(epoch)?;
        let day_nanoseconds = i128::from(days)
            .checked_mul(Duration::NANOSECONDS_PER_DAY)
            .ok_or(Error::Overflow {
                operation: "converting UTC date to nominal nanoseconds",
            })?;
        day_nanoseconds
            .checked_add(i128::from(time.nanoseconds_since_midnight()))
            .ok_or(Error::Overflow {
                operation: "converting UTC label to nominal nanoseconds",
            })
    }

    fn label_from_nominal<C: Calendar>(
        self,
        nominal_nanoseconds: i128,
    ) -> Result<DateTime<C, Utc>, Error> {
        let days = nominal_nanoseconds.div_euclid(Duration::NANOSECONDS_PER_DAY);
        let days = i64::try_from(days).map_err(|_| Error::Overflow {
            operation: "converting nominal UTC nanoseconds to a date",
        })?;
        let within_day = nominal_nanoseconds.rem_euclid(Duration::NANOSECONDS_PER_DAY);
        let within_day = u64::try_from(within_day).map_err(|_| Error::Overflow {
            operation: "converting nominal UTC nanoseconds to time of day",
        })?;
        let epoch = Date::<Gregorian>::from_valid_components(1900, 1, 1);
        let date = epoch.checked_add_days(days)?.convert()?;
        let time = TimeOfDay::from_nanoseconds_since_midnight(within_day)?;
        DateTime::new(date, time)
    }
}

const BUILTIN_LEAP_SECONDS: [LeapSecond; 27] = [
    LeapSecond::new(Date::from_valid_components(1972, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1973, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1974, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1975, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1976, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1977, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1978, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1979, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1980, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1981, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1982, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1983, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1985, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1988, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1990, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1991, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1992, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1993, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1994, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1996, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1997, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(1999, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(2006, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(2009, 1, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(2012, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(2015, 7, 1), LeapKind::Positive),
    LeapSecond::new(Date::from_valid_components(2017, 1, 1), LeapKind::Positive),
];
