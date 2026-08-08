use hyastro::time::{
    BesselianEpoch, Date, DateTime, Duration, Gps, Gregorian, Instant, Julian, JulianDate,
    JulianEpoch, Tai, TimeContext, Tt, UnixTimestamp, Utc,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gregorian = Date::<Gregorian>::new(1582, 10, 15)?;
    let julian: Date<Julian> = gregorian.convert()?;
    assert_eq!((julian.year(), julian.month(), julian.day()), (1582, 10, 5));
    assert_eq!(julian.convert::<Gregorian>()?, gregorian);

    let context = TimeContext::builtin();
    let before_label =
        DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 23, 59, 59, 500_000_000)?;
    let leap_label =
        DateTime::<Gregorian, Utc>::from_components(2016, 12, 31, 23, 59, 60, 500_000_000)?;
    let after_label =
        DateTime::<Gregorian, Utc>::from_components(2017, 1, 1, 0, 0, 0, 500_000_000)?;
    let before = context.resolve(before_label)?;
    let leap = context.resolve(leap_label)?;
    let after = context.resolve(after_label)?;

    assert_eq!(leap.duration_since(before)?, Duration::from_seconds(1)?);
    assert_eq!(after.duration_since(leap)?, Duration::from_seconds(1)?);
    assert_eq!(context.represent::<Gregorian, Utc>(leap)?, leap_label);

    let tai = Instant::<Tai>::from_instant(after, &context)?;
    let tt = Instant::<Tt>::from_instant(after, &context)?;
    let gps = Instant::<Gps>::from_instant(after, &context)?;
    assert_eq!(
        tai.tai_nanoseconds_since_1900(),
        tt.tai_nanoseconds_since_1900()
    );
    assert_eq!(
        tai.tai_nanoseconds_since_1900(),
        gps.tai_nanoseconds_since_1900()
    );

    let tai_label = context.represent::<Gregorian, Tai>(tai)?;
    let tt_label = context.represent::<Gregorian, Tt>(tt)?;
    let gps_label = context.represent::<Gregorian, Gps>(gps)?;
    assert_eq!(tai_label.time().second(), 37);
    assert_eq!(tt_label.time().minute(), 1);
    assert_eq!(tt_label.time().second(), 9);
    assert_eq!(gps_label.time().second(), 18);

    let tt_julian = JulianDate::<Tt>::from_instant(tt, &context)?;
    let tt_modified = tt_julian.to_modified()?;
    let julian_epoch = JulianEpoch::from_tt(tt_julian)?;
    let besselian_epoch = BesselianEpoch::from_tt(tt_julian)?;
    assert_eq!(tt_modified.to_julian()?, tt_julian);

    let exact_tick = JulianDate::<Tt>::from_j2000_offset_days(0.0)?
        .checked_add_duration(Duration::from_nanoseconds(1))?;
    assert_eq!(
        exact_tick.duration_since_rounded(JulianEpoch::J2000.to_tt()?)?,
        Duration::from_nanoseconds(1)
    );

    let unix = UnixTimestamp::EPOCH.checked_add(Duration::from_seconds(1_700_000_000)?)?;
    assert_eq!(
        unix.duration_since(UnixTimestamp::EPOCH)?,
        Duration::from_seconds(1_700_000_000)?
    );

    println!(
        "Gregorian 1582-10-15 = Julian {:04}-{:02}-{:02}",
        julian.year(),
        julian.month(),
        julian.day()
    );
    println!("leap label            = {:?}", leap_label);
    println!("TAI label             = {:?}", tai_label);
    println!("TT label              = {:?}", tt_label);
    println!("GPS label             = {:?}", gps_label);
    println!("TT Julian Date        = {:.12}", tt_julian.as_f64_lossy());
    println!("TT Modified Julian    = {:.12}", tt_modified.as_f64_lossy());
    println!("Julian epoch          = J{:.9}", julian_epoch.value());
    println!("Besselian epoch       = B{:.9}", besselian_epoch.value());
    println!("typed epoch           = {:?}", tt.as_epoch());
    println!("Unix nanoseconds      = {}", unix.as_nanoseconds());
    println!(
        "leap data version     = {}",
        context.leap_seconds().version()
    );

    Ok(())
}
